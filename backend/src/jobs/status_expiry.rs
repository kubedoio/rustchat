//! Background worker for clearing expired custom statuses.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn};
use uuid::Uuid;

use crate::api::v4::status;
use crate::api::AppState;
use crate::error::ApiResult;

pub fn spawn_custom_status_expiry_worker(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval_secs = state
            .config
            .messaging
            .status_expiry_poll_interval_seconds
            .max(1);
        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;
                _ = state.shutdown.cancelled() => break,
                _ = ticker.tick() => {}
            }

            match run_custom_status_expiry_cleanup(&state).await {
                Ok(expired_user_ids) => {
                    if !expired_user_ids.is_empty() {
                        info!(
                            expired_user_count = expired_user_ids.len(),
                            "Cleared expired custom statuses"
                        );
                    }
                }
                Err(err) => {
                    warn!(error = %err, "Custom status expiry cleanup failed");
                }
            }
        }
    })
}

pub async fn run_custom_status_expiry_cleanup(state: &AppState) -> ApiResult<Vec<Uuid>> {
    let expired_user_ids = status::clear_expired_custom_statuses(state).await?;

    for user_id in &expired_user_ids {
        if let Err(err) = status::broadcast_status_change(state, *user_id).await {
            warn!(
                error = %err,
                user_id = %user_id,
                "Failed to broadcast expired custom status cleanup"
            );
        }
    }

    Ok(expired_user_ids)
}
