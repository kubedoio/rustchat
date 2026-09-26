//! Transactional outbox for integration event delivery.
//!
//! Rows are written in the same database transaction as the RustChat state
//! change they describe, then drained asynchronously by a dispatcher
//! (see `buzz::dispatcher`). Delivery retries are bounded with exponential
//! backoff plus jitter; rows that exhaust their attempts (or fail with a
//! terminal error) are dead-lettered and remain inspectable and retryable by
//! an administrator.

use std::time::Duration;

use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

/// Delivery status of an outbox row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    InFlight,
    Delivered,
    DeadLetter,
}

impl OutboxStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutboxStatus::Pending => "pending",
            OutboxStatus::InFlight => "in_flight",
            OutboxStatus::Delivered => "delivered",
            OutboxStatus::DeadLetter => "dead_letter",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(OutboxStatus::Pending),
            "in_flight" => Some(OutboxStatus::InFlight),
            "delivered" => Some(OutboxStatus::Delivered),
            "dead_letter" => Some(OutboxStatus::DeadLetter),
            _ => None,
        }
    }
}

/// An outbox row as stored in `integration_outbox`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OutboxRecord {
    pub id: Uuid,
    pub provider: String,
    pub connection_id: Option<Uuid>,
    pub idempotency_key: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub next_attempt_at: chrono::DateTime<chrono::Utc>,
    pub last_error: Option<String>,
    pub remote_event_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Bounded exponential backoff with jitter.
///
/// `delay(attempts) = min(base * 2^(attempts-1), max)` scaled by a uniform
/// jitter factor in `[1, 1 + jitter_fraction]` so that multiple failing
/// deliveries do not retry in lockstep (thundering herd against the remote).
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base: Duration,
    pub max: Duration,
    /// Jitter fraction in `0.0..=1.0` (0.2 = up to +20%).
    pub jitter_fraction: f64,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32, base: Duration, max: Duration) -> Self {
        Self {
            max_attempts,
            base,
            max,
            jitter_fraction: 0.2,
        }
    }

    /// Delay before the next attempt, given the number of attempts already
    /// made (1 after the first failure).
    pub fn delay_for_attempt(&self, attempts: u32) -> Duration {
        let exp = self
            .base
            .saturating_mul(2u32.saturating_pow(attempts.saturating_sub(1)));
        let capped = exp.min(self.max);
        let jitter = 1.0 + rand::thread_rng().gen::<f64>() * self.jitter_fraction;
        capped.mul_f64(jitter)
    }

    /// Whether one more attempt is allowed after `attempts` failures.
    pub fn may_retry(&self, attempts: u32) -> bool {
        attempts < self.max_attempts
    }
}

/// Result of a delivery attempt against a remote system.
#[derive(Debug, Clone)]
pub enum DeliveryOutcome {
    /// Delivered; `remote_id` is the remote event identifier when known.
    Delivered { remote_id: Option<String> },
    /// Transient failure; retry with backoff. Covers 5xx, 429, network and
    /// timeout errors. Timeouts are intentionally retryable: deliveries are
    /// idempotent by construction (stable remote event id), so a retry can
    /// never double-deliver.
    Retryable { reason: String },
    /// Permanent failure; dead-letter immediately (4xx auth/validation).
    Terminal { reason: String },
}

/// Data-access for the `integration_outbox` table.
pub struct OutboxRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> OutboxRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// The underlying pool (for status-count queries).
    pub(crate) fn pool(&self) -> &PgPool {
        self.pool
    }

    /// Atomically claim up to `limit` due rows for delivery.
    ///
    /// Uses `FOR UPDATE SKIP LOCKED` inside a transaction so multiple
    /// dispatcher instances (or a dispatcher racing the admin retry endpoint)
    /// never claim the same row. Claimed rows are flipped to `in_flight` with
    /// `attempts` incremented and a fresh `updated_at` lease timestamp.
    pub async fn claim_due(
        &self,
        provider: &str,
        limit: i64,
    ) -> Result<Vec<OutboxRecord>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, OutboxRecord>(
            r#"
            UPDATE integration_outbox
            SET status = 'in_flight',
                attempts = attempts + 1,
                updated_at = now()
            WHERE id IN (
                SELECT id FROM integration_outbox
                WHERE provider = $1
                  AND status = 'pending'
                  AND next_attempt_at <= now()
                ORDER BY next_attempt_at, id
                LIMIT $2
                FOR UPDATE SKIP LOCKED
            )
            RETURNING *
            "#,
        )
        .bind(provider)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(rows)
    }

    /// Reset rows stuck in `in_flight` past the lease window to `pending`.
    ///
    /// Crash recovery: if a dispatcher dies mid-delivery its claims are
    /// reclaimed after the lease expires and retried. Retries are safe
    /// because delivery is idempotent (stable remote event id).
    pub async fn reclaim_stale_in_flight(
        &self,
        provider: &str,
        lease: Duration,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE integration_outbox
            SET status = 'pending', updated_at = now()
            WHERE provider = $1
              AND status = 'in_flight'
              AND updated_at < now() - make_interval(secs => $2)
            "#,
        )
        .bind(provider)
        .bind(lease.as_secs() as f64)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Record a successful delivery.
    pub async fn mark_delivered(
        &self,
        id: Uuid,
        remote_event_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        // Defensive cap: the stored column is VARCHAR(64); a longer value
        // would error and leave the row in_flight forever (relay-controlled
        // data). Cap at 64 characters (char-boundary safe — VARCHAR counts
        // characters, and byte-slicing could panic on a multi-byte char) and
        // never store more than the column allows.
        let remote_event_id: Option<String> = remote_event_id.and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.chars().take(64).collect::<String>())
            }
        });
        // A row that left `in_flight` (e.g. dead-lettered by a concurrent key
        // rotation or connection deletion) must not be resurrected by a late
        // outcome: guard on the in-flight state so the rotation/delete wins.
        sqlx::query(
            r#"
            UPDATE integration_outbox
            SET status = 'delivered',
                remote_event_id = $2,
                last_error = NULL,
                updated_at = now()
            WHERE id = $1 AND status = 'in_flight'
            "#,
        )
        .bind(id)
        .bind(remote_event_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Record a failed attempt: retry with backoff or dead-letter.
    pub async fn mark_failed(
        &self,
        id: Uuid,
        outcome: &DeliveryOutcome,
        next_attempt_at: chrono::DateTime<chrono::Utc>,
        dead_letter: bool,
    ) -> Result<(), sqlx::Error> {
        let reason = match outcome {
            DeliveryOutcome::Retryable { reason } | DeliveryOutcome::Terminal { reason } => {
                truncate_error(reason)
            }
            DeliveryOutcome::Delivered { .. } => unreachable!("mark_failed on Delivered"),
        };
        sqlx::query(
            r#"
            UPDATE integration_outbox
            SET status = CASE WHEN $3 THEN 'dead_letter' ELSE 'pending' END,
                next_attempt_at = $2,
                last_error = $4,
                updated_at = now()
            WHERE id = $1 AND status = 'in_flight'
            "#,
        )
        .bind(id)
        .bind(next_attempt_at)
        .bind(dead_letter)
        .bind(reason)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Admin retry: requeue a dead-lettered row for immediate delivery.
    pub async fn requeue_dead_letter(&self, provider: &str, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE integration_outbox
            SET status = 'pending',
                attempts = 0,
                next_attempt_at = now(),
                last_error = NULL,
                updated_at = now()
            WHERE id = $1 AND provider = $2 AND status = 'dead_letter'
            "#,
        )
        .bind(id)
        .bind(provider)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// List deliveries for a connection (optionally filtered by status).
    pub async fn list_for_connection(
        &self,
        connection_id: Uuid,
        status: Option<&OutboxStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OutboxRecord>, sqlx::Error> {
        sqlx::query_as::<_, OutboxRecord>(
            r#"
            SELECT * FROM integration_outbox
            WHERE connection_id = $1 AND ($2::varchar IS NULL OR status = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(connection_id)
        .bind(status.map(|s| s.as_str()))
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
    }
}

/// Bound stored error strings so a hostile remote response cannot balloon
/// row/log size. Remote responses are never echoed verbatim beyond this cap.
fn truncate_error(reason: &str) -> String {
    const MAX: usize = 512;
    if reason.len() <= MAX {
        reason.to_string()
    } else {
        let mut end = MAX;
        while !reason.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &reason[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_is_bounded_and_grows() {
        let policy = RetryPolicy::new(10, Duration::from_secs(5), Duration::from_secs(3600));
        let d1 = policy.delay_for_attempt(1);
        // Base with jitter: [5s, 6s) (uniform factor in [1, 1.2)).
        assert!(d1 >= Duration::from_secs(5) && d1 <= Duration::from_secs(6));
        let d5 = policy.delay_for_attempt(5);
        assert!(d5 > d1, "backoff must grow with attempts");
        let d30 = policy.delay_for_attempt(30);
        // Capped at max + jitter: < 3600 * 1.2 = 4320s.
        assert!(d30 <= Duration::from_secs(4320));
    }

    #[test]
    fn retry_policy_respects_max_attempts() {
        let policy = RetryPolicy::new(3, Duration::from_secs(1), Duration::from_secs(10));
        assert!(policy.may_retry(2));
        assert!(!policy.may_retry(3));
    }

    #[test]
    fn error_truncation_respects_char_boundaries() {
        let reason = "é".repeat(600);
        let truncated = truncate_error(&reason);
        // Truncated to MAX bytes (512, on a char boundary) plus the ellipsis.
        assert!(truncated.len() <= 515);
        assert!(truncated.ends_with('…'));
        assert!(truncated.starts_with("é".repeat(256).as_str()));
    }
}
