//! Retention job for scheduled data cleanup
//!
//! This module provides a background task that periodically cleans up
//! old messages and files based on the server's retention configuration.

use std::collections::HashSet;
use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
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
    let mut stats = RetentionStats::default();

    // Clean up old messages
    if config.message_retention_days > 0 {
        let cutoff = Utc::now() - Duration::days(config.message_retention_days);

        let result = sqlx::query("DELETE FROM posts WHERE created_at < $1 AND NOT is_pinned")
            .bind(cutoff)
            .execute(db)
            .await?;

        stats.messages_deleted = result.rows_affected();
        info!(
            "Retention: Deleted {} messages older than {} days",
            stats.messages_deleted, config.message_retention_days
        );
    }

    // Clean up old files
    if config.file_retention_days > 0 {
        let cutoff = Utc::now() - Duration::days(config.file_retention_days);

        let files: Vec<(String,)> = sqlx::query_as("SELECT key FROM files WHERE created_at < $1")
            .bind(cutoff)
            .fetch_all(db)
            .await?;

        let keys: Vec<String> = files.into_iter().map(|f| f.0).collect();
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
            let result = sqlx::query("DELETE FROM files WHERE key = ANY($1::text[])")
                .bind(&deleted_keys)
                .execute(db)
                .await?;

            stats.files_deleted = result.rows_affected();
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
            .list_objects(None, continuation_token.as_deref(), max_keys)
            .await?;

        process_orphan_page(db, storage, &page, &mut stats).await?;

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

async fn process_orphan_page<S: ObjectStorage>(
    db: &PgPool,
    storage: &S,
    page: &ListObjectsResult,
    stats: &mut OrphanScanStats,
) -> Result<(), sqlx::Error> {
    if page.keys.is_empty() {
        return Ok(());
    }

    let existing_keys: Vec<(String,)> =
        sqlx::query_as("SELECT key FROM files WHERE key = ANY($1::text[])")
            .bind(&page.keys)
            .fetch_all(db)
            .await?;

    let existing: HashSet<String> = existing_keys.into_iter().map(|r| r.0).collect();
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

    #[derive(Clone, Default)]
    struct MockStorage {
        deleted: Arc<Mutex<Vec<String>>>,
        listed: Arc<Mutex<Vec<Vec<String>>>>,
        fail_keys: Arc<HashSet<String>>,
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
    }

    #[async_trait::async_trait]
    impl ObjectStorage for MockStorage {
        async fn delete_object(&self, key: &str) -> Result<(), AppError> {
            if self.fail_keys.contains(key) {
                return Err(AppError::ExternalService(format!(
                    "mock delete failed for {}",
                    key
                )));
            }
            self.deleted.lock().await.push(key.to_string());
            Ok(())
        }

        async fn list_objects(
            &self,
            _prefix: Option<&str>,
            _continuation_token: Option<&str>,
            _max_keys: i32,
        ) -> Result<ListObjectsResult, AppError> {
            let mut listed = self.listed.lock().await;
            let keys = listed.remove(0);
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

    #[test]
    fn spawn_retention_job_accepts_expected_arguments() {
        // Compile-time check that the public spawn API matches the expected signature.
        let _: fn(PgPool, crate::storage::S3Client, RetentionJobConfig, CancellationToken) =
            spawn_retention_job;
    }

    #[test]
    fn find_orphan_keys_filters_existing() {
        let keys = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let existing: HashSet<String> = ["a".to_string(), "c".to_string()].into_iter().collect();

        let orphans: Vec<String> = keys
            .into_iter()
            .filter(|key| !existing.contains(key))
            .collect();

        assert_eq!(orphans, vec!["b"]);
    }

    #[tokio::test]
    async fn mock_storage_records_deletions_and_failures() {
        let storage = MockStorage::with_failed_keys(&["bad"]);

        assert!(storage.delete_object("good").await.is_ok());
        assert!(storage.delete_object("bad").await.is_err());

        let deleted = storage.deleted.lock().await;
        assert_eq!(deleted.as_slice(), &["good"]);
    }

    #[tokio::test]
    async fn mock_storage_paginates_listings() {
        let storage = MockStorage::with_listing(vec![vec!["a".to_string()], vec!["b".to_string()]]);

        let first = storage.list_objects(None, None, 10).await.unwrap();
        assert_eq!(first.keys, vec!["a"]);
        assert!(first.next_continuation_token.is_some());

        let second = storage.list_objects(None, None, 10).await.unwrap();
        assert_eq!(second.keys, vec!["b"]);
        assert!(second.next_continuation_token.is_none());
    }
}
