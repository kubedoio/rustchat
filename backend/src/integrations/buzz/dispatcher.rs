//! Outbox dispatcher for the Buzz bridge.
//!
//! A single application-lifetime worker (spawned only when the integration
//! is enabled) that:
//!
//! 1. reclaims `in_flight` rows whose lease expired (crash recovery),
//! 2. claims due `pending` rows (`FOR UPDATE SKIP LOCKED` — multi-instance
//!    safe),
//! 3. resolves each row's connection, rebuilds the Nostr event
//!    deterministically from the stored payload and the row's enqueue
//!    timestamp, and submits it through the [`BuzzConnector`],
//! 4. records the outcome: delivered, retry with bounded exponential
//!    backoff + jitter, or dead-letter (terminal errors / attempt budget
//!    exhausted).
//!
//! Every code path is failure-isolated: an error while loading a connection
//! or building an event dead-letters only that row and the loop continues.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sqlx::PgConnection;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::BuzzIntegrationConfig;
use crate::error::AppError;
use crate::integrations::buzz::connector::{BuzzConnectionRuntime, BuzzConnectorProvider};
use crate::integrations::buzz::events::{build_stream_message_event, MessageCreatedPayload};
use crate::integrations::buzz::repository::BuzzRepository;
use crate::integrations::outbox::{DeliveryOutcome, OutboxRecord, OutboxRepository, RetryPolicy};
use crate::state::AppState;
use crate::telemetry::metrics;

/// Outbound event type written by the enqueue path.
pub const EVENT_MESSAGE_CREATED: &str = "message.created";

/// Enqueue `message.created` outbox rows for a newly created post.
///
/// Called **inside the post-creation transaction** so the RustChat state
/// change and the integration intent commit atomically (transactional
/// outbox). No-op when the channel has no active mapping.
///
/// `author_label` is the human-readable attribution rendered into the
/// bridged message content.
pub async fn enqueue_message_created_in_tx(
    tx: &mut PgConnection,
    rustchat_channel_id: Uuid,
    payload_post_id: Uuid,
    payload_root_post_id: Option<Uuid>,
    author_label: &str,
    content: &str,
) -> Result<(), AppError> {
    let mappings: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT m.connection_id, m.buzz_channel_id
        FROM buzz_channel_mappings m
        JOIN buzz_connections c ON c.id = m.connection_id
        WHERE m.rustchat_channel_id = $1
          AND m.outbound_enabled
          AND c.enabled
        "#,
    )
    .bind(rustchat_channel_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    for (connection_id, buzz_channel_id) in mappings {
        let payload = MessageCreatedPayload {
            post_id: payload_post_id,
            rustchat_channel_id,
            buzz_channel_id,
            author_label: author_label.to_string(),
            content: content.to_string(),
            root_post_id: payload_root_post_id,
        };
        // Duplicate enqueue (e.g. retried request that hit the
        // client_msg_id dedup path) violates the unique constraint and is
        // skipped — the row already exists.
        let _ = sqlx::query(
            r#"
            INSERT INTO integration_outbox
                (provider, connection_id, idempotency_key, event_type, payload)
            VALUES ('buzz', $1, $2, $3, $4)
            ON CONFLICT (provider, idempotency_key) DO NOTHING
            "#,
        )
        .bind(connection_id)
        .bind(MessageCreatedPayload::idempotency_key(
            connection_id,
            payload_post_id,
        ))
        .bind(EVENT_MESSAGE_CREATED)
        .bind(serde_json::to_value(&payload).map_err(|e| AppError::Internal(e.to_string()))?)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// Resolve the runtime (decrypted key) for a claimed row's connection.
///
/// Missing or disabled connections dead-letter the row: there is no point
/// retrying against a credential that no longer exists.
async fn resolve_connection(
    repo: &BuzzRepository<'_>,
    record: &OutboxRecord,
    encryption_key: &str,
) -> Result<BuzzConnectionRuntime, DeliveryOutcome> {
    let connection = match record.connection_id {
        None => None,
        Some(id) => repo
            .get_connection(id)
            .await
            .map_err(|e| DeliveryOutcome::Terminal {
                reason: format!("connection lookup failed: {e}"),
            })?,
    };

    let connection = match connection {
        Some(c) if c.enabled => c,
        _ => {
            return Err(DeliveryOutcome::Terminal {
                reason: "connection is missing or disabled".to_string(),
            })
        }
    };

    connection
        .into_runtime(encryption_key)
        .map_err(|e| DeliveryOutcome::Terminal {
            reason: format!("{e}"),
        })
}

/// Deliver one claimed row. Returns the delivery outcome to record.
async fn deliver_row(
    state: &AppState,
    provider: &dyn BuzzConnectorProvider,
    record: &OutboxRecord,
) -> DeliveryOutcome {
    let repo = BuzzRepository::new(&state.db);

    let runtime = match resolve_connection(&repo, record, &state.config.encryption_key).await {
        Ok(rt) => rt,
        Err(outcome) => return outcome,
    };

    match record.event_type.as_str() {
        EVENT_MESSAGE_CREATED => {
            let payload: MessageCreatedPayload =
                match serde_json::from_value(record.payload.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        return DeliveryOutcome::Terminal {
                            reason: format!("unparseable outbox payload: {e}"),
                        }
                    }
                };
            let signed = match build_stream_message_event(
                &runtime,
                &payload,
                record.created_at.timestamp().max(0) as u64,
            ) {
                Ok(s) => s,
                Err(e) => return DeliveryOutcome::Terminal { reason: e },
            };
            match provider.connector_for(&runtime).await {
                Ok(connector) => connector.submit_event(&signed).await,
                Err(e) => DeliveryOutcome::Retryable {
                    reason: format!("connector construction failed: {e}"),
                },
            }
        }
        other => DeliveryOutcome::Terminal {
            reason: format!("unknown event type: {other}"),
        },
    }
}

/// Record a delivery outcome for a claimed row (shared by dispatcher and
/// tests): status transition, metrics, and next attempt scheduling.
async fn record_outcome(
    outbox: &OutboxRepository<'_>,
    policy: &RetryPolicy,
    record: &OutboxRecord,
    outcome: &DeliveryOutcome,
    dead_letter: bool,
) {
    match outcome {
        DeliveryOutcome::Delivered { remote_id } => {
            if let Err(e) = outbox.mark_delivered(record.id, remote_id.as_deref()).await {
                tracing::error!(outbox_id = %record.id, error = %e, "failed to mark outbox row delivered");
            }
            metrics::INTEGRATION_OUTBOX_EVENTS_TOTAL
                .with_label_values(&["buzz", "delivered"])
                .inc();
        }
        failed => {
            let next_attempt_at = if dead_letter {
                record.next_attempt_at
            } else {
                Utc::now()
                    + chrono::Duration::from_std(
                        policy.delay_for_attempt(record.attempts.max(1) as u32),
                    )
                    .unwrap_or_else(|_| chrono::Duration::seconds(60))
            };
            if let Err(e) = outbox
                .mark_failed(record.id, failed, next_attempt_at, dead_letter)
                .await
            {
                tracing::error!(outbox_id = %record.id, error = %e, "failed to record outbox failure");
            }
            let outcome_label = if dead_letter { "dead_letter" } else { "retry" };
            metrics::INTEGRATION_OUTBOX_EVENTS_TOTAL
                .with_label_values(&["buzz", outcome_label])
                .inc();
        }
    }
    update_outbox_gauges(outbox).await;
}

/// Refresh the pending/dead-letter gauges (bounded cardinality: status only).
async fn update_outbox_gauges(outbox: &OutboxRepository<'_>) {
    let pending = count_status(outbox, "pending").await;
    let dead = count_status(outbox, "dead_letter").await;
    metrics::INTEGRATION_OUTBOX_PENDING
        .with_label_values(&["buzz", "pending"])
        .set(pending);
    metrics::INTEGRATION_OUTBOX_PENDING
        .with_label_values(&["buzz", "dead_letter"])
        .set(dead);
}

async fn count_status(outbox: &OutboxRepository<'_>, status: &str) -> f64 {
    #[derive(sqlx::FromRow)]
    struct CountRow {
        count: i64,
    }
    let row: Result<CountRow, _> = sqlx::query_as(
        "SELECT COUNT(*) AS count FROM integration_outbox WHERE provider = 'buzz' AND status = $1",
    )
    .bind(status)
    .fetch_one(outbox.pool())
    .await;
    row.map(|r| r.count as f64).unwrap_or(0.0)
}

/// One dispatch cycle: reclaim stale leases, claim due rows, deliver.
///
/// Exposed for tests so a full cycle runs deterministically without the
/// poll-interval sleep.
pub async fn dispatch_cycle(
    state: &AppState,
    provider: &dyn BuzzConnectorProvider,
    config: &BuzzIntegrationConfig,
) {
    let policy = RetryPolicy::new(
        config.max_attempts,
        Duration::from_secs(config.backoff_base_secs),
        Duration::from_secs(config.backoff_max_secs),
    );
    let outbox = OutboxRepository::new(&state.db);

    if config.in_flight_lease_secs > 0 {
        match outbox
            .reclaim_stale_in_flight("buzz", Duration::from_secs(config.in_flight_lease_secs))
            .await
        {
            Ok(0) | Err(_) => {}
            Ok(n) => {
                tracing::warn!(reclaimed = n, "reclaimed expired in-flight buzz deliveries");
            }
        }
    }

    let rows = match outbox.claim_due("buzz", config.batch_size as i64).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "buzz outbox claim failed");
            return;
        }
    };

    for record in rows {
        let outcome = deliver_row(state, provider, &record).await;
        // `attempts` was incremented by the claim, so it already counts the
        // attempt that just failed; dead-letter when the budget is exhausted.
        let dead_letter = match &outcome {
            DeliveryOutcome::Delivered { .. } => false,
            DeliveryOutcome::Terminal { .. } => true,
            DeliveryOutcome::Retryable { .. } => {
                (record.attempts.max(0) as u32) >= config.max_attempts
            }
        };
        record_outcome(&outbox, &policy, &record, &outcome, dead_letter).await;
    }
}

/// Spawn the outbox dispatcher worker (application lifetime).
pub fn spawn_outbox_dispatcher(
    state: AppState,
    provider: Arc<dyn BuzzConnectorProvider>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let config = state.config.integrations.buzz.clone();
    tokio::spawn(async move {
        tracing::info!(
            poll_interval_secs = config.poll_interval_secs,
            "buzz outbox dispatcher started"
        );
        loop {
            let sleep = tokio::time::sleep(Duration::from_secs(config.poll_interval_secs.max(1)));
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("buzz outbox dispatcher stopped");
                    break;
                }
                _ = sleep => {}
            }
            dispatch_cycle(&state, provider.as_ref(), &config).await;
        }
    })
}
