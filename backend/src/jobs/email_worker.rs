//! Email Worker
//!
//! Background worker that processes the email outbox queue.
//! Handles retries with exponential backoff, rate limiting, and error tracking.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::models::email::{EmailEventType, EmailOutbox, EmailStatus, MailProviderSettings};
use crate::services::email_provider::{EmailAddress, EmailContent, SmtpProvider};

/// Configuration for the email worker
#[derive(Debug, Clone)]
pub struct EmailWorkerConfig {
    /// How often to check for new emails (seconds)
    pub poll_interval_secs: u64,
    /// How many emails to process per batch
    pub batch_size: i64,
    /// Maximum retry attempts
    pub max_retries: i32,
    /// Base delay for exponential backoff (seconds)
    pub retry_base_delay_secs: i64,
    /// Maximum delay between retries (seconds)
    pub retry_max_delay_secs: i64,
    /// Whether to process scheduled emails
    pub process_scheduled: bool,
    /// Rate limit: max emails per second per provider
    pub rate_limit_per_second: f64,
}

impl Default for EmailWorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 10,
            batch_size: 100,
            max_retries: 3,
            retry_base_delay_secs: 60,
            retry_max_delay_secs: 3600,
            process_scheduled: true,
            rate_limit_per_second: 10.0,
        }
    }
}

/// Statistics from a worker run
#[derive(Debug, Default)]
pub struct WorkerStats {
    pub processed: u64,
    pub sent: u64,
    pub failed: u64,
    pub rate_limited: u64,
    pub quiet_hours_delayed: u64,
}

/// Email worker
pub struct EmailWorker {
    db: PgPool,
    config: EmailWorkerConfig,
    encryption_key: String,
}

impl EmailWorker {
    /// Create a new email worker
    pub fn new(db: PgPool, config: EmailWorkerConfig, encryption_key: String) -> Self {
        Self {
            db,
            config,
            encryption_key,
        }
    }

    /// Run the worker loop, exiting cleanly when `shutdown` is cancelled.
    pub async fn run(&self, shutdown: CancellationToken) {
        info!(
            "Email worker started (poll_interval={}s, batch_size={})",
            self.config.poll_interval_secs, self.config.batch_size
        );

        let mut interval = interval(tokio::time::Duration::from_secs(
            self.config.poll_interval_secs,
        ));

        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                _ = interval.tick() => {}
            }

            match self.process_batch().await {
                Ok(stats) => {
                    if stats.processed > 0 {
                        info!(
                            "Email batch processed: {} sent, {} failed, {} rate_limited",
                            stats.sent, stats.failed, stats.rate_limited
                        );
                    }
                }
                Err(e) => {
                    error!("Email worker error: {}", e);
                }
            }
        }
    }

    /// Process a batch of pending emails
    async fn process_batch(&self) -> Result<WorkerStats, sqlx::Error> {
        let mut stats = WorkerStats::default();

        // Get pending emails
        let pending = self.get_pending_emails().await?;

        if pending.is_empty() {
            return Ok(stats);
        }

        debug!("Processing {} pending emails", pending.len());

        for email in pending {
            match self.process_email(email).await {
                Ok(()) => {
                    stats.processed += 1;
                    stats.sent += 1;
                }
                Err(WorkerError::RateLimited) => {
                    stats.rate_limited += 1;
                }
                Err(WorkerError::QuietHours) => {
                    stats.quiet_hours_delayed += 1;
                }
                Err(e) => {
                    error!("Failed to process email: {}", e);
                    stats.failed += 1;
                }
            }

            // Small delay to respect rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(
                (1000.0 / self.config.rate_limit_per_second) as u64,
            ))
            .await;
        }

        Ok(stats)
    }

    /// Get pending emails from the outbox
    async fn get_pending_emails(&self) -> Result<Vec<EmailOutbox>, sqlx::Error> {
        let now = Utc::now();

        let emails: Vec<EmailOutbox> = sqlx::query_as(
            r#"
            SELECT * FROM email_outbox
            WHERE status = 'queued'
              AND (scheduled_at IS NULL OR scheduled_at <= $1)
              AND (send_after IS NULL OR send_after <= $1)
              AND (next_attempt_at IS NULL OR next_attempt_at <= $1)
            ORDER BY 
                CASE priority 
                    WHEN 'high' THEN 1 
                    WHEN 'normal' THEN 2 
                    WHEN 'low' THEN 3 
                END,
                created_at ASC
            LIMIT $2
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .bind(self.config.batch_size)
        .fetch_all(&self.db)
        .await?;

        Ok(emails)
    }

    /// Process a single email
    async fn process_email(&self, email: EmailOutbox) -> Result<(), WorkerError> {
        // Mark as sending
        self.update_status(email.id, EmailStatus::Sending, None, None)
            .await?;

        // Get provider
        let (provider, provider_settings) = match self
            .get_or_create_provider(email.provider_id, email.tenant_id)
            .await
        {
            Ok(provider_and_settings) => provider_and_settings,
            Err(e) => {
                self.handle_delivery_error(&email, &e.to_string()).await?;
                return Err(e);
            }
        };

        // Build addresses
        let from = EmailAddress::with_name(
            &provider_settings.from_address,
            &provider_settings.from_name,
        );
        let to = EmailAddress::new(&email.recipient_email);

        // Build content
        let content = EmailContent {
            subject: email.subject.clone(),
            body_text: email.body_text.clone().unwrap_or_default(),
            body_html: email.body_html.clone(),
            headers: vec![], // Could extract from headers_json
        };

        // Try to send
        match provider.send_email(&from, &to, &content).await {
            Ok(result) => {
                // Success
                self.mark_sent(email.id, &result.server_response).await?;
                info!(
                    "Email sent: id={}, recipient={}",
                    email.id, email.recipient_email
                );
                Ok(())
            }
            Err(e) => {
                self.handle_delivery_error(&email, &e.to_string()).await?;
                Err(WorkerError::Provider(e.to_string()))
            }
        }
    }

    async fn handle_delivery_error(
        &self,
        email: &EmailOutbox,
        error_msg: &str,
    ) -> Result<(), sqlx::Error> {
        let error_category = classify_error(error_msg);

        // Check if we should retry
        if email.attempt_count < email.max_attempts.min(self.config.max_retries) {
            let next_attempt = calculate_backoff(
                email.attempt_count,
                self.config.retry_base_delay_secs,
                self.config.retry_max_delay_secs,
            );

            self.schedule_retry(email.id, &error_category, error_msg, next_attempt)
                .await?;
            warn!(
                "Email failed, scheduled retry: id={}, attempt={}/{}, next_attempt={}",
                email.id,
                email.attempt_count + 1,
                email.max_attempts,
                next_attempt
            );
        } else {
            // Max retries exceeded
            self.mark_failed(email.id, &error_category, error_msg)
                .await?;
            error!(
                "Email failed permanently: id={}, attempts={}, error={}",
                email.id, email.attempt_count, error_msg
            );
        }

        Ok(())
    }

    /// Get or create a mail provider
    async fn get_or_create_provider(
        &self,
        provider_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
    ) -> Result<(SmtpProvider, MailProviderSettings), WorkerError> {
        // First, determine which provider to use
        let settings = if let Some(id) = provider_id {
            // Use specific provider
            self.get_provider_settings(id).await?
        } else {
            // Get default provider for tenant
            self.get_default_provider(tenant_id).await?
        };

        let settings = settings
            .ok_or_else(|| WorkerError::Configuration("No mail provider available".to_string()))?;

        // Create new provider
        let provider = SmtpProvider::new(settings.clone(), &self.encryption_key)
            .await
            .map_err(|e| WorkerError::Configuration(format!("Failed to create provider: {}", e)))?;

        Ok((provider, settings))
    }

    /// Get provider settings by ID
    async fn get_provider_settings(
        &self,
        id: Uuid,
    ) -> Result<Option<MailProviderSettings>, sqlx::Error> {
        let settings: Option<MailProviderSettings> =
            sqlx::query_as("SELECT * FROM mail_provider_settings WHERE id = $1 AND enabled = true")
                .bind(id)
                .fetch_optional(&self.db)
                .await?;

        Ok(settings)
    }

    /// Get default provider for tenant
    async fn get_default_provider(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<MailProviderSettings>, sqlx::Error> {
        let settings: Option<MailProviderSettings> = sqlx::query_as(
            r#"
            SELECT * FROM mail_provider_settings
            WHERE (tenant_id = $1 OR (tenant_id IS NULL AND $1 IS NULL))
              AND enabled = true
              AND is_default = true
            ORDER BY tenant_id NULLS LAST
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.db)
        .await?;

        if settings.is_some() {
            return Ok(settings);
        }

        // If no default, try any enabled provider in the same scope first.
        let any: Option<MailProviderSettings> = sqlx::query_as(
            r#"
            SELECT * FROM mail_provider_settings
            WHERE (tenant_id = $1 OR (tenant_id IS NULL AND $1 IS NULL))
              AND enabled = true
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.db)
        .await?;
        if any.is_some() {
            return Ok(any);
        }

        // If tenant-specific lookup failed, fall back to global provider.
        if tenant_id.is_some() {
            let global_default: Option<MailProviderSettings> = sqlx::query_as(
                r#"
                SELECT * FROM mail_provider_settings
                WHERE tenant_id IS NULL
                  AND enabled = true
                  AND is_default = true
                ORDER BY created_at ASC
                LIMIT 1
                "#,
            )
            .fetch_optional(&self.db)
            .await?;
            if global_default.is_some() {
                warn!(
                    tenant_id = ?tenant_id,
                    "No tenant-specific mail provider configured; falling back to global default provider"
                );
                return Ok(global_default);
            }

            let global_any: Option<MailProviderSettings> = sqlx::query_as(
                r#"
                SELECT * FROM mail_provider_settings
                WHERE tenant_id IS NULL
                  AND enabled = true
                ORDER BY created_at ASC
                LIMIT 1
                "#,
            )
            .fetch_optional(&self.db)
            .await?;
            if global_any.is_some() {
                warn!(
                    tenant_id = ?tenant_id,
                    "No tenant-specific mail provider configured; falling back to first enabled global provider"
                );
                return Ok(global_any);
            }
        }

        Ok(None)
    }

    /// Update email status
    async fn update_status(
        &self,
        id: Uuid,
        status: EmailStatus,
        error_category: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE email_outbox SET
                status = $2,
                last_error_category = COALESCE($3, last_error_category),
                last_error_message = COALESCE($4, last_error_message),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(error_category)
        .bind(error_message)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Mark email as sent
    async fn mark_sent(&self, id: Uuid, _server_response: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE email_outbox SET
                status = 'sent',
                sent_at = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(now)
        .execute(&self.db)
        .await?;

        // Record event
        self.record_event(id, EmailEventType::Sent, None, None)
            .await?;

        Ok(())
    }

    /// Schedule a retry
    async fn schedule_retry(
        &self,
        id: Uuid,
        error_category: &str,
        error_message: &str,
        next_attempt: chrono::DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE email_outbox SET
                status = 'queued',
                attempt_count = attempt_count + 1,
                next_attempt_at = $2,
                last_error_category = $3,
                last_error_message = $4,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(next_attempt)
        .bind(error_category)
        .bind(error_message)
        .execute(&self.db)
        .await?;

        // Record failure event
        self.record_event(
            id,
            EmailEventType::Failed,
            Some(error_category),
            Some(error_message),
        )
        .await?;

        Ok(())
    }

    /// Mark email as permanently failed
    async fn mark_failed(
        &self,
        id: Uuid,
        error_category: &str,
        error_message: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE email_outbox SET
                status = 'failed',
                last_error_category = $2,
                last_error_message = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error_category)
        .bind(error_message)
        .execute(&self.db)
        .await?;

        // Record failure event
        self.record_event(
            id,
            EmailEventType::Failed,
            Some(error_category),
            Some(error_message),
        )
        .await?;

        Ok(())
    }

    /// Record an email event
    async fn record_event(
        &self,
        outbox_id: Uuid,
        event_type: EmailEventType,
        error_category: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        // Get outbox info first
        let outbox: EmailOutbox = sqlx::query_as("SELECT * FROM email_outbox WHERE id = $1")
            .bind(outbox_id)
            .fetch_one(&self.db)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO email_events (
                outbox_id, tenant_id, workflow_key, event_type,
                recipient_email, recipient_user_id, template_family_id, template_version, locale,
                provider_id, error_category, error_message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(outbox_id)
        .bind(outbox.tenant_id)
        .bind(&outbox.workflow_key)
        .bind(event_type.as_str())
        .bind(&outbox.recipient_email)
        .bind(outbox.recipient_user_id)
        .bind(outbox.template_family_id)
        .bind(outbox.template_version)
        .bind(&outbox.locale)
        .bind(outbox.provider_id)
        .bind(error_category)
        .bind(error_message)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

/// Errors specific to the worker
#[derive(Debug)]
#[allow(dead_code)]
enum WorkerError {
    Database(sqlx::Error),
    Configuration(String),
    Provider(String),
    RateLimited,
    QuietHours,
}

impl From<sqlx::Error> for WorkerError {
    fn from(e: sqlx::Error) -> Self {
        WorkerError::Database(e)
    }
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerError::Database(e) => write!(f, "Database error: {}", e),
            WorkerError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            WorkerError::Provider(msg) => write!(f, "Provider error: {}", msg),
            WorkerError::RateLimited => write!(f, "Rate limited"),
            WorkerError::QuietHours => write!(f, "In quiet hours"),
        }
    }
}

/// Calculate exponential backoff
fn calculate_backoff(
    attempt_count: i32,
    base_delay_secs: i64,
    max_delay_secs: i64,
) -> chrono::DateTime<Utc> {
    let delay = base_delay_secs * (2_i64.pow(attempt_count as u32));
    let delay = delay.min(max_delay_secs);

    // Add some jitter (±10%)
    let jitter = (delay as f64 * 0.1) as i64;
    let delay = delay + rand::random::<i64>().rem_euclid(jitter * 2) - jitter;

    Utc::now() + Duration::seconds(delay)
}

/// Classify an error message
fn classify_error(error: &str) -> String {
    let lower = error.to_lowercase();

    if lower.contains("authentication") || lower.contains("auth") || lower.contains("535") {
        "auth_failed".to_string()
    } else if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        "tls_error".to_string()
    } else if lower.contains("dns") || lower.contains("resolve") {
        "dns_error".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout".to_string()
    } else if lower.contains("relay") || lower.contains("denied") || lower.contains("550") {
        "relay_denied".to_string()
    } else if lower.contains("rate") || lower.contains("throttle") || lower.contains("421") {
        "rate_limited".to_string()
    } else if lower.contains("recipient") || lower.contains("mailbox") {
        "invalid_recipient".to_string()
    } else {
        "smtp_error".to_string()
    }
}

/// Spawn the email worker as a background task
pub fn spawn_email_worker(
    db: PgPool,
    config: EmailWorkerConfig,
    encryption_key: String,
    shutdown: CancellationToken,
) {
    tokio::spawn(async move {
        let mut restart_delay_secs = 1u64;

        loop {
            let db_for_run = db.clone();
            let config_for_run = config.clone();
            let encryption_key_for_run = encryption_key.clone();
            let shutdown_for_run = shutdown.clone();

            let run_handle = tokio::spawn(async move {
                let worker = EmailWorker::new(db_for_run, config_for_run, encryption_key_for_run);
                worker.run(shutdown_for_run).await;
            });

            match run_handle.await {
                Ok(()) => {
                    warn!("Email worker exited unexpectedly; restarting");
                }
                Err(join_error) => {
                    error!(error = %join_error, "Email worker panicked; restarting");
                }
            }

            tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(restart_delay_secs)) => {}
            }
            restart_delay_secs = (restart_delay_secs * 2).min(60);
        }
    });

    info!("Email worker supervisor started");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_email_worker_accepts_cancellation_token() {
        // Compile-time check that the public spawn API accepts a shutdown token.
        let _: fn(PgPool, EmailWorkerConfig, String, CancellationToken) = spawn_email_worker;
    }
}
