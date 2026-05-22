//! Admin Email API Endpoints
//!
//! Provides administrative endpoints for managing the email subsystem:
//! - Mail provider settings
//! - Notification workflows
//! - Email templates
//! - Outbox monitoring
//! - Email events/audit

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::api::{admin::require_admin, AppState};
use crate::error::{ApiResult, AppError};
use crate::models::email::*;
use crate::repositories::AdminRepository;
use crate::services::email_provider::{EmailAddress, EmailContent, MailProvider, SmtpProvider};
use crate::services::email_service::{EmailService, EnqueueOptions, OutboxFilters};
use crate::services::template_renderer::TemplateRenderer;

fn auth_org_id(auth: &crate::auth::AuthUser) -> ApiResult<Uuid> {
    auth.org_id
        .ok_or_else(|| AppError::Forbidden("Organization context is required".to_string()))
}

fn classify_email_error(error: &str) -> (&'static str, &'static str) {
    let lower = error.to_ascii_lowercase();
    if lower.contains("auth") || lower.contains("credential") || lower.contains("password") {
        ("authentication", "Check the SMTP username and password.")
    } else if lower.contains("tls") || lower.contains("certificate") || lower.contains("ssl") {
        ("tls", "Check TLS mode and certificate settings.")
    } else if lower.contains("timeout") || lower.contains("timed out") {
        (
            "timeout",
            "Check SMTP host, port, and network reachability.",
        )
    } else if lower.contains("connection") || lower.contains("connect") {
        ("connection", "Check SMTP host and port.")
    } else {
        ("smtp", "Check the provider logs for details.")
    }
}

async fn record_provider_email_test_event(
    state: &AppState,
    user_id: Uuid,
    provider: &MailProviderSettings,
    recipient: &str,
    success: bool,
    message: Option<String>,
    error_category: Option<&str>,
) {
    let event_type = if success { "sent" } else { "failed" };
    let provider_response = success.then(|| {
        sqlx::types::Json(serde_json::json!({
            "server_response": message.clone().unwrap_or_default()
        }))
    });

    let result = sqlx::query(
        r#"
        INSERT INTO email_events (
            tenant_id, workflow_key, event_type, recipient_email, recipient_user_id,
            provider_id, error_category, error_message, provider_response
        )
        VALUES ($1, 'provider_test', $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(provider.tenant_id)
    .bind(event_type)
    .bind(recipient)
    .bind(user_id)
    .bind(provider.id)
    .bind(error_category)
    .bind(if success { None } else { message.as_deref() })
    .bind(provider_response)
    .execute(&state.db)
    .await;

    if let Err(error) = result {
        tracing::warn!(%error, "failed to record provider email test event");
    }
}

/// Build admin email routes
pub fn router() -> Router<AppState> {
    Router::new()
        // Provider Settings
        .route(
            "/admin/email/providers",
            get(list_providers).post(create_provider),
        )
        .route(
            "/admin/email/providers/{id}",
            get(get_provider)
                .put(update_provider)
                .delete(delete_provider),
        )
        .route("/admin/email/providers/{id}/test", post(test_provider))
        .route(
            "/admin/email/providers/{id}/default",
            post(set_default_provider),
        )
        // Workflows
        .route("/admin/email/workflows", get(list_workflows))
        .route(
            "/admin/email/workflows/{id}",
            get(get_workflow).patch(update_workflow),
        )
        // Template Families
        .route(
            "/admin/email/template-families",
            get(list_template_families).post(create_template_family),
        )
        .route(
            "/admin/email/template-families/{id}",
            get(get_template_family)
                .patch(update_template_family)
                .delete(delete_template_family),
        )
        // Template Versions
        .route(
            "/admin/email/template-families/{id}/versions",
            get(list_template_versions).post(create_template_version),
        )
        .route(
            "/admin/email/template-versions/{version_id}",
            get(get_template_version).patch(update_template_version),
        )
        .route(
            "/admin/email/template-versions/{version_id}/publish",
            post(publish_template_version),
        )
        .route(
            "/admin/email/template-versions/{version_id}/preview",
            post(preview_template),
        )
        .route(
            "/admin/email/template-versions/{version_id}/send-preview",
            post(send_preview_email),
        )
        // Outbox
        .route("/admin/email/outbox", get(list_outbox))
        .route("/admin/email/outbox/{id}", get(get_outbox_entry))
        .route("/admin/email/outbox/{id}/cancel", post(cancel_outbox_entry))
        .route("/admin/email/outbox/{id}/retry", post(retry_outbox_entry))
        // Events
        .route("/admin/email/events", get(list_email_events))
        // Send test email
        .route("/admin/email/send-test", post(send_test_email))
        // User preferences (admin view)
        .route(
            "/admin/email/users/{user_id}/prefs",
            get(get_user_prefs).put(update_user_prefs),
        )
        .route("/admin/email/test", post(test_email_config))
}

// ============================================
// Provider Settings
// ============================================

#[derive(Debug, Deserialize)]
struct ListProvidersQuery {
    tenant_id: Option<Uuid>,
}

async fn list_providers(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Query(query): Query<ListProvidersQuery>,
) -> ApiResult<Json<Vec<MailProviderResponse>>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let providers = repo.list_mail_providers(query.tenant_id).await?;

    let responses: Vec<MailProviderResponse> = providers.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

async fn get_provider(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<MailProviderResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let provider = repo
        .get_mail_provider(id)
        .await?
        .ok_or_else(|| AppError::ProviderNotFound)?;

    Ok(Json(provider.into()))
}

async fn create_provider(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Json(body): Json<CreateMailProviderRequest>,
) -> ApiResult<Json<MailProviderResponse>> {
    require_admin(&auth)?;

    let provider_type = MailProviderType::from_str(&body.provider_type).ok_or_else(|| {
        AppError::Validation(format!("Invalid provider type: {}", body.provider_type))
    })?;

    let tls_mode = TlsMode::from_str(&body.tls_mode)
        .ok_or_else(|| AppError::Validation(format!("Invalid TLS mode: {}", body.tls_mode)))?;

    // Encrypt password
    let password_encrypted = if body.password.is_empty() {
        String::new()
    } else {
        crate::crypto::encrypt(&body.password, &state.config.encryption_key)?
    };

    let repo = AdminRepository::new(&state.db);

    // If this is set as default, clear other defaults
    if body.is_default {
        repo.clear_default_mail_providers(None).await?;
    }

    let provider = repo
        .create_mail_provider(
            auth.org_id,
            provider_type,
            &body.host,
            body.port,
            &body.username,
            &password_encrypted,
            tls_mode,
            body.skip_cert_verify,
            &body.from_address,
            &body.from_name,
            body.reply_to.as_deref(),
            body.max_emails_per_minute,
            body.max_emails_per_hour,
            body.enabled,
            body.is_default,
            Some(auth.user_id),
        )
        .await?;

    info!(
        "Created mail provider: id={}, host={}",
        provider.id, provider.host
    );
    Ok(Json(provider.into()))
}

async fn update_provider(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateMailProviderRequest>,
) -> ApiResult<Json<MailProviderResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    // Get existing provider
    let existing = repo
        .get_mail_provider(id)
        .await?
        .ok_or_else(|| AppError::ProviderNotFound)?;

    // Process password if provided
    let password_encrypted = if let Some(ref password) = body.password {
        if password.is_empty() {
            None
        } else {
            Some(crate::crypto::encrypt(
                password,
                &state.config.encryption_key,
            )?)
        }
    } else {
        None
    };

    // If setting as default, clear others
    if body.is_default == Some(true) && !existing.is_default {
        repo.clear_default_mail_providers(existing.tenant_id)
            .await?;
    }

    let provider = repo
        .update_mail_provider(
            id,
            body.provider_type
                .as_deref()
                .and_then(MailProviderType::from_str),
            body.host.as_deref(),
            body.port,
            body.username.as_deref(),
            password_encrypted.as_deref(),
            body.tls_mode.as_deref().and_then(TlsMode::from_str),
            body.skip_cert_verify,
            body.from_address.as_deref(),
            body.from_name.as_deref(),
            body.reply_to.as_deref(),
            body.max_emails_per_minute,
            body.max_emails_per_hour,
            body.enabled,
            body.is_default,
        )
        .await?;

    info!("Updated mail provider: id={}", id);
    Ok(Json(provider.into()))
}

async fn delete_provider(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let deleted = repo.delete_mail_provider(id).await?;

    if !deleted {
        return Err(AppError::ProviderNotFound);
    }

    info!("Deleted mail provider: id={}", id);
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn set_default_provider(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<MailProviderResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    // Get the provider
    let provider = repo
        .get_mail_provider(id)
        .await?
        .ok_or_else(|| AppError::ProviderNotFound)?;

    // Clear other defaults for this tenant
    repo.clear_default_mail_providers(provider.tenant_id)
        .await?;

    // Set this one as default
    let provider = repo.set_default_mail_provider(id).await?;

    Ok(Json(provider.into()))
}

#[derive(Debug, Deserialize)]
struct TestProviderRequest {
    to_email: String,
}

async fn test_provider(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<TestProviderRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    // Get provider settings
    let settings = repo
        .get_mail_provider(id)
        .await?
        .ok_or_else(|| AppError::ProviderNotFound)?;

    // Create provider and test connection
    let provider = SmtpProvider::new(settings.clone(), &state.config.encryption_key)
        .await
        .map_err(|e| AppError::ExternalService(format!("Failed to create provider: {}", e)))?;

    // Test connection first
    if let Err(e) = provider.test_connection().await {
        return Ok(Json(serde_json::json!({
            "success": false,
            "stage": "connection",
            "error": e.to_string()
        })));
    }

    // Send test email
    let from = EmailAddress::with_name(&settings.from_address, &settings.from_name);
    let to = EmailAddress::new(&body.to_email);
    let content = EmailContent {
        subject: "RustChat Email Test".to_string(),
        body_text: format!(
            "This is a test email from RustChat.\n\nProvider: {}:{}\nTLS: {}\nSent at: {}",
            settings.host,
            settings.port,
            settings.tls_mode.as_str(),
            Utc::now()
        ),
        body_html: None,
        headers: vec![],
    };

    match provider.send_email(&from, &to, &content).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "success": true,
            "stage": "sent",
            "message": format!("Test email sent to {}", body.to_email),
            "server_response": result.server_response
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "success": false,
            "stage": "sending",
            "error": e.to_string()
        }))),
    }
}

// ============================================
// Workflows
// ============================================

async fn list_workflows(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
) -> ApiResult<Json<Vec<WorkflowResponse>>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let workflows = repo
        .list_notification_workflows(auth_org_id(&auth)?)
        .await?;

    let responses: Vec<WorkflowResponse> = workflows.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

async fn get_workflow(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<WorkflowResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let workflow = repo
        .get_notification_workflow(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Workflow not found".to_string()))?;

    Ok(Json(workflow.into()))
}

async fn update_workflow(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateWorkflowRequest>,
) -> ApiResult<Json<WorkflowResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    // Get existing to check if system required
    let existing = repo
        .get_notification_workflow(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Workflow not found".to_string()))?;

    // Don't allow disabling system required workflows
    if let Some(false) = body.enabled {
        if existing.system_required {
            return Err(AppError::Forbidden(
                "Cannot disable system-required workflow".to_string(),
            ));
        }
    }

    let policy_json = body.policy.map(sqlx::types::Json);

    let workflow = repo
        .update_notification_workflow(
            id,
            body.enabled,
            body.default_locale.as_deref(),
            body.selected_template_family_id,
            policy_json,
        )
        .await?;

    Ok(Json(workflow.into()))
}

// ============================================
// Template Families
// ============================================

async fn list_template_families(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
) -> ApiResult<Json<Vec<EmailTemplateFamily>>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let families = repo
        .list_email_template_families(auth_org_id(&auth)?)
        .await?;

    Ok(Json(families))
}

async fn get_template_family(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<EmailTemplateFamily>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let family = repo
        .get_email_template_family(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Template family not found".to_string()))?;

    Ok(Json(family))
}

async fn create_template_family(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Json(body): Json<CreateTemplateFamilyRequest>,
) -> ApiResult<Json<EmailTemplateFamily>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let family = repo
        .create_email_template_family(
            auth.org_id,
            &body.key,
            &body.name,
            body.description.as_deref(),
            body.workflow_key.as_deref(),
            Some(auth.user_id),
        )
        .await?;

    info!(
        "Created template family: id={}, key={}",
        family.id, family.key
    );
    Ok(Json(family))
}

async fn update_template_family(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTemplateFamilyRequest>,
) -> ApiResult<Json<EmailTemplateFamily>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let family = repo
        .update_email_template_family(id, body.name.as_deref(), body.description.as_deref())
        .await?
        .ok_or_else(|| AppError::NotFound("Template family not found or is system".to_string()))?;

    Ok(Json(family))
}

async fn delete_template_family(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let deleted = repo.delete_email_template_family(id).await?;

    if !deleted {
        return Err(AppError::NotFound(
            "Template family not found or is system".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============================================
// Template Versions
// ============================================

async fn list_template_versions(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(family_id): Path<Uuid>,
) -> ApiResult<Json<Vec<TemplateVersionResponse>>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let versions = repo.list_email_template_versions(family_id).await?;

    let responses: Vec<TemplateVersionResponse> = versions.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

async fn get_template_version(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(version_id): Path<Uuid>,
) -> ApiResult<Json<TemplateVersionResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let version = repo
        .get_email_template_version(version_id)
        .await?
        .ok_or_else(|| AppError::TemplateVersionNotFound)?;

    Ok(Json(version.into()))
}

async fn create_template_version(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(family_id): Path<Uuid>,
    Json(body): Json<CreateTemplateVersionRequest>,
) -> ApiResult<Json<TemplateVersionResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let new_version = repo
        .create_email_template_version(
            family_id,
            &body.locale,
            &body.subject,
            &body.body_text,
            &body.body_html,
            body.variables,
            body.is_compiled_from_mjml,
            body.mjml_source.as_deref(),
            Some(auth.user_id),
        )
        .await?;

    Ok(Json(new_version.into()))
}

async fn update_template_version(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(version_id): Path<Uuid>,
    Json(body): Json<UpdateTemplateVersionRequest>,
) -> ApiResult<Json<TemplateVersionResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    // Can only update draft versions
    let existing = repo
        .get_email_template_version(version_id)
        .await?
        .ok_or_else(|| AppError::TemplateVersionNotFound)?;

    if existing.status != TemplateStatus::Draft {
        return Err(AppError::Forbidden(
            "Cannot edit published or archived versions".to_string(),
        ));
    }

    let variables_json = body.variables.map(sqlx::types::Json);

    let version = repo
        .update_email_template_version(
            version_id,
            body.subject.as_deref(),
            body.body_text.as_deref(),
            body.body_html.as_deref(),
            variables_json,
            body.mjml_source.as_deref(),
        )
        .await?;

    Ok(Json(version.into()))
}

async fn publish_template_version(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(version_id): Path<Uuid>,
) -> ApiResult<Json<TemplateVersionResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let version = repo
        .publish_email_template_version(version_id, auth.user_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Template version not found or not in draft status".to_string())
        })?;

    info!(
        "Published template version: id={}, family_id={}, version={}",
        version_id, version.family_id, version.version
    );

    Ok(Json(version.into()))
}

#[derive(Debug, Deserialize)]
struct PreviewTemplateRequest {
    sample_data: Option<serde_json::Value>,
}

async fn preview_template(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(version_id): Path<Uuid>,
    Json(body): Json<PreviewTemplateRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let version = repo
        .get_email_template_version(version_id)
        .await?
        .ok_or_else(|| AppError::TemplateVersionNotFound)?;

    let renderer = TemplateRenderer::new();

    // Use provided sample data or build from schema
    let sample_data = body.sample_data.unwrap_or_else(|| {
        TemplateRenderer::build_sample_payload(&version.variables_schema_json.0)
    });

    match renderer.preview_template(&version, &sample_data) {
        Ok(rendered) => Ok(Json(serde_json::json!({
            "subject": rendered.subject,
            "body_text": rendered.body_text,
            "body_html": rendered.body_html,
            "sample_data_used": sample_data,
        }))),
        Err(e) => Err(AppError::BadRequest(format!(
            "Template render error: {}",
            e
        ))),
    }
}

#[derive(Debug, Deserialize)]
struct SendPreviewRequest {
    to_email: String,
    sample_data: Option<serde_json::Value>,
}

async fn send_preview_email(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(version_id): Path<Uuid>,
    Json(body): Json<SendPreviewRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    // Get default provider
    let provider_settings = repo
        .get_default_mail_provider()
        .await?
        .ok_or_else(|| AppError::Config("No default mail provider configured".to_string()))?;

    // Get template
    let version = repo
        .get_email_template_version(version_id)
        .await?
        .ok_or_else(|| AppError::TemplateVersionNotFound)?;

    // Render
    let renderer = TemplateRenderer::new();
    let sample_data = body.sample_data.unwrap_or_else(|| {
        TemplateRenderer::build_sample_payload(&version.variables_schema_json.0)
    });

    let rendered = renderer
        .preview_template(&version, &sample_data)
        .map_err(|e| AppError::BadRequest(format!("Template render error: {}", e)))?;

    // Send via provider
    let provider = SmtpProvider::new(provider_settings.clone(), &state.config.encryption_key)
        .await
        .map_err(|e| AppError::ExternalService(format!("Provider error: {}", e)))?;

    let from = EmailAddress::with_name(
        &provider_settings.from_address,
        &provider_settings.from_name,
    );
    let to = EmailAddress::new(&body.to_email);
    let content = EmailContent {
        subject: format!("[PREVIEW] {}", rendered.subject),
        body_text: rendered.body_text,
        body_html: rendered.body_html,
        headers: vec![("X-RustChat-Preview".to_string(), "true".to_string())],
    };

    match provider.send_email(&from, &to, &content).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Preview email sent to {}", body.to_email)
        }))),
        Err(e) => Err(AppError::ExternalService(format!("Failed to send: {}", e))),
    }
}

// ============================================
// Outbox
// ============================================

#[derive(Debug, Deserialize)]
struct ListOutboxQuery {
    status: Option<EmailStatus>,
    workflow_key: Option<String>,
    recipient_email: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn list_outbox(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Query(query): Query<ListOutboxQuery>,
) -> ApiResult<Json<Vec<EmailOutboxResponse>>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(200);
    let offset = (page - 1) * per_page;

    let service = EmailService::new(state.db.clone());
    let filters = OutboxFilters {
        status: query.status,
        workflow_key: query.workflow_key,
        recipient_email: query.recipient_email,
        recipient_user_id: None,
    };

    let entries = service
        .list_outbox(filters, per_page, offset)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let responses: Vec<EmailOutboxResponse> = entries.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

async fn get_outbox_entry(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<EmailOutbox>> {
    require_admin(&auth)?;

    let service = EmailService::new(state.db.clone());
    let entry = service
        .get_outbox_entry(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Outbox entry not found".to_string()))?;

    Ok(Json(entry))
}

async fn cancel_outbox_entry(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let cancelled = repo.cancel_outbox_entry(id).await?;

    if !cancelled {
        return Err(AppError::Conflict(
            "Email cannot be cancelled (may already be sent or failed)".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({"status": "cancelled"})))
}

async fn retry_outbox_entry(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let retried = repo.retry_outbox_entry(id).await?;

    if !retried {
        return Err(AppError::Conflict(
            "Email cannot be retried (may not be in failed status)".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({"status": "queued_for_retry"})))
}

// ============================================
// Email Events
// ============================================

#[derive(Debug, Deserialize)]
struct ListEventsQuery {
    outbox_id: Option<Uuid>,
    workflow_key: Option<String>,
    event_type: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn list_email_events(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Query(query): Query<ListEventsQuery>,
) -> ApiResult<Json<Vec<EmailEventResponse>>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(200);
    let offset = (page - 1) * per_page;

    let repo = AdminRepository::new(&state.db);
    let events = repo
        .list_email_events(
            query.outbox_id,
            query.workflow_key.as_deref(),
            query.event_type.as_deref(),
            per_page,
            offset,
        )
        .await?;

    let responses: Vec<EmailEventResponse> = events.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

// ============================================
// Send Test Email
// ============================================

#[derive(Debug, Deserialize)]
struct SendTestEmailRequest {
    provider_id: Option<Uuid>,
    to_email: String,
    workflow_key: Option<String>,
    locale: Option<String>,
    subject: Option<String>,
    body_text: Option<String>,
}

async fn send_test_email(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Json(body): Json<SendTestEmailRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let service = EmailService::new(state.db.clone());

    // If workflow and template specified, use template rendering
    if let Some(workflow_key) = body.workflow_key {
        let options = EnqueueOptions {
            locale: body.locale,
            priority: EmailPriority::High,
            created_by: Some(auth.user_id),
            ..Default::default()
        };

        let payload = serde_json::json!({
            "user_name": "Test User",
            "email": body.to_email,
            "site_name": "RustChat",
            "verification_link": "https://example.com/verify?token=test",
            "reset_link": "https://example.com/reset?token=test",
            "channel_name": "general",
            "message_count": 5,
        });

        let outbox_id = service
            .enqueue_email(
                &workflow_key,
                &body.to_email,
                None, // No user_id for test
                payload,
                options,
            )
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        return Ok(Json(serde_json::json!({
            "success": true,
            "outbox_id": outbox_id,
            "message": format!("Test email enqueued: {}", outbox_id)
        })));
    }

    // Otherwise, send simple test via provider
    let repo = AdminRepository::new(&state.db);
    let provider_settings = if let Some(id) = body.provider_id {
        repo.get_mail_provider(id).await?
    } else {
        repo.get_default_mail_provider().await?
    };

    let settings =
        provider_settings.ok_or_else(|| AppError::Config("No mail provider found".to_string()))?;

    let provider = SmtpProvider::new(settings.clone(), &state.config.encryption_key)
        .await
        .map_err(|e| AppError::ExternalService(format!("Provider error: {}", e)))?;

    let from = EmailAddress::with_name(&settings.from_address, &settings.from_name);
    let to = EmailAddress::new(&body.to_email);
    let content = EmailContent {
        subject: body
            .subject
            .unwrap_or_else(|| "RustChat Test Email".to_string()),
        body_text: body
            .body_text
            .unwrap_or_else(|| "This is a test email from RustChat.".to_string()),
        body_html: None,
        headers: vec![],
    };

    match provider.send_email(&from, &to, &content).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "success": true,
            "server_response": result.server_response,
            "message": format!("Test email sent to {}", body.to_email)
        }))),
        Err(e) => Err(AppError::ExternalService(format!("Failed to send: {}", e))),
    }
}

// ============================================
// User Preferences (Admin)
// ============================================

async fn get_user_prefs(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<UserNotificationPrefsResponse>> {
    require_admin(&auth)?;

    let service = EmailService::new(state.db.clone());
    let prefs = service
        .get_user_prefs(user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(prefs.into()))
}

async fn update_user_prefs(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateNotificationPrefsRequest>,
) -> ApiResult<Json<UserNotificationPrefsResponse>> {
    require_admin(&auth)?;

    let service = EmailService::new(state.db.clone());
    let prefs = service
        .update_user_prefs(user_id, body)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(prefs.into()))
}
// ============ Email Testing ============

#[derive(Debug, serde::Deserialize)]
pub struct TestEmailRequest {
    /// Email address to send test to (defaults to admin's email)
    pub email: Option<String>,
    /// Alternative field name used by frontend
    #[serde(rename = "to")]
    pub to_email: Option<String>,
}

pub async fn test_email_config(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Json(payload): Json<TestEmailRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    // Get default provider from the new provider system
    let provider_settings = repo.get_default_mail_provider().await?.ok_or_else(|| {
        AppError::Config(
            "No default mail provider configured. Please configure an email provider first."
                .to_string(),
        )
    })?;

    // Check if SMTP is configured
    if provider_settings.host.trim().is_empty() {
        return Err(AppError::BadRequest(
            "SMTP host is not configured in the default provider".to_string(),
        ));
    }

    if provider_settings.from_address.trim().is_empty() {
        return Err(AppError::BadRequest(
            "From address is not configured in the default provider".to_string(),
        ));
    }

    if payload.to_email.is_none() && payload.email.is_none() && auth.email.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Test recipient email is required".to_string(),
        ));
    }

    // Determine test recipient (use 'to' field or 'email' field, fallback to admin's email)
    let test_email = payload
        .to_email
        .or(payload.email)
        .unwrap_or_else(|| auth.email.clone());

    // Create provider and test
    let provider = SmtpProvider::new(provider_settings.clone(), &state.config.encryption_key)
        .await
        .map_err(|e| AppError::Config(format!("Failed to create SMTP provider: {}", e)))?;

    // Test connection first
    if let Err(e) = provider.test_connection().await {
        let error_msg = e.to_string();
        let (kind, hint) = classify_email_error(&error_msg);
        record_provider_email_test_event(
            &state,
            auth.user_id,
            &provider_settings,
            &test_email,
            false,
            Some(error_msg.clone()),
            Some(kind),
        )
        .await;
        return Err(AppError::ExternalService(format!(
            "SMTP connection failed ({}): {}. {}",
            kind, error_msg, hint
        )));
    }

    tracing::info!("SMTP connection test successful");

    // Send test email
    let from = EmailAddress::with_name(
        &provider_settings.from_address,
        &provider_settings.from_name,
    );
    let to = EmailAddress::new(&test_email);
    let content = EmailContent {
        subject: "RustChat Test Email".to_string(),
        body_text: format!(
            "This is a test email from RustChat.\n\nIf you received this, your email configuration is working correctly!\n\nConfiguration used:\n- SMTP Server: {}:{}\n- TLS: {}\n- From: {}\n",
            provider_settings.host,
            provider_settings.port,
            provider_settings.tls_mode.as_str(),
            provider_settings.from_address
        ),
        body_html: None,
        headers: vec![],
    };

    match provider.send_email(&from, &to, &content).await {
        Ok(result) => {
            record_provider_email_test_event(
                &state,
                auth.user_id,
                &provider_settings,
                &test_email,
                true,
                Some(result.server_response.clone()),
                None,
            )
            .await;

            Ok(Json(serde_json::json!({
                "status": "success",
                "message": format!("Test email sent successfully to {}", test_email),
                "delivery": {
                    "accepted": true,
                    "message_id": result.message_id,
                    "server_response": result.server_response,
                },
                "config": {
                    "smtp_host": provider_settings.host,
                    "smtp_port": provider_settings.port,
                    "smtp_security": provider_settings.tls_mode.as_str(),
                    "from_address": provider_settings.from_address,
                    "from_name": provider_settings.from_name,
                    "reply_to": provider_settings.reply_to.as_deref().unwrap_or(""),
                }
            })))
        }
        Err(e) => {
            let error_msg = e.to_string();
            let (kind, hint) = classify_email_error(&error_msg);
            record_provider_email_test_event(
                &state,
                auth.user_id,
                &provider_settings,
                &test_email,
                false,
                Some(error_msg.clone()),
                Some(kind),
            )
            .await;
            Err(AppError::ExternalService(format!(
                "Test email send failed ({}): {}. {}",
                kind, error_msg, hint
            )))
        }
    }
}
