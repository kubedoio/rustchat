//! Retention job for scheduled data cleanup
//!
//! This module provides a background task that periodically cleans up
//! old messages and files based on the server's retention configuration.

use std::collections::HashSet;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::config::RetentionJobConfig;
use crate::error::AppError;
use crate::storage::{ListObjectsResult, ObjectStorage};

/// Retention job configuration (days-based, from server_config).
#[derive(Debug, Clone)]
pub struct RetentionConfig {
    pub message_retention_days: i64,
    pub file_retention_days: i64,
}

/// Statistics from a retention cleanup run.
#[derive(Debug, Default)]
pub struct RetentionStats {
    pub messages_deleted: u64,
    pub files_deleted: u64,
    pub file_keys: Vec<String>,
    pub file_delete_errors: u64,
}

/// Statistics from an orphan S3 object scan.
#[derive(Debug, Default)]
pub struct OrphanScanStats {
    pub pages_scanned: u64,
    pub objects_scanned: u64,
    pub orphans_deleted: u64,
    pub orphan_delete_errors: u64,
}

/// Backend storage operations required by [`run_retention_cleanup`].
#[async_trait]
pub trait RetentionStore: Send + Sync {
    /// Delete messages older than `cutoff`. Returns number of rows deleted.
    async fn delete_old_messages(&self, cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error>;

    /// Fetch keys of file rows older than `cutoff`.
    async fn fetch_expired_file_keys(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<String>, sqlx::Error>;

    /// Delete file rows whose keys are in `keys`. Returns number of rows deleted.
    async fn delete_files_by_keys(&self, keys: &[&str]) -> Result<u64, sqlx::Error>;
}

/// PostgreSQL implementation of [`RetentionStore`].
pub struct PgRetentionStore<'a>(pub &'a PgPool);

#[async_trait]
impl RetentionStore for PgRetentionStore<'_> {
    async fn delete_old_messages(&self, cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM posts WHERE created_at < $1 AND NOT is_pinned")
            .bind(cutoff)
            .execute(self.0)
            .await?;

        Ok(result.rows_affected())
    }

    async fn fetch_expired_file_keys(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<String>, sqlx::Error> {
        let files: Vec<(String,)> = sqlx::query_as("SELECT key FROM files WHERE created_at < $1")
            .bind(cutoff)
            .fetch_all(self.0)
            .await?;

        Ok(files.into_iter().map(|f| f.0).collect())
    }

    async fn delete_files_by_keys(&self, keys: &[&str]) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM files WHERE key = ANY($1::text[])")
            .bind(keys)
            .execute(self.0)
            .await?;

        Ok(result.rows_affected())
    }
}

/// Backend storage operations required by [`run_orphan_scan`].
#[async_trait]
pub trait OrphanStore: Send + Sync {
    /// Return the subset of `keys` that exist in the `files` table.
    async fn filter_existing_keys(&self, keys: &[String]) -> Result<HashSet<String>, sqlx::Error>;
}

/// PostgreSQL implementation of [`OrphanStore`].
pub struct PgOrphanStore<'a>(pub &'a PgPool);

#[async_trait]
impl OrphanStore for PgOrphanStore<'_> {
    async fn filter_existing_keys(&self, keys: &[String]) -> Result<HashSet<String>, sqlx::Error> {
        let existing_keys: Vec<(String,)> =
            sqlx::query_as("SELECT key FROM files WHERE key = ANY($1::text[])")
                .bind(keys)
                .fetch_all(self.0)
                .await?;

        Ok(existing_keys.into_iter().map(|r| r.0).collect())
    }
}

/// Run the retention cleanup job.
///
/// File rows are deleted only after their corresponding S3 object has been
/// removed (or is already absent). If S3 deletion fails for a particular key,
/// the error is logged and processing continues; that row is left in place for
/// the next run.
pub async fn run_retention_cleanup<S: ObjectStorage>(
    db: &PgPool,
    storage: &S,
    config: RetentionConfig,
) -> Result<RetentionStats, sqlx::Error> {
    run_retention_cleanup_with_store(&PgRetentionStore(db), storage, config).await
}

/// Testable version of [`run_retention_cleanup`] that accepts any
/// [`RetentionStore`] implementation.
pub async fn run_retention_cleanup_with_store<S: ObjectStorage, R: RetentionStore>(
    store: &R,
    storage: &S,
    config: RetentionConfig,
) -> Result<RetentionStats, sqlx::Error> {
    let mut stats = RetentionStats::default();

    // Clean up old messages
    if config.message_retention_days > 0 {
        let cutoff = Utc::now() - Duration::days(config.message_retention_days);

        stats.messages_deleted = store.delete_old_messages(cutoff).await?;
        info!(
            "Retention: Deleted {} messages older than {} days",
            stats.messages_deleted, config.message_retention_days
        );
    }

    // Clean up old files
    if config.file_retention_days > 0 {
        let cutoff = Utc::now() - Duration::days(config.file_retention_days);

        let keys = store.fetch_expired_file_keys(cutoff).await?;
        let mut deleted_keys = Vec::with_capacity(keys.len());

        for key in &keys {
            match storage.delete_object(key).await {
                Ok(()) => {
                    deleted_keys.push(key.as_str());
                }
                Err(e) => {
                    stats.file_delete_errors += 1;
                    warn!(
                        error = %e,
                        key = %key,
                        "Retention: failed to delete S3 object; leaving files row for retry"
                    );
                }
            }
        }

        if !deleted_keys.is_empty() {
            stats.files_deleted = store.delete_files_by_keys(&deleted_keys).await?;
        }

        stats.file_keys = keys;
        info!(
            "Retention: Deleted {} files older than {} days ({} S3 errors)",
            stats.files_deleted, config.file_retention_days, stats.file_delete_errors
        );
    }

    Ok(stats)
}

/// Run an orphan S3 object scan.
///
/// Lists objects in the configured bucket and deletes any object whose key is
/// not referenced by a row in the `files` table. The scan is paginated and
/// throttled to avoid overwhelming S3 or the database.
pub async fn run_orphan_scan<S: ObjectStorage>(
    db: &PgPool,
    storage: &S,
    config: &RetentionJobConfig,
) -> Result<OrphanScanStats, AppError> {
    run_orphan_scan_with_store(&PgOrphanStore(db), storage, config).await
}

/// Testable version of [`run_orphan_scan`] that accepts any [`OrphanStore`]
/// implementation.
pub async fn run_orphan_scan_with_store<S: ObjectStorage, O: OrphanStore>(
    store: &O,
    storage: &S,
    config: &RetentionJobConfig,
) -> Result<OrphanScanStats, AppError> {
    let mut stats = OrphanScanStats::default();
    let mut continuation_token: Option<String> = None;
    let max_keys = config
        .orphan_scan_page_size
        .clamp(1, 1000)
        .try_into()
        .unwrap_or(1000);
    let page_delay = StdDuration::from_millis(config.orphan_scan_page_delay_ms.max(1));

    loop {
        let page = storage
            .list_objects(Some("files/"), continuation_token.as_deref(), max_keys)
            .await?;

        process_orphan_page(store, storage, &page, &mut stats).await?;

        continuation_token = page.next_continuation_token.clone();
        stats.pages_scanned += 1;
        stats.objects_scanned += page.keys.len() as u64;

        if continuation_token.is_none() {
            break;
        }

        tokio::time::sleep(page_delay).await;
    }

    info!(
        "Orphan scan complete: {} objects scanned, {} orphans deleted, {} errors",
        stats.objects_scanned, stats.orphans_deleted, stats.orphan_delete_errors
    );

    Ok(stats)
}

async fn process_orphan_page<S: ObjectStorage, O: OrphanStore>(
    store: &O,
    storage: &S,
    page: &ListObjectsResult,
    stats: &mut OrphanScanStats,
) -> Result<(), sqlx::Error> {
    if page.keys.is_empty() {
        return Ok(());
    }

    let existing = store.filter_existing_keys(&page.keys).await?;
    let orphans: Vec<&String> = page
        .keys
        .iter()
        .filter(|key| !existing.contains(*key))
        .collect();

    info!(
        "Orphan scan: page of {} objects, {} orphans found",
        page.keys.len(),
        orphans.len()
    );

    for key in orphans {
        match storage.delete_object(key).await {
            Ok(()) => {
                stats.orphans_deleted += 1;
                info!(key = %key, "Orphan scan: deleted orphan object");
            }
            Err(e) => {
                stats.orphan_delete_errors += 1;
                warn!(error = %e, key = %key, "Orphan scan: failed to delete orphan object");
            }
        }
    }

    Ok(())
}

/// Spawn the retention job as a background task.
pub fn spawn_retention_job(
    db: PgPool,
    storage: crate::storage::S3Client,
    job_config: RetentionJobConfig,
    shutdown: CancellationToken,
) {
    tokio::spawn(async move {
        let mut restart_delay_secs = 1u64;

        loop {
            let db_for_run = db.clone();
            let storage_for_run = storage.clone();
            let config_for_run = job_config.clone();
            let shutdown_for_run = shutdown.clone();
            let run_handle = tokio::spawn(async move {
                run_retention_loop(
                    db_for_run,
                    storage_for_run,
                    config_for_run,
                    shutdown_for_run,
                )
                .await;
            });

            match run_handle.await {
                Ok(()) => {
                    warn!("Retention worker exited unexpectedly; restarting");
                }
                Err(join_error) => {
                    error!(
                        error = %join_error,
                        "Retention worker panicked; restarting"
                    );
                }
            }

            tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                _ = tokio::time::sleep(StdDuration::from_secs(restart_delay_secs)) => {}
            }
            restart_delay_secs = (restart_delay_secs * 2).min(60);
        }
    });

    info!("Retention worker supervisor started");
}

async fn run_retention_loop(
    db: PgPool,
    storage: crate::storage::S3Client,
    job_config: RetentionJobConfig,
    shutdown: CancellationToken,
) {
    // Run every hour
    let mut interval = tokio::time::interval(StdDuration::from_secs(3600));
    let mut last_orphan_scan: Option<tokio::time::Instant> = None;
    let orphan_scan_interval = StdDuration::from_secs(job_config.orphan_scan_interval_hours * 3600);

    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = interval.tick() => {}
        }

        // Fetch current retention config from DB
        let config_result: Result<Option<(i32, i32)>, sqlx::Error> = sqlx::query_as(
            "SELECT 
                (compliance->'message_retention_days')::int,
                (compliance->'file_retention_days')::int
             FROM server_config WHERE id = 'default'",
        )
        .fetch_optional(&db)
        .await;

        match config_result {
            Ok(Some((message_days, file_days))) => {
                if message_days > 0 || file_days > 0 {
                    let config = RetentionConfig {
                        message_retention_days: message_days as i64,
                        file_retention_days: file_days as i64,
                    };

                    match run_retention_cleanup(&db, &storage, config).await {
                        Ok(stats) => {
                            if stats.messages_deleted > 0
                                || stats.files_deleted > 0
                                || stats.file_delete_errors > 0
                            {
                                info!(
                                    "Retention cleanup complete: {} messages, {} files deleted, {} S3 errors",
                                    stats.messages_deleted, stats.files_deleted, stats.file_delete_errors
                                );
                            }
                        }
                        Err(e) => {
                            error!("Retention cleanup failed: {}", e);
                        }
                    }
                }
            }
            Ok(None) => {
                // No config found, skip
            }
            Err(e) => {
                warn!("Failed to fetch retention config: {}", e);
            }
        }

        if job_config.orphan_scan_enabled
            && last_orphan_scan
                .map(|last| last.elapsed() >= orphan_scan_interval)
                .unwrap_or(true)
        {
            match run_orphan_scan(&db, &storage, &job_config).await {
                Ok(stats) => {
                    if stats.orphans_deleted > 0 || stats.orphan_delete_errors > 0 {
                        info!(
                            "Orphan scan complete: {} scanned, {} deleted, {} errors",
                            stats.objects_scanned,
                            stats.orphans_deleted,
                            stats.orphan_delete_errors
                        );
                    }
                    last_orphan_scan = Some(tokio::time::Instant::now());
                }
                Err(e) => {
                    error!("Orphan scan failed: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use tokio::sync::Mutex;

    #[derive(Default, Clone)]
    struct InMemoryRetentionStore {
        file_keys: Arc<Mutex<Vec<String>>>,
        deleted_files: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl RetentionStore for InMemoryRetentionStore {
        async fn delete_old_messages(&self, _cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error> {
            Ok(0)
        }

        async fn fetch_expired_file_keys(
            &self,
            _cutoff: DateTime<Utc>,
        ) -> Result<Vec<String>, sqlx::Error> {
            Ok(self.file_keys.lock().await.clone())
        }

        async fn delete_files_by_keys(&self, keys: &[&str]) -> Result<u64, sqlx::Error> {
            let mut deleted = self.deleted_files.lock().await;
            deleted.extend(keys.iter().map(|&k| k.to_string()));
            Ok(keys.len() as u64)
        }
    }

    #[derive(Default, Clone)]
    struct InMemoryOrphanStore {
        existing: Arc<HashSet<String>>,
    }

    impl InMemoryOrphanStore {
        fn with_existing(keys: &[&str]) -> Self {
            Self {
                existing: Arc::new(keys.iter().map(|s| s.to_string()).collect()),
            }
        }
    }

    #[async_trait]
    impl OrphanStore for InMemoryOrphanStore {
        async fn filter_existing_keys(
            &self,
            keys: &[String],
        ) -> Result<HashSet<String>, sqlx::Error> {
            Ok(keys
                .iter()
                .filter(|k| self.existing.contains(*k))
                .cloned()
                .collect())
        }
    }

    #[derive(Clone, Default)]
    struct MockStorage {
        deleted: Arc<Mutex<Vec<String>>>,
        listed: Arc<Mutex<Vec<Vec<String>>>>,
        fail_keys: Arc<HashSet<String>>,
        not_found_keys: Arc<HashSet<String>>,
    }

    impl MockStorage {
        fn with_listing(pages: Vec<Vec<String>>) -> Self {
            Self {
                listed: Arc::new(Mutex::new(pages)),
                ..Default::default()
            }
        }

        fn with_failed_keys(fail_keys: &[&str]) -> Self {
            Self {
                fail_keys: Arc::new(fail_keys.iter().map(|s| s.to_string()).collect()),
                ..Default::default()
            }
        }

        fn with_not_found_keys(not_found_keys: &[&str]) -> Self {
            Self {
                not_found_keys: Arc::new(not_found_keys.iter().map(|s| s.to_string()).collect()),
                ..Default::default()
            }
        }
    }

    #[async_trait]
    impl ObjectStorage for MockStorage {
        async fn delete_object(&self, key: &str) -> Result<(), AppError> {
            if self.fail_keys.contains(key) {
                return Err(AppError::ExternalService(format!(
                    "mock delete failed for {}",
                    key
                )));
            }
            if self.not_found_keys.contains(key) {
                return Ok(());
            }
            self.deleted.lock().await.push(key.to_string());
            Ok(())
        }

        async fn list_objects(
            &self,
            prefix: Option<&str>,
            _continuation_token: Option<&str>,
            _max_keys: i32,
        ) -> Result<ListObjectsResult, AppError> {
            let mut listed = self.listed.lock().await;
            let page = listed.remove(0);
            let keys: Vec<String> = page
                .into_iter()
                .filter(|key| prefix.is_none_or(|p| key.starts_with(p)))
                .collect();
            let next_token = if listed.is_empty() {
                None
            } else {
                Some("next".to_string())
            };
            Ok(ListObjectsResult {
                keys,
                next_continuation_token: next_token,
            })
        }
    }

    fn test_retention_config() -> RetentionConfig {
        RetentionConfig {
            message_retention_days: 0,
            file_retention_days: 30,
        }
    }

    fn test_orphan_config() -> RetentionJobConfig {
        RetentionJobConfig {
            orphan_scan_enabled: true,
            orphan_scan_interval_hours: 24,
            orphan_scan_page_size: 10,
            orphan_scan_page_delay_ms: 1,
        }
    }

    #[test]
    fn spawn_retention_job_accepts_expected_arguments() {
        // Compile-time check that the public spawn API matches the expected signature.
        let _: fn(PgPool, crate::storage::S3Client, RetentionJobConfig, CancellationToken) =
            spawn_retention_job;
    }

    #[tokio::test]
    async fn retention_cleanup_deletes_db_rows_when_s3_succeeds() {
        let store = InMemoryRetentionStore {
            file_keys: Arc::new(Mutex::new(vec!["a".to_string(), "b".to_string()])),
            ..Default::default()
        };
        let storage = MockStorage::default();

        let stats = run_retention_cleanup_with_store(&store, &storage, test_retention_config())
            .await
            .unwrap();

        assert_eq!(stats.files_deleted, 2);
        assert_eq!(stats.file_delete_errors, 0);
        assert_eq!(store.deleted_files.lock().await.as_slice(), &["a", "b"]);
        assert_eq!(storage.deleted.lock().await.as_slice(), &["a", "b"]);
    }

    #[tokio::test]
    async fn retention_cleanup_preserves_db_rows_when_s3_fails() {
        let store = InMemoryRetentionStore {
            file_keys: Arc::new(Mutex::new(vec!["a".to_string(), "b".to_string()])),
            ..Default::default()
        };
        let storage = MockStorage::with_failed_keys(&["b"]);

        let stats = run_retention_cleanup_with_store(&store, &storage, test_retention_config())
            .await
            .unwrap();

        assert_eq!(stats.files_deleted, 1);
        assert_eq!(stats.file_delete_errors, 1);
        assert_eq!(store.deleted_files.lock().await.as_slice(), &["a"]);
        assert_eq!(storage.deleted.lock().await.as_slice(), &["a"]);
    }

    #[tokio::test]
    async fn retention_cleanup_deletes_db_rows_when_s3_not_found() {
        let store = InMemoryRetentionStore {
            file_keys: Arc::new(Mutex::new(vec!["a".to_string(), "b".to_string()])),
            ..Default::default()
        };
        let storage = MockStorage::with_not_found_keys(&["b"]);

        let stats = run_retention_cleanup_with_store(&store, &storage, test_retention_config())
            .await
            .unwrap();

        assert_eq!(stats.files_deleted, 2);
        assert_eq!(stats.file_delete_errors, 0);
        assert_eq!(store.deleted_files.lock().await.as_slice(), &["a", "b"]);
        assert_eq!(storage.deleted.lock().await.as_slice(), &["a"]);
    }

    #[tokio::test]
    async fn orphan_scan_deletes_only_unreferenced_keys() {
        let store = InMemoryOrphanStore::with_existing(&["files/a", "files/c"]);
        let storage = MockStorage::with_listing(vec![vec![
            "files/a".to_string(),
            "files/b".to_string(),
            "files/c".to_string(),
        ]]);

        let stats = run_orphan_scan_with_store(&store, &storage, &test_orphan_config())
            .await
            .unwrap();

        assert_eq!(stats.objects_scanned, 3);
        assert_eq!(stats.pages_scanned, 1);
        assert_eq!(stats.orphans_deleted, 1);
        assert_eq!(stats.orphan_delete_errors, 0);
        assert_eq!(storage.deleted.lock().await.as_slice(), &["files/b"]);
    }

    #[tokio::test]
    async fn orphan_scan_continues_on_delete_failure() {
        let store = InMemoryOrphanStore::default();
        let storage = MockStorage {
            listed: Arc::new(Mutex::new(vec![vec![
                "files/a".to_string(),
                "files/b".to_string(),
            ]])),
            fail_keys: Arc::new(["files/b".to_string()].into_iter().collect()),
            ..Default::default()
        };

        let stats = run_orphan_scan_with_store(&store, &storage, &test_orphan_config())
            .await
            .unwrap();

        assert_eq!(stats.objects_scanned, 2);
        assert_eq!(stats.orphans_deleted, 1);
        assert_eq!(stats.orphan_delete_errors, 1);
        assert_eq!(storage.deleted.lock().await.as_slice(), &["files/a"]);
    }

    #[tokio::test]
    async fn orphan_scan_deletes_orphans_across_multiple_pages() {
        let store = InMemoryOrphanStore::with_existing(&["files/a", "files/c"]);
        let storage = MockStorage::with_listing(vec![
            vec!["files/a".to_string(), "files/b".to_string()],
            vec!["files/c".to_string(), "files/d".to_string()],
        ]);

        let mut config = test_orphan_config();
        config.orphan_scan_page_delay_ms = 0;

        let stats = run_orphan_scan_with_store(&store, &storage, &config)
            .await
            .unwrap();

        assert_eq!(stats.objects_scanned, 4);
        assert_eq!(stats.pages_scanned, 2);
        assert_eq!(stats.orphans_deleted, 2);
        assert_eq!(stats.orphan_delete_errors, 0);

        let mut deleted = storage.deleted.lock().await.clone();
        deleted.sort();
        assert_eq!(deleted, vec!["files/b", "files/d"]);
    }

    #[tokio::test]
    async fn orphan_scan_respects_files_prefix() {
        let store = InMemoryOrphanStore::default();
        let storage = MockStorage::with_listing(vec![vec![
            "files/orphan.txt".to_string(),
            "other-bucket-object".to_string(),
            "logs/debug.log".to_string(),
        ]]);

        let stats = run_orphan_scan_with_store(&store, &storage, &test_orphan_config())
            .await
            .unwrap();

        assert_eq!(stats.objects_scanned, 1);
        assert_eq!(stats.orphans_deleted, 1);
        assert_eq!(
            storage.deleted.lock().await.as_slice(),
            &["files/orphan.txt"]
        );
    }
}
