//! Upload-session expiry cleanup
//!
//! Upload sessions buffer uploaded bytes in the database (`upload_sessions.
//! file_data` bytea) until the client finalizes them. Sessions carry a 24h
//! `expires_at`, but nothing enforced it: an abandoned session (client
//! crashed, never finalized) leaked its row — and every buffered byte —
//! forever, letting any channel member bloat the database unboundedly by
//! repeatedly starting uploads they never finish.
//!
//! This worker drains expired sessions in bounded batches, oldest first.

use sqlx::PgPool;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::repositories::UploadRepository;

/// Rows (and buffered bytes) removed per batch, to keep each DELETE
/// transaction bounded even after a long worker outage.
const DELETE_BATCH_SIZE: i64 = 500;

/// How often the worker scans for expired sessions.
const SCAN_INTERVAL: Duration = Duration::from_secs(3600);

/// Spawn the upload-session expiry cleanup worker.
pub fn spawn_upload_session_cleanup(
    db: PgPool,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = interval(SCAN_INTERVAL);

        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                _ = ticker.tick() => {}
            }

            if let Err(e) = run_cleanup_pass(&db).await {
                error!(error = %e, "Upload-session cleanup pass failed");
            }
        }

        info!("Upload-session cleanup worker stopped");
    })
}

/// Drain all currently-expired sessions in bounded batches.
async fn run_cleanup_pass(db: &PgPool) -> Result<(), sqlx::Error> {
    let repo = UploadRepository::new(db);
    let mut total_removed: u64 = 0;

    loop {
        let removed = repo.delete_expired_sessions(DELETE_BATCH_SIZE).await?;
        total_removed += removed;
        if removed < DELETE_BATCH_SIZE as u64 {
            break;
        }
    }

    if total_removed > 0 {
        info!(
            sessions_removed = total_removed,
            "Expired upload sessions purged"
        );
    } else {
        // Quiet log for a healthy system; keep at debug to avoid noise.
        tracing::debug!("No expired upload sessions to purge");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_scan_interval_is_not_tight() {
        // The worker scans hourly; a tighter interval would hammer the
        // sessions table for no benefit (sessions live 24h).
        assert!(SCAN_INTERVAL >= Duration::from_secs(60));
    }
}
