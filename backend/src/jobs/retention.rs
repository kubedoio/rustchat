//! Retention job for scheduled data cleanup
//!
//! This module provides a background task that periodically cleans up
//! old messages and files based on the server's retention configuration.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::time::Duration as StdDuration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Retention job configuration
#[derive(Debug, Clone)]
pub struct RetentionConfig {
    pub message_retention_days: i64,
    pub file_retention_days: i64,
}

/// Run the retention cleanup job
pub async fn run_retention_cleanup(
    db: &PgPool,
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

        // Get files to delete (for S3 cleanup)
        let files: Vec<(String,)> =
            sqlx::query_as("SELECT s3_key FROM files WHERE created_at < $1")
                .bind(cutoff)
                .fetch_all(db)
                .await?;

        stats.file_keys = files.into_iter().map(|f| f.0).collect();

        let result = sqlx::query("DELETE FROM files WHERE created_at < $1")
            .bind(cutoff)
            .execute(db)
            .await?;

        stats.files_deleted = result.rows_affected();
        info!(
            "Retention: Deleted {} files older than {} days",
            stats.files_deleted, config.file_retention_days
        );
    }

    Ok(stats)
}

/// Statistics from a retention cleanup run
#[derive(Debug, Default)]
pub struct RetentionStats {
    pub messages_deleted: u64,
    pub files_deleted: u64,
    pub file_keys: Vec<String>,
}

/// Spawn the retention job as a background task
pub fn spawn_retention_job(db: PgPool, shutdown: CancellationToken) {
    tokio::spawn(async move {
        let mut restart_delay_secs = 1u64;

        loop {
            let db_for_run = db.clone();
            let shutdown_for_run = shutdown.clone();
            let run_handle = tokio::spawn(async move {
                run_retention_loop(db_for_run, shutdown_for_run).await;
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

async fn run_retention_loop(db: PgPool, shutdown: CancellationToken) {
    // Run every hour
    let mut interval = tokio::time::interval(StdDuration::from_secs(3600));

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

                    match run_retention_cleanup(&db, config).await {
                        Ok(stats) => {
                            if stats.messages_deleted > 0 || stats.files_deleted > 0 {
                                info!(
                                    "Retention cleanup complete: {} messages, {} files deleted",
                                    stats.messages_deleted, stats.files_deleted
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_retention_job_accepts_cancellation_token() {
        // Compile-time check that the public spawn API accepts a shutdown token.
        let _: fn(PgPool, CancellationToken) = spawn_retention_job;
    }
}
