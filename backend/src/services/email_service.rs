//! Email Service
//!
//! High-level email service that manages the outbox, template rendering,
//! and integration with the mail provider.

use chrono::{DateTime, Duration, Timelike, Utc};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::models::email::*;
use crate::services::email_provider::{EmailAddress, EmailContent};
use crate::services::template_renderer::{RenderContext, TemplateRenderer};

/// Errors that can occur in the email service
#[derive(Debug, thiserror::Error)]
pub enum EmailServiceError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Template error: {0}")]
    Template(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Workflow disabled: {0}")]
    WorkflowDisabled(String),
    #[error("User opted out: {0}")]
    UserOptedOut(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Quiet hours: {0}")]
    QuietHours(String),
}

pub type EmailServiceResult<T> = Result<T, EmailServiceError>;

/// High-level email service
pub struct EmailService {
    db: PgPool,
    renderer: TemplateRenderer,
}

impl EmailService {
    /// Create a new email service
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            renderer: TemplateRenderer::new(),
        }
    }

    /// Enqueue an email for sending
    pub async fn enqueue_email(
        &self,
        workflow_key: &str,
        recipient_email: &str,
        recipient_user_id: Option<Uuid>,
        payload: serde_json::Value,
        options: EnqueueOptions,
    ) -> EmailServiceResult<Uuid> {
        // Get tenant from recipient if provided
        let tenant_id = if let Some(user_id) = recipient_user_id {
            self.get_user_tenant(user_id).await?
        } else {
            None
        };

        // Check workflow is enabled
        let workflow = self.get_workflow(workflow_key, tenant_id).await?;
        if !workflow.enabled {
            return Err(EmailServiceError::WorkflowDisabled(format!(
                "Workflow '{}' is disabled",
                workflow_key
            )));
        }

        // Check user preferences (if we have a user_id)
        if let Some(user_id) = recipient_user_id {
            self.check_user_preferences(workflow_key, user_id).await?;
        }

        // Check throttling
        if let Some(ref throttle_key) = options.throttle_key {
            if self
                .is_throttled(throttle_key, &workflow.policy_json.0)
                .await?
            {
                return Err(EmailServiceError::RateLimited(format!(
                    "Email throttled for key: {}",
                    throttle_key
                )));
            }
        }

        // Get template
        let (template_family, template_version) = self
            .get_template_for_workflow(&workflow, options.locale.as_deref())
            .await?;

        // Render the template
        let context = RenderContext::new()
            .with_variables(&payload)
            .map_err(|e| EmailServiceError::Template(e.to_string()))?;

        let rendered = self
            .renderer
            .render_email(&template_version, &context)
            .map_err(|e| EmailServiceError::Template(e.to_string()))?;

        // Calculate send_after for quiet hours
        let send_after = if let Some(user_id) = recipient_user_id {
            self.calculate_send_after(user_id, &workflow.policy_json.0)
                .await?
        } else {
            None
        };

        // Insert into outbox
        let outbox_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO email_outbox (
                tenant_id, workflow_key, template_family_id, template_version, locale,
                recipient_email, recipient_user_id, subject, body_text, body_html,
                payload_json, headers_json, status, priority, scheduled_at, send_after,
                throttle_key, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING id
            "#,
        )
        .bind(tenant_id)
        .bind(workflow_key)
        .bind(template_family.id)
        .bind(template_version.version)
        .bind(template_version.locale)
        .bind(recipient_email)
        .bind(recipient_user_id)
        .bind(rendered.subject)
        .bind(rendered.body_text)
        .bind(rendered.body_html)
        .bind(sqlx::types::Json(payload))
        .bind(options.headers.map(sqlx::types::Json))
        .bind(EmailStatus::Queued)
        .bind(options.priority)
        .bind(options.scheduled_at)
        .bind(send_after)
        .bind(options.throttle_key)
        .bind(options.created_by)
        .fetch_one(&self.db)
        .await?;

        // Record the event
        self.record_event(
            Some(outbox_id),
            tenant_id,
            workflow_key,
            EmailEventType::Queued,
            recipient_email,
            recipient_user_id,
            None,
            None,
        )
        .await?;

        info!(
            "Email enqueued: id={}, workflow={}, recipient={}",
            outbox_id, workflow_key, recipient_email
        );

        Ok(outbox_id)
    }

    /// Send an email immediately (bypasses outbox)
    pub async fn send_immediate(
        &self,
        provider: &crate::services::email_provider::SmtpProvider,
        from: &EmailAddress,
        to: &EmailAddress,
        content: EmailContent,
    ) -> EmailServiceResult<crate::services::email_provider::SendResult> {
        provider
            .send_email(from, to, &content)
            .await
            .map_err(|e| EmailServiceError::Provider(e.to_string()))
    }

    /// Get user's tenant ID
    async fn get_user_tenant(&self, user_id: Uuid) -> EmailServiceResult<Option<Uuid>> {
        let tenant_id: Option<(Option<Uuid>,)> =
            sqlx::query_as("SELECT org_id FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.db)
                .await?;

        Ok(tenant_id.and_then(|t| t.0))
    }

    /// Get workflow configuration
    async fn get_workflow(
        &self,
        workflow_key: &str,
        tenant_id: Option<Uuid>,
    ) -> EmailServiceResult<NotificationWorkflow> {
        let workflow: Option<NotificationWorkflow> = sqlx::query_as(
            r#"
            SELECT * FROM notification_workflows
            WHERE workflow_key = $1 AND (tenant_id = $2 OR (tenant_id IS NULL AND $2 IS NULL))
            ORDER BY tenant_id NULLS LAST
            LIMIT 1
            "#,
        )
        .bind(workflow_key)
        .bind(tenant_id)
        .fetch_optional(&self.db)
        .await?;

        workflow.ok_or_else(|| {
            EmailServiceError::Configuration(format!("Workflow '{}' not found", workflow_key))
        })
    }

    /// Check if user has opted out of this workflow
    async fn check_user_preferences(
        &self,
        workflow_key: &str,
        user_id: Uuid,
    ) -> EmailServiceResult<()> {
        // Get or create user preferences
        let prefs: Option<UserNotificationPrefs> =
            sqlx::query_as("SELECT * FROM user_notification_prefs WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.db)
                .await?;

        if let Some(prefs) = prefs {
            // Check if email is enabled globally
            if !prefs.email_enabled {
                return Err(EmailServiceError::UserOptedOut(
                    "User has disabled all emails".to_string(),
                ));
            }

            // Check workflow-specific opt-ins
            let opt_in = match workflow_key {
                "announcements" => prefs.announcements_opt_in,
                "offline_messages" => prefs.offline_notifications_opt_in,
                "mention_notifications" => prefs.mention_notifications_opt_in,
                "weekly_digest" => prefs.digest_opt_in,
                _ => None, // System workflows don't have opt-in
            };

            if opt_in == Some(false) {
                return Err(EmailServiceError::UserOptedOut(format!(
                    "User has opted out of '{}'",
                    workflow_key
                )));
            }
        }

        Ok(())
    }

    /// Check if email is throttled
    async fn is_throttled(
        &self,
        throttle_key: &str,
        policy: &WorkflowPolicy,
    ) -> EmailServiceResult<bool> {
        let throttle_minutes = policy.throttle_minutes.unwrap_or(5);
        let since = Utc::now() - Duration::minutes(throttle_minutes as i64);

        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM email_outbox
            WHERE throttle_key = $1 AND created_at > $2
            "#,
        )
        .bind(throttle_key)
        .bind(since)
        .fetch_one(&self.db)
        .await?;

        Ok(count > 0)
    }

    /// Get template for workflow
    async fn get_template_for_workflow(
        &self,
        workflow: &NotificationWorkflow,
        locale: Option<&str>,
    ) -> EmailServiceResult<(EmailTemplateFamily, EmailTemplateVersion)> {
        let family_id = workflow.selected_template_family_id.ok_or_else(|| {
            EmailServiceError::Configuration(format!(
                "No template family configured for workflow '{}'",
                workflow.workflow_key
            ))
        })?;

        let family: EmailTemplateFamily =
            sqlx::query_as("SELECT * FROM email_template_families WHERE id = $1")
                .bind(family_id)
                .fetch_one(&self.db)
                .await?;

        // Determine locale with fallback
        let locale = locale.unwrap_or(&workflow.default_locale);
        let locales = vec![locale.to_string(), "en".to_string()];

        // Try to find published template for locale
        let mut template: Option<EmailTemplateVersion> = None;
        for loc in &locales {
            template = sqlx::query_as(
                r#"
                SELECT * FROM email_template_versions
                WHERE family_id = $1 AND locale = $2 AND status = 'published'
                ORDER BY version DESC
                LIMIT 1
                "#,
            )
            .bind(family_id)
            .bind(loc)
            .fetch_optional(&self.db)
            .await?;

            if template.is_some() {
                break;
            }
        }

        let template = template.ok_or_else(|| {
            EmailServiceError::Configuration(format!(
                "No published template found for family '{}'",
                family.key
            ))
        })?;

        Ok((family, template))
    }

    /// Calculate send_after time based on quiet hours
    async fn calculate_send_after(
        &self,
        user_id: Uuid,
        policy: &WorkflowPolicy,
    ) -> EmailServiceResult<Option<DateTime<Utc>>> {
        // Check if workflow respects quiet hours
        if !policy.respect_quiet_hours.unwrap_or(true) {
            return Ok(None);
        }

        // Get user's quiet hours
        let prefs: Option<UserNotificationPrefs> =
            sqlx::query_as("SELECT * FROM user_notification_prefs WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.db)
                .await?;

        let quiet_hours = prefs
            .and_then(|p| p.quiet_hours_json)
            .map(|j| j.0)
            .unwrap_or_default();

        if !quiet_hours.enabled {
            return Ok(None);
        }

        // Parse quiet hours (format: "HH:MM")
        let now = Utc::now();
        let current_time = now.time();
        let current_hour = current_time.hour() as i32;
        let current_minute = current_time.minute() as i32;

        let start_parts: Vec<i32> = quiet_hours
            .start
            .split(':')
            .filter_map(|s| s.parse().ok())
            .collect();
        let end_parts: Vec<i32> = quiet_hours
            .end
            .split(':')
            .filter_map(|s| s.parse().ok())
            .collect();

        if start_parts.len() != 2 || end_parts.len() != 2 {
            return Ok(None);
        }

        let start_hour = start_parts[0];
        let start_minute = start_parts[1];
        let end_hour = end_parts[0];
        let end_minute = end_parts[1];

        // Check if currently in quiet hours
        let current_minutes = current_hour * 60 + current_minute;
        let start_minutes = start_hour * 60 + start_minute;
        let end_minutes = end_hour * 60 + end_minute;

        let in_quiet_hours = if start_minutes < end_minutes {
            // Simple range (e.g., 22:00 - 08:00)
            current_minutes >= start_minutes && current_minutes < end_minutes
        } else {
            // Wraps around midnight (e.g., 22:00 - 08:00)
            current_minutes >= start_minutes || current_minutes < end_minutes
        };

        if in_quiet_hours {
            // Calculate when quiet hours end
            let send_after = if current_minutes >= start_minutes {
                // Same day end
                let minutes_until_end = (24 * 60 - current_minutes) + end_minutes;
                now + Duration::minutes(minutes_until_end as i64)
            } else {
                // Same day (before midnight)
                let minutes_until_end = end_minutes - current_minutes;
                now + Duration::minutes(minutes_until_end as i64)
            };
            return Ok(Some(send_after));
        }

        Ok(None)
    }

    /// Record an email event
    #[allow(clippy::too_many_arguments)]
    async fn record_event(
        &self,
        outbox_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        workflow_key: &str,
        event_type: EmailEventType,
        recipient_email: &str,
        recipient_user_id: Option<Uuid>,
        error_category: Option<&str>,
        error_message: Option<&str>,
    ) -> EmailServiceResult<()> {
        sqlx::query(
            r#"
            INSERT INTO email_events (
                outbox_id, tenant_id, workflow_key, event_type,
                recipient_email, recipient_user_id, error_category, error_message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(outbox_id)
        .bind(tenant_id)
        .bind(workflow_key)
        .bind(event_type.as_str())
        .bind(recipient_email)
        .bind(recipient_user_id)
        .bind(error_category)
        .bind(error_message)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get outbox entry by ID
    pub async fn get_outbox_entry(&self, id: Uuid) -> EmailServiceResult<Option<EmailOutbox>> {
        let entry: Option<EmailOutbox> = sqlx::query_as("SELECT * FROM email_outbox WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(entry)
    }

    /// List outbox entries with filters
    pub async fn list_outbox(
        &self,
        filters: OutboxFilters,
        limit: i64,
        offset: i64,
    ) -> EmailServiceResult<Vec<EmailOutbox>> {
        let entries: Vec<EmailOutbox> = sqlx::query_as(
            r#"
            SELECT * FROM email_outbox
            WHERE ($1::email_status IS NULL OR status = $1)
              AND ($2::varchar IS NULL OR workflow_key = $2)
              AND ($3::varchar IS NULL OR recipient_email = $3)
              AND ($4::uuid IS NULL OR recipient_user_id = $4)
            ORDER BY created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(filters.status)
        .bind(filters.workflow_key)
        .bind(filters.recipient_email)
        .bind(filters.recipient_user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(entries)
    }

    /// Get email events for an outbox entry
    pub async fn get_events(&self, outbox_id: Uuid) -> EmailServiceResult<Vec<EmailEvent>> {
        let events: Vec<EmailEvent> = sqlx::query_as(
            r#"
            SELECT * FROM email_events
            WHERE outbox_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(outbox_id)
        .fetch_all(&self.db)
        .await?;

        Ok(events)
    }

    /// Get or create user notification preferences
    pub async fn get_user_prefs(&self, user_id: Uuid) -> EmailServiceResult<UserNotificationPrefs> {
        let prefs: Option<UserNotificationPrefs> =
            sqlx::query_as("SELECT * FROM user_notification_prefs WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.db)
                .await?;

        if let Some(prefs) = prefs {
            return Ok(prefs);
        }

        // Create default preferences
        let new_prefs: UserNotificationPrefs = sqlx::query_as(
            r#"
            INSERT INTO user_notification_prefs (user_id)
            VALUES ($1)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(new_prefs)
    }

    /// Update user notification preferences
    pub async fn update_user_prefs(
        &self,
        user_id: Uuid,
        request: UpdateNotificationPrefsRequest,
    ) -> EmailServiceResult<UserNotificationPrefs> {
        // Ensure preferences exist first
        let _ = self.get_user_prefs(user_id).await?;

        let updated: UserNotificationPrefs = sqlx::query_as(
            r#"
            UPDATE user_notification_prefs SET
                email_enabled = COALESCE($2, email_enabled),
                announcements_opt_in = COALESCE($3, announcements_opt_in),
                offline_notifications_opt_in = COALESCE($4, offline_notifications_opt_in),
                mention_notifications_opt_in = COALESCE($5, mention_notifications_opt_in),
                digest_opt_in = COALESCE($6, digest_opt_in),
                quiet_hours_json = COALESCE($7, quiet_hours_json),
                locale = COALESCE($8, locale),
                offline_throttle_minutes = COALESCE($9, offline_throttle_minutes),
                include_message_content = COALESCE($10, include_message_content),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(request.email_enabled)
        .bind(request.announcements_opt_in)
        .bind(request.offline_notifications_opt_in)
        .bind(request.mention_notifications_opt_in)
        .bind(request.digest_opt_in)
        .bind(request.quiet_hours.map(sqlx::types::Json))
        .bind(request.locale)
        .bind(request.offline_throttle_minutes)
        .bind(request.include_message_content)
        .fetch_one(&self.db)
        .await?;

        Ok(updated)
    }
}

/// Options for enqueuing an email
#[derive(Debug, Clone, Default)]
pub struct EnqueueOptions {
    pub locale: Option<String>,
    pub priority: EmailPriority,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub headers: Option<serde_json::Value>,
    pub throttle_key: Option<String>,
    pub created_by: Option<Uuid>,
}

/// Filters for listing outbox entries
#[derive(Debug, Clone, Default)]
pub struct OutboxFilters {
    pub status: Option<EmailStatus>,
    pub workflow_key: Option<String>,
    pub recipient_email: Option<String>,
    pub recipient_user_id: Option<Uuid>,
}
