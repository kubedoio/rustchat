//! Admin API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::auth::policy::{permissions as policy_permissions, AuthzResult, PolicyEngine};
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::middleware::reliability::{send_reqwest_with_retry, RetryCondition, RetryConfig};
use crate::models::{
    normalize_avatar_url, AddTeamMember, AuditLog, AuditLogQuery, CreateChannel,
    CreateRetentionPolicy, CreateSsoConfig, Permission, RetentionPolicy, ServerConfig,
    ServerConfigResponse, SsoConfig, SsoConfigResponse, SsoProviderType, SsoTestResult, TeamMember,
    TeamMemberResponse, UpdateChannel, UpdateSsoConfig,
};
use crate::services::email_provider::{EmailAddress, EmailContent, MailProvider, SmtpProvider};
use crate::services::membership_policies::apply_auto_membership_for_new_user;
use crate::services::oidc_discovery::OidcDiscoveryService;
use crate::services::team_membership::apply_default_channel_membership_for_team_join;
use sqlx::FromRow;
use std::time::Duration;

/// Build admin routes
pub fn router() -> Router<AppState> {
    // Email routes
    let email_routes = super::admin_email::router();

    Router::new()
        // Merge email routes
        .merge(email_routes)
        // Server config
        .route("/admin/config", get(get_config))
        .route("/admin/config/{category}", patch(update_config))
        // Audit logs
        .route("/admin/audit", get(list_audit_logs))
        // SSO - new endpoints
        .route("/admin/sso", get(list_sso_configs).post(create_sso_config))
        .route(
            "/admin/sso/{id}",
            get(get_sso_config)
                .put(update_sso_config)
                .delete(delete_sso_config),
        )
        .route("/admin/sso/{id}/test", post(test_sso_config))
        // Retention
        .route(
            "/admin/retention",
            get(list_retention_policies).post(create_retention_policy),
        )
        .route(
            "/admin/retention/{id}",
            get(get_retention_policy).delete(delete_retention_policy),
        )
        // Permissions
        .route("/admin/permissions", get(list_permissions))
        .route(
            "/admin/roles/{role}/permissions",
            get(get_role_permissions).put(update_role_permissions),
        )
        // Users management
        .route("/admin/users", get(list_users).post(create_admin_user))
        .route(
            "/admin/users/{id}",
            patch(update_admin_user).delete(delete_admin_user),
        )
        .route(
            "/admin/users/{id}/deactivate",
            axum::routing::post(deactivate_user),
        )
        .route(
            "/admin/users/{id}/reactivate",
            axum::routing::post(reactivate_user),
        )
        .route("/admin/users/{id}/wipe", axum::routing::post(wipe_user))
        // Teams & Channels management
        .route("/admin/teams", get(list_admin_teams))
        .route(
            "/admin/teams/{id}",
            get(get_admin_team)
                .put(update_admin_team)
                .delete(delete_admin_team),
        )
        .route(
            "/admin/teams/{id}/members",
            get(list_team_members).post(add_team_member),
        )
        .route(
            "/admin/teams/{id}/members/{user_id}",
            axum::routing::delete(remove_team_member),
        )
        .route(
            "/admin/channels",
            get(list_admin_channels).post(create_admin_channel),
        )
        .route(
            "/admin/channels/{id}",
            patch(update_admin_channel).delete(delete_admin_channel),
        )
        // Groups management (for membership policies)
        .route("/admin/groups", get(list_admin_groups))
        // Stats & Health
        .route("/admin/stats", get(get_stats))
        .route("/admin/health", get(get_health))
        // Plugins - RustChat Calls Plugin
        .route(
            "/admin/plugins/calls",
            get(get_calls_plugin_config).put(update_calls_plugin_config),
        )
        // Email testing
        .route("/admin/email/test", post(test_email_config))
        // Membership policies
        .merge(super::admin_membership_policies::router())
        // Audit endpoints
        .merge(super::admin_audit::router())
}

/// Check if user is admin
pub fn require_admin(auth: &AuthUser) -> ApiResult<()> {
    match PolicyEngine::check_permission(&auth.role, &policy_permissions::SYSTEM_MANAGE) {
        AuthzResult::Allow => Ok(()),
        AuthzResult::Deny(_) => Err(AppError::Forbidden("Admin access required".to_string())),
    }
}

pub fn require_global_admin(auth: &AuthUser) -> ApiResult<()> {
    match PolicyEngine::check_permission(&auth.role, &policy_permissions::ADMIN_FULL) {
        AuthzResult::Allow => Ok(()),
        AuthzResult::Deny(_) => Err(AppError::Forbidden(
            "Global admin access required".to_string(),
        )),
    }
}

async fn insert_admin_audit_log(
    db: &sqlx::PgPool,
    actor_user_id: uuid::Uuid,
    action: &str,
    target_type: &str,
    target_id: Option<uuid::Uuid>,
    metadata: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, target_type, target_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(metadata)
    .execute(db)
    .await?;

    Ok(())
}

// ============ Server Configuration ============

async fn get_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<ServerConfigResponse>> {
    require_admin(&auth)?;

    let config: ServerConfig = sqlx::query_as("SELECT * FROM server_config WHERE id = 'default'")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(config.into()))
}

async fn update_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let column = match category.as_str() {
        "site" => "site",
        "authentication" => "authentication",
        "integrations" => "integrations",
        "compliance" => "compliance",
        "experimental" => "experimental",
        _ => {
            return Err(AppError::BadRequest(format!(
                "Invalid config category: {}",
                category
            )))
        }
    };

    let query = format!(
        "UPDATE server_config SET {} = $1, updated_at = NOW(), updated_by = $2 WHERE id = 'default' RETURNING {}",
        column, column
    );

    let result: (sqlx::types::Json<serde_json::Value>,) = sqlx::query_as(&query)
        .bind(sqlx::types::Json(&body))
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    // Broadcast config update to all connected users
    let event = crate::realtime::events::WsEnvelope::event(
        crate::realtime::events::EventType::ConfigUpdated,
        serde_json::json!({
            "category": category,
            "config": result.0.0
        }),
        None,
    );
    state.ws_hub.broadcast(event).await;

    Ok(Json(result.0 .0))
}

// ============ Audit Logs ============

async fn list_audit_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Json<Vec<AuditLog>>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(200);
    let offset = (page - 1) * per_page;

    let logs: Vec<AuditLog> = sqlx::query_as(
        r#"
        SELECT * FROM audit_logs
        WHERE ($1::VARCHAR IS NULL OR action = $1)
          AND ($2::VARCHAR IS NULL OR target_type = $2)
          AND ($3::UUID IS NULL OR actor_user_id = $3)
          AND ($4::TIMESTAMPTZ IS NULL OR created_at >= $4)
          AND ($5::TIMESTAMPTZ IS NULL OR created_at <= $5)
        ORDER BY created_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(&query.action)
    .bind(&query.target_type)
    .bind(query.actor_user_id)
    .bind(query.from_date)
    .bind(query.to_date)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(logs))
}

// ============ SSO Configuration ============

/// List all SSO configurations
async fn list_sso_configs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<SsoConfigResponse>>> {
    require_admin(&auth)?;

    let configs: Vec<SsoConfig> = sqlx::query_as(
        r#"
        SELECT 
            id, org_id, provider, provider_key, provider_type, display_name,
            issuer_url, client_id, client_secret_encrypted, scopes,
            idp_metadata_url, idp_entity_id, is_active, auto_provision,
            default_role, allow_domains, github_org, github_team,
            groups_claim, role_mappings, created_at, updated_at
        FROM sso_configs
        ORDER BY provider_key
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let responses: Vec<SsoConfigResponse> = configs.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

/// Get a single SSO configuration
async fn get_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SsoConfigResponse>> {
    require_admin(&auth)?;

    let config: SsoConfig = sqlx::query_as(
        r#"
        SELECT 
            id, org_id, provider, provider_key, provider_type, display_name,
            issuer_url, client_id, client_secret_encrypted, scopes,
            idp_metadata_url, idp_entity_id, is_active, auto_provision,
            default_role, allow_domains, github_org, github_team,
            groups_claim, role_mappings, created_at, updated_at
        FROM sso_configs WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("SSO configuration not found".to_string()))?;

    Ok(Json(config.into()))
}

/// Validate provider key (URL-safe: a-z, 0-9, -)
fn validate_provider_key(key: &str) -> ApiResult<()> {
    if key.is_empty() || key.len() > 64 {
        return Err(AppError::Validation(
            "Provider key must be 1-64 characters".to_string(),
        ));
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AppError::Validation(
            "Provider key must be lowercase alphanumeric with hyphens only".to_string(),
        ));
    }
    Ok(())
}

/// Validate SSO configuration input
fn validate_sso_config(input: &CreateSsoConfig, is_update: bool) -> ApiResult<SsoProviderType> {
    let provider_type = SsoProviderType::from_str(&input.provider_type).ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid provider_type '{}'. Must be one of: github, google, oidc",
            input.provider_type
        ))
    })?;

    // Validate provider_key for new configs
    if !is_update {
        validate_provider_key(&input.provider_key)?;
    }

    // Validate required fields based on provider type
    match provider_type {
        SsoProviderType::GitHub => {
            if input.client_id.as_ref().is_none_or(|s| s.is_empty()) {
                return Err(AppError::Validation(
                    "GitHub requires client_id".to_string(),
                ));
            }
            if input.client_secret.as_ref().is_none_or(|s| s.is_empty()) {
                return Err(AppError::Validation(
                    "GitHub requires client_secret".to_string(),
                ));
            }
        }
        SsoProviderType::Google | SsoProviderType::Oidc => {
            if input.issuer_url.as_ref().is_none_or(|s| s.is_empty()) {
                return Err(AppError::Validation(format!(
                    "{} requires issuer_url",
                    provider_type.as_str()
                )));
            }
            if input.client_id.as_ref().is_none_or(|s| s.is_empty()) {
                return Err(AppError::Validation(format!(
                    "{} requires client_id",
                    provider_type.as_str()
                )));
            }
            if input.client_secret.as_ref().is_none_or(|s| s.is_empty()) {
                return Err(AppError::Validation(format!(
                    "{} requires client_secret",
                    provider_type.as_str()
                )));
            }
            // Ensure scopes include 'openid' for OIDC
            let scopes = input.scopes.as_ref();
            let has_openid = scopes.is_none_or(|s| s.iter().any(|scope| scope == "openid"));
            if !has_openid {
                return Err(AppError::Validation(
                    "OIDC providers require 'openid' in scopes".to_string(),
                ));
            }
        }
        SsoProviderType::Saml => {
            return Err(AppError::Validation(
                "SAML is not supported via this API".to_string(),
            ));
        }
    }

    Ok(provider_type)
}

/// Create a new SSO configuration
async fn create_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateSsoConfig>,
) -> ApiResult<Json<SsoConfigResponse>> {
    require_admin(&auth)?;

    // For multi-tenant deployments, require org_id. For single-tenant (RustChat), org_id is optional.
    let org_id = auth.org_id;

    // Validate input
    let provider_type = validate_sso_config(&input, false)?;

    // Check for duplicate provider_key
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM sso_configs WHERE provider_key = $1")
            .bind(&input.provider_key)
            .fetch_optional(&state.db)
            .await?;

    if existing.is_some() {
        return Err(AppError::Validation(format!(
            "Provider key '{}' already exists",
            input.provider_key
        )));
    }

    // Encrypt client secret
    let encrypted_secret = input
        .client_secret
        .as_ref()
        .map(|s| crate::crypto::encrypt(s, &state.config.encryption_key))
        .transpose()?;

    // Use default scopes if not provided
    let scopes = input
        .scopes
        .clone()
        .unwrap_or_else(|| provider_type.default_scopes());

    let config: SsoConfig = sqlx::query_as(
        r#"
        INSERT INTO sso_configs (
            org_id, provider, provider_key, provider_type, display_name,
            issuer_url, client_id, client_secret_encrypted, scopes,
            is_active, auto_provision, default_role,
            allow_domains, github_org, github_team,
            groups_claim, role_mappings
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        RETURNING 
            id, org_id, provider, provider_key, provider_type, display_name,
            issuer_url, client_id, client_secret_encrypted, scopes,
            idp_metadata_url, idp_entity_id, is_active, auto_provision,
            default_role, allow_domains, github_org, github_team,
            groups_claim, role_mappings, created_at, updated_at
        "#,
    )
    .bind(org_id)
    .bind(&input.provider_type) // legacy provider field
    .bind(&input.provider_key)
    .bind(&input.provider_type)
    .bind(&input.display_name)
    .bind(&input.issuer_url)
    .bind(&input.client_id)
    .bind(&encrypted_secret)
    .bind(&scopes)
    .bind(input.is_active.unwrap_or(true))
    .bind(input.auto_provision.unwrap_or(true))
    .bind(input.default_role.as_ref().unwrap_or(&"member".to_string()))
    .bind(&input.allow_domains)
    .bind(&input.github_org)
    .bind(&input.github_team)
    .bind(&input.groups_claim)
    .bind(&input.role_mappings)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::Validation(format!(
                "Provider key '{}' already exists",
                input.provider_key
            ))
        } else {
            AppError::Internal(format!("Failed to create SSO config: {}", e))
        }
    })?;

    Ok(Json(config.into()))
}

/// Update an existing SSO configuration
async fn update_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateSsoConfig>,
) -> ApiResult<Json<SsoConfigResponse>> {
    require_admin(&auth)?;

    // Get existing config
    let existing: SsoConfig = sqlx::query_as(
        r#"
        SELECT 
            id, org_id, provider, provider_key, provider_type, display_name,
            issuer_url, client_id, client_secret_encrypted, scopes,
            idp_metadata_url, idp_entity_id, is_active, auto_provision,
            default_role, allow_domains, github_org, github_team,
            groups_claim, role_mappings, created_at, updated_at
        FROM sso_configs WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("SSO configuration not found".to_string()))?;

    // Validate new provider_key if changing
    if let Some(ref new_key) = input.provider_key {
        if new_key != &existing.provider_key {
            validate_provider_key(new_key)?;
            // Check for duplicates
            let dup: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM sso_configs WHERE provider_key = $1 AND id != $2")
                    .bind(new_key)
                    .bind(id)
                    .fetch_optional(&state.db)
                    .await?;
            if dup.is_some() {
                return Err(AppError::Validation(format!(
                    "Provider key '{}' already exists",
                    new_key
                )));
            }
        }
    }

    // Encrypt new client secret if provided
    let encrypted_secret = input
        .client_secret
        .as_ref()
        .map(|s| crate::crypto::encrypt(s, &state.config.encryption_key))
        .transpose()?;

    let config: SsoConfig = sqlx::query_as(
        r#"
        UPDATE sso_configs SET
            provider_key = COALESCE($1, provider_key),
            display_name = COALESCE($2, display_name),
            issuer_url = COALESCE($3, issuer_url),
            client_id = COALESCE($4, client_id),
            client_secret_encrypted = COALESCE($5, client_secret_encrypted),
            scopes = COALESCE($6, scopes),
            is_active = COALESCE($7, is_active),
            auto_provision = COALESCE($8, auto_provision),
            default_role = COALESCE($9, default_role),
            allow_domains = COALESCE($10, allow_domains),
            github_org = COALESCE($11, github_org),
            github_team = COALESCE($12, github_team),
            groups_claim = COALESCE($13, groups_claim),
            role_mappings = COALESCE($14, role_mappings),
            updated_at = NOW()
        WHERE id = $15
        RETURNING 
            id, org_id, provider, provider_key, provider_type, display_name,
            issuer_url, client_id, client_secret_encrypted, scopes,
            idp_metadata_url, idp_entity_id, is_active, auto_provision,
            default_role, allow_domains, github_org, github_team,
            groups_claim, role_mappings, created_at, updated_at
        "#,
    )
    .bind(&input.provider_key)
    .bind(&input.display_name)
    .bind(&input.issuer_url)
    .bind(&input.client_id)
    .bind(&encrypted_secret)
    .bind(&input.scopes)
    .bind(input.is_active)
    .bind(input.auto_provision)
    .bind(&input.default_role)
    .bind(&input.allow_domains)
    .bind(&input.github_org)
    .bind(&input.github_team)
    .bind(&input.groups_claim)
    .bind(&input.role_mappings)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update SSO config: {}", e)))?;

    Ok(Json(config.into()))
}

/// Delete an SSO configuration
async fn delete_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let result = sqlx::query("DELETE FROM sso_configs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "SSO configuration not found".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// Test an SSO configuration
async fn test_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SsoTestResult>> {
    require_admin(&auth)?;

    let config: SsoConfig = sqlx::query_as(
        r#"
        SELECT 
            id, org_id, provider, provider_key, provider_type, display_name,
            issuer_url, client_id, client_secret_encrypted, scopes,
            idp_metadata_url, idp_entity_id, is_active, auto_provision,
            default_role, allow_domains, github_org, github_team,
            groups_claim, role_mappings, created_at, updated_at
        FROM sso_configs WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("SSO configuration not found".to_string()))?;

    let provider_type = match SsoProviderType::from_str(&config.provider_type) {
        Some(t) => t,
        None => {
            return Ok(Json(SsoTestResult {
                success: false,
                message: format!("Unknown provider type: {}", config.provider_type),
                details: None,
            }));
        }
    };

    // Test based on provider type
    match provider_type {
        SsoProviderType::GitHub => test_github_config(&config).await,
        SsoProviderType::Google | SsoProviderType::Oidc => test_oidc_config(&config).await,
        SsoProviderType::Saml => Ok(Json(SsoTestResult {
            success: false,
            message: "SAML testing is not supported".to_string(),
            details: None,
        })),
    }
}

/// Test GitHub OAuth configuration
async fn test_github_config(config: &SsoConfig) -> ApiResult<Json<SsoTestResult>> {
    let client = reqwest::Client::new();
    let retry_config = RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(150),
        max_delay: Duration::from_secs(2),
        backoff_multiplier: 2.0,
        retry_if: RetryCondition::Default,
    };

    // Test that we can reach GitHub's token endpoint
    // We can't actually test authentication without a valid code,
    // but we can verify the endpoint is reachable
    let response = send_reqwest_with_retry(
        client
            .get("https://api.github.com")
            .header("User-Agent", "RustChat-SSO-Test"),
        &retry_config,
        |e| AppError::ExternalService(format!("Failed to reach GitHub API: {}", e)),
        || {
            AppError::Internal(
                "Failed to reach GitHub API: request could not be cloned for retry".to_string(),
            )
        },
    )
    .await;

    match response {
        Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 401 => {
            // 401 is expected since we didn't provide credentials
            Ok(Json(SsoTestResult {
                success: true,
                message: "GitHub API is reachable".to_string(),
                details: Some(serde_json::json!({
                    "provider_key": config.provider_key,
                    "client_id_configured": config.client_id.is_some(),
                    "client_secret_configured": config.client_secret_encrypted.is_some(),
                    "auth_url": "https://github.com/login/oauth/authorize",
                })),
            }))
        }
        Ok(resp) => Ok(Json(SsoTestResult {
            success: false,
            message: format!("GitHub API returned unexpected status: {}", resp.status()),
            details: None,
        })),
        Err(e) => Ok(Json(SsoTestResult {
            success: false,
            message: e.to_string(),
            details: None,
        })),
    }
}

/// Test OIDC configuration via discovery
async fn test_oidc_config(config: &SsoConfig) -> ApiResult<Json<SsoTestResult>> {
    let issuer = match &config.issuer_url {
        Some(url) => url,
        None => {
            return Ok(Json(SsoTestResult {
                success: false,
                message: "Issuer URL not configured".to_string(),
                details: None,
            }));
        }
    };

    let discovery = OidcDiscoveryService::new();

    // Attempt OIDC discovery
    match discovery.discover(issuer).await {
        Ok(result) => {
            // Try to fetch JWKS to verify it's accessible
            match discovery.fetch_jwks(&result.jwks_uri).await {
                Ok(jwks) => Ok(Json(SsoTestResult {
                    success: true,
                    message: "OIDC discovery and JWKS fetch successful".to_string(),
                    details: Some(serde_json::json!({
                        "issuer": result.issuer,
                        "authorization_endpoint": result.authorization_endpoint,
                        "token_endpoint": result.token_endpoint,
                        "userinfo_endpoint": result.userinfo_endpoint,
                        "jwks_keys_count": jwks.keys.len(),
                        "scopes_supported": result.scopes_supported,
                        "response_types_supported": result.response_types_supported,
                    })),
                })),
                Err(e) => Ok(Json(SsoTestResult {
                    success: false,
                    message: format!("OIDC discovery succeeded but JWKS fetch failed: {}", e),
                    details: Some(serde_json::json!({
                        "issuer": result.issuer,
                        "jwks_uri": result.jwks_uri,
                    })),
                })),
            }
        }
        Err(e) => Ok(Json(SsoTestResult {
            success: false,
            message: format!("OIDC discovery failed: {}", e),
            details: Some(serde_json::json!({
                "issuer_url": issuer,
                "discovery_url": format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/')),
            })),
        })),
    }
}

// ============ Retention Policies ============

async fn list_retention_policies(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<RetentionPolicy>>> {
    require_admin(&auth)?;

    let policies: Vec<RetentionPolicy> = if let Some(org_id) = auth.org_id {
        sqlx::query_as(
            "SELECT * FROM retention_policies WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(&state.db)
        .await?
    } else if auth.has_permission(&policy_permissions::ADMIN_FULL) {
        sqlx::query_as("SELECT * FROM retention_policies ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?
    } else {
        vec![]
    };

    Ok(Json(policies))
}

async fn create_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateRetentionPolicy>,
) -> ApiResult<Json<RetentionPolicy>> {
    require_admin(&auth)?;

    // Validate scope
    let scope_count = [input.org_id, input.team_id, input.channel_id]
        .iter()
        .filter(|x| x.is_some())
        .count();

    if scope_count != 1 {
        return Err(AppError::Validation(
            "Exactly one of org_id, team_id, or channel_id required".to_string(),
        ));
    }

    let policy: RetentionPolicy = sqlx::query_as(
        r#"
        INSERT INTO retention_policies (org_id, team_id, channel_id, retention_days, delete_files)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.org_id)
    .bind(input.team_id)
    .bind(input.channel_id)
    .bind(input.retention_days)
    .bind(input.delete_files)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(policy))
}

async fn get_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RetentionPolicy>> {
    require_admin(&auth)?;

    let policy: RetentionPolicy = sqlx::query_as("SELECT * FROM retention_policies WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Policy not found".to_string()))?;

    Ok(Json(policy))
}

async fn delete_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM retention_policies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Permissions ============

async fn list_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<Permission>>> {
    require_admin(&auth)?;

    let permissions: Vec<Permission> =
        sqlx::query_as("SELECT * FROM permissions ORDER BY category, id")
            .fetch_all(&state.db)
            .await?;

    Ok(Json(permissions))
}

async fn get_role_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(role): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    require_admin(&auth)?;

    let permissions: Vec<(String,)> =
        sqlx::query_as("SELECT permission_id FROM role_permissions WHERE role = $1")
            .bind(&role)
            .fetch_all(&state.db)
            .await?;

    Ok(Json(permissions.into_iter().map(|p| p.0).collect()))
}

#[derive(Deserialize)]
struct RolePermissionsUpdate {
    permissions: Vec<String>,
}

async fn update_role_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(role): Path<String>,
    Json(input): Json<RolePermissionsUpdate>,
) -> ApiResult<Json<Vec<String>>> {
    require_admin(&auth)?;

    let valid_permissions: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM permissions WHERE id = ANY($1)")
            .bind(&input.permissions)
            .fetch_all(&state.db)
            .await?;

    let valid_ids: Vec<String> = valid_permissions.into_iter().map(|p| p.0).collect();

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM role_permissions WHERE role = $1")
        .bind(&role)
        .execute(&mut *tx)
        .await?;

    for permission_id in &valid_ids {
        sqlx::query("INSERT INTO role_permissions (role, permission_id) VALUES ($1, $2)")
            .bind(&role)
            .bind(permission_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Json(valid_ids))
}

// ============ User Management ============

#[derive(Debug, serde::Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub role: Option<String>,
    pub search: Option<String>,
    pub include_deleted: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
pub struct UsersListResponse {
    pub users: Vec<crate::models::User>,
    pub total: i64,
}

async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListUsersQuery>,
) -> ApiResult<Json<UsersListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    let include_deleted = query.include_deleted.unwrap_or(false);

    let users: Vec<crate::models::User> = sqlx::query_as(
        r#"
        SELECT * FROM users
        WHERE ($1::BOOL IS NULL OR is_active = $1)
          AND ($2::VARCHAR IS NULL OR role = $2)
          AND ($3::VARCHAR IS NULL OR username ILIKE '%' || $3 || '%' OR email ILIKE '%' || $3 || '%')
          AND ($4::BOOL = TRUE OR deleted_at IS NULL)
        ORDER BY created_at DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(match query.status.as_deref() {
        Some("active") => Some(true),
        Some("inactive") => Some(false),
        _ => None,
    })
    .bind(&query.role)
    .bind(&query.search)
    .bind(include_deleted)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM users
        WHERE ($1::BOOL IS NULL OR is_active = $1)
          AND ($2::VARCHAR IS NULL OR role = $2)
          AND ($3::VARCHAR IS NULL OR username ILIKE '%' || $3 || '%' OR email ILIKE '%' || $3 || '%')
          AND ($4::BOOL = TRUE OR deleted_at IS NULL)
        "#,
    )
        .bind(match query.status.as_deref() {
            Some("active") => Some(true),
            Some("inactive") => Some(false),
            _ => None,
        })
        .bind(&query.role)
        .bind(&query.search)
        .bind(include_deleted)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(UsersListResponse {
        users,
        total: total.0,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
    pub display_name: Option<String>,
}

async fn create_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateUserInput>,
) -> ApiResult<Json<crate::models::User>> {
    require_admin(&auth)?;

    let password_hash = crate::auth::hash_password(&input.password)?;
    let role = input.role.unwrap_or_else(|| "member".to_string());

    let user: crate::models::User = sqlx::query_as(
        r#"
        INSERT INTO users (username, email, password_hash, role, display_name, is_active)
        VALUES ($1, $2, $3, $4, $5, true)
        RETURNING *
        "#,
    )
    .bind(&input.username)
    .bind(&input.email)
    .bind(&password_hash)
    .bind(&role)
    .bind(&input.display_name)
    .fetch_one(&state.db)
    .await?;

    // Apply auto-membership policies for the new user
    match apply_auto_membership_for_new_user(&state, user.id).await {
        Ok(audit_entries) => {
            let success_count = audit_entries
                .iter()
                .filter(|e| e.status == "success" && e.action == "add")
                .count();
            if success_count > 0 {
                tracing::info!("Applied auto-membership policies for admin-created user {}: {} memberships added", user.id, success_count);
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to apply auto-membership policies for admin-created user {}: {}",
                user.id,
                e
            );
        }
    }

    Ok(Json(user))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateUserInput {
    pub role: Option<String>,
    pub display_name: Option<String>,
}

async fn update_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUserInput>,
) -> ApiResult<Json<crate::models::User>> {
    require_admin(&auth)?;

    let user: crate::models::User = sqlx::query_as(
        r#"
        UPDATE users SET
            role = COALESCE($1, role),
            display_name = COALESCE($2, display_name),
            updated_at = NOW()
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(&input.role)
    .bind(&input.display_name)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user))
}

async fn deactivate_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deactivated"})))
}

async fn reactivate_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("UPDATE users SET is_active = true, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "reactivated"})))
}

#[derive(Debug, serde::Deserialize)]
pub struct DeleteAdminUserInput {
    pub confirm: String,
    pub reason: Option<String>,
}

async fn delete_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<DeleteAdminUserInput>,
) -> ApiResult<Json<serde_json::Value>> {
    require_global_admin(&auth)?;

    if auth.user_id == id {
        return Err(AppError::Conflict(
            "You cannot delete your own account while logged in".to_string(),
        ));
    }

    let target: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if target.deleted_at.is_some() {
        return Err(AppError::Conflict("User is already deleted".to_string()));
    }

    let confirm = input.confirm.trim();
    if confirm != target.username && confirm != target.email {
        return Err(AppError::BadRequest(
            "Confirmation text must exactly match the user's username or email".to_string(),
        ));
    }

    if target.role == "system_admin" {
        let admin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE role = 'system_admin' AND deleted_at IS NULL",
        )
        .fetch_one(&state.db)
        .await?;

        if admin_count <= 1 {
            return Err(AppError::Conflict(
                "Cannot delete the last remaining global admin".to_string(),
            ));
        }
    }

    let reason = input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    let mut tx = state.db.begin().await?;

    let deleted_user: crate::models::User = sqlx::query_as(
        r#"
        UPDATE users
        SET is_active = false,
            deleted_at = NOW(),
            deleted_by = $2,
            delete_reason = $3,
            updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&reason)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM upload_sessions WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    insert_admin_audit_log(
        &state.db,
        auth.user_id,
        "user.soft_delete",
        "user",
        Some(id),
        serde_json::json!({
            "username": target.username,
            "email": target.email,
            "reason": reason,
        }),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "user_id": deleted_user.id,
        "deleted_at": deleted_user.deleted_at,
    })))
}

/// Permanently wipe a soft-deleted user from the database.
/// Only allowed if the user has no posts/messages.
async fn wipe_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_global_admin(&auth)?;

    if auth.user_id == id {
        return Err(AppError::Conflict(
            "You cannot wipe your own account".to_string(),
        ));
    }

    // Get the user and verify they are soft-deleted
    let target: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if target.deleted_at.is_none() {
        return Err(AppError::Conflict(
            "User must be soft-deleted before wiping. Use DELETE endpoint first.".to_string(),
        ));
    }

    // Check if user has any posts
    let post_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE user_id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    if post_count > 0 {
        return Err(AppError::Conflict(format!(
            "Cannot wipe user with {} post(s). User has messages in channels.",
            post_count
        )));
    }

    // Begin transaction to delete user and related data
    let mut tx = state.db.begin().await?;

    // Delete related data in proper order (respecting foreign keys)
    // Delete user preferences
    sqlx::query("DELETE FROM user_preferences WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Delete channel memberships
    sqlx::query("DELETE FROM channel_members WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Delete team memberships
    sqlx::query("DELETE FROM team_members WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Delete reactions by user
    sqlx::query("DELETE FROM reactions WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Delete saved posts
    sqlx::query("DELETE FROM saved_posts WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Delete upload sessions
    sqlx::query("DELETE FROM upload_sessions WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Delete password reset tokens
    sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Finally, permanently delete the user
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Log the wipe action
    insert_admin_audit_log(
        &state.db,
        auth.user_id,
        "user.wipe",
        "user",
        Some(id),
        serde_json::json!({
            "username": target.username,
            "email": target.email,
            "deleted_at": target.deleted_at,
        }),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "status": "wiped",
        "user_id": id,
        "message": "User permanently deleted from database",
    })))
}

// ============ Stats & Health ============

#[derive(Debug, serde::Serialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_teams: i64,
    pub total_channels: i64,
    pub messages_24h: i64,
    pub files_count: i64,
}

async fn get_stats(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<SystemStats>> {
    require_admin(&auth)?;

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let active_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = true")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let total_teams: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let total_channels: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM channels")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let messages_24h: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM posts WHERE created_at > NOW() - INTERVAL '24 hours'")
            .fetch_one(&state.db)
            .await
            .unwrap_or((0,));
    let files_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    Ok(Json(SystemStats {
        total_users: total_users.0,
        active_users: active_users.0,
        total_teams: total_teams.0,
        total_channels: total_channels.0,
        messages_24h: messages_24h.0,
        files_count: files_count.0,
    }))
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub database: DatabaseHealth,
    pub storage: StorageHealth,
    pub redis: RedisHealth,
    pub disk: DiskHealth,
    pub websocket: WebSocketHealth,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub latency_ms: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct StorageHealth {
    pub connected: bool,
    #[serde(rename = "type")]
    pub storage_type: String,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct RedisHealth {
    pub connected: bool,
    pub latency_ms: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DiskHealth {
    pub connected: bool,
    pub used_percent: u64,
    pub available_mb: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct WebSocketHealth {
    pub active_connections: u64,
}

use std::sync::Mutex;
use std::time::Instant;

struct HealthCache {
    timestamp: Instant,
    status: HealthStatus,
}

static HEALTH_CACHE: Mutex<Option<HealthCache>> = Mutex::new(None);
const HEALTH_CACHE_TTL: Duration = Duration::from_secs(30);

async fn get_health(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<HealthStatus>> {
    require_admin(&auth)?;

    // Return cached result if still fresh
    {
        let cache = HEALTH_CACHE.lock().unwrap();
        if let Some(ref cached) = *cache {
            if cached.timestamp.elapsed() < HEALTH_CACHE_TTL {
                return Ok(Json(cached.status.clone()));
            }
        }
    }

    // Check DB
    let db_start = std::time::Instant::now();
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let db_latency = db_start.elapsed().as_millis() as u64;

    // Check Redis
    let redis_start = std::time::Instant::now();
    let redis_ok = check_redis_admin(&state.redis).await;
    let redis_latency = redis_start.elapsed().as_millis() as u64;

    // Check S3
    let s3_ok = state.s3_client.health_check().await;

    // Check disk (root mount)
    let disk = check_disk("/");

    let all_ok = db_ok && redis_ok && s3_ok && disk.connected;

    let status = HealthStatus {
        status: if all_ok {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        database: DatabaseHealth {
            connected: db_ok,
            latency_ms: db_latency,
        },
        storage: StorageHealth {
            connected: s3_ok,
            storage_type: "s3".to_string(),
        },
        redis: RedisHealth {
            connected: redis_ok,
            latency_ms: redis_latency,
        },
        disk,
        websocket: WebSocketHealth {
            active_connections: state.ws_hub.count_connections().await as u64,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    };

    // Update cache
    {
        let mut cache = HEALTH_CACHE.lock().unwrap();
        *cache = Some(HealthCache {
            timestamp: Instant::now(),
            status: status.clone(),
        });
    }

    Ok(Json(status))
}

async fn check_redis_admin(redis: &deadpool_redis::Pool) -> bool {
    crate::api::health::check_redis(redis).await
}

fn check_disk(path: &str) -> DiskHealth {
    #[cfg(unix)]
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        let c_path = match std::ffi::CString::new(path) {
            Ok(p) => p,
            Err(_) => {
                return DiskHealth {
                    connected: false,
                    used_percent: 0,
                    available_mb: 0,
                }
            }
        };
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            let block_size = stat.f_frsize;
            let total = (stat.f_blocks as u64) * block_size;
            let available = (stat.f_bavail as u64) * block_size;
            let used = total.saturating_sub(available);
            let used_percent = if total > 0 {
                ((used as f64 / total as f64) * 100.0) as u64
            } else {
                0
            };
            DiskHealth {
                connected: true,
                used_percent,
                available_mb: available / (1024 * 1024),
            }
        } else {
            DiskHealth {
                connected: false,
                used_percent: 0,
                available_mb: 0,
            }
        }
    }
    #[cfg(not(unix))]
    {
        // Non-Unix fallback: verify we can write a temp file
        let temp_path = std::env::temp_dir().join("rustchat_disk_check.tmp");
        let writable = std::fs::write(&temp_path, b"check").is_ok()
            && std::fs::remove_file(&temp_path).is_ok();
        DiskHealth {
            connected: writable,
            used_percent: 0,
            available_mb: 0,
        }
    }
}

// ============ Teams & Channels Management ============

#[derive(Debug, serde::Deserialize)]
pub struct ListTeamsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, serde::Serialize, FromRow)]
pub struct AdminTeamResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub team: crate::models::team::Team,
    pub members_count: i64,
    pub channels_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminTeamsListResponse {
    pub teams: Vec<AdminTeamResponse>,
    pub total: i64,
}

async fn list_admin_teams(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListTeamsQuery>,
) -> ApiResult<Json<AdminTeamsListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let teams: Vec<AdminTeamResponse> = sqlx::query_as(
        r#"
        SELECT t.*, 
               (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) as members_count,
               (SELECT COUNT(*) FROM channels WHERE team_id = t.id) as channels_count
        FROM teams t
        WHERE ($1::VARCHAR IS NULL OR t.name ILIKE '%' || $1 || '%' OR t.display_name ILIKE '%' || $1 || '%')
        ORDER BY t.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&query.search)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(AdminTeamsListResponse {
        teams,
        total: total.0,
    }))
}

async fn get_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AdminTeamResponse>> {
    require_admin(&auth)?;

    let team: AdminTeamResponse = sqlx::query_as(
        r#"
        SELECT t.*, 
               (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) as members_count,
               (SELECT COUNT(*) FROM channels WHERE team_id = t.id) as channels_count
        FROM teams t
        WHERE t.id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(team))
}

#[derive(Debug, serde::Deserialize)]
struct UpdateTeamRequest {
    display_name: Option<String>,
    description: Option<String>,
}

async fn update_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTeamRequest>,
) -> ApiResult<Json<AdminTeamResponse>> {
    require_admin(&auth)?;

    // Update the team with provided fields
    sqlx::query(
        r#"
        UPDATE teams 
        SET display_name = COALESCE($2, display_name),
            description = COALESCE($3, description)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&payload.display_name)
    .bind(&payload.description)
    .execute(&state.db)
    .await?;

    // Fetch and return updated team
    let team: AdminTeamResponse = sqlx::query_as(
        r#"
        SELECT t.*, 
               (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) as members_count,
               (SELECT COUNT(*) FROM channels WHERE team_id = t.id) as channels_count
        FROM teams t
        WHERE t.id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(team))
}

async fn delete_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    // Cascade delete in the database handles related members/channels/posts
    sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn list_team_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<TeamMemberResponse>>> {
    require_admin(&auth)?;

    let mut members = sqlx::query_as::<_, TeamMemberResponse>(
        r#"
        SELECT tm.team_id, tm.user_id, tm.role, tm.created_at,
               u.username, u.display_name, u.avatar_url,
               COALESCE(tm.presence, 'offline') as presence
        FROM team_members tm
        JOIN users u ON tm.user_id = u.id
        WHERE tm.team_id = $1
        ORDER BY u.username
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    for member in &mut members {
        member.avatar_url = normalize_avatar_url(member.user_id, member.avatar_url.as_deref());
    }

    Ok(Json(members))
}

async fn add_team_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddTeamMember>,
) -> ApiResult<Json<TeamMember>> {
    require_admin(&auth)?;

    let member = sqlx::query_as::<_, TeamMember>(
        r#"
        INSERT INTO team_members (team_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(payload.user_id)
    .bind(payload.role.unwrap_or_else(|| "member".into()))
    .fetch_one(&state.db)
    .await?;

    if let Err(err) =
        apply_default_channel_membership_for_team_join(&state, id, payload.user_id).await
    {
        tracing::warn!(
            team_id = %id,
            user_id = %payload.user_id,
            error = %err,
            "Default channel auto-join failed after admin add_team_member"
        );
    }

    Ok(Json(member))
}

async fn remove_team_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "removed"})))
}

#[derive(Debug, serde::Deserialize)]
pub struct ListChannelsQuery {
    pub team_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, serde::Serialize, FromRow)]
pub struct AdminChannelResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub channel: crate::models::channel::Channel,
    pub members_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminChannelsListResponse {
    pub channels: Vec<AdminChannelResponse>,
    pub total: i64,
}

async fn list_admin_channels(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListChannelsQuery>,
) -> ApiResult<Json<AdminChannelsListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let channels: Vec<AdminChannelResponse> = sqlx::query_as(
        r#"
        SELECT c.*, 
               (SELECT COUNT(*) FROM channel_members WHERE channel_id = c.id) as members_count
        FROM channels c
        WHERE ($1::UUID IS NULL OR c.team_id = $1)
          AND ($2::VARCHAR IS NULL OR c.name ILIKE '%' || $2 || '%' OR c.display_name ILIKE '%' || $2 || '%')
        ORDER BY c.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(query.team_id)
    .bind(&query.search)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM channels WHERE ($1::UUID IS NULL OR team_id = $1)")
            .bind(query.team_id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(AdminChannelsListResponse {
        channels,
        total: total.0,
    }))
}

async fn create_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateChannel>,
) -> ApiResult<Json<crate::models::channel::Channel>> {
    require_admin(&auth)?;

    // Create channel
    let channel: crate::models::channel::Channel = sqlx::query_as(
        r#"
        INSERT INTO channels (team_id, name, display_name, purpose, type, creator_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(input.team_id)
    .bind(&input.name)
    .bind(&input.display_name)
    .bind(&input.purpose)
    .bind(input.channel_type)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    // Broadcast event
    let broadcast = if channel.channel_type == crate::models::ChannelType::Public {
        // Broadcast to entire team
        crate::realtime::WsBroadcast {
            team_id: Some(input.team_id),
            channel_id: None,
            user_id: None,
            exclude_user_id: None,
        }
    } else {
        // Private channel: broadcast only to creator (admin)
        crate::realtime::WsBroadcast {
            user_id: Some(auth.user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        }
    };

    let event = crate::realtime::WsEnvelope::event(
        crate::realtime::events::EventType::ChannelCreated,
        channel.clone(),
        Some(channel.id),
    )
    .with_broadcast(broadcast);

    state.ws_hub.broadcast(event).await;

    Ok(Json(channel))
}

async fn update_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateChannel>,
) -> ApiResult<Json<crate::models::channel::Channel>> {
    require_admin(&auth)?;

    // Update fields
    if let Some(ref display_name) = input.display_name {
        sqlx::query("UPDATE channels SET display_name = $1 WHERE id = $2")
            .bind(display_name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref purpose) = input.purpose {
        sqlx::query("UPDATE channels SET purpose = $1 WHERE id = $2")
            .bind(purpose)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref header) = input.header {
        sqlx::query("UPDATE channels SET header = $1 WHERE id = $2")
            .bind(header)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    let channel: crate::models::channel::Channel =
        sqlx::query_as("SELECT * FROM channels WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(channel))
}

async fn delete_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM channels WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Plugins - RustChat Calls Plugin ============

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct CallsPluginConfig {
    pub enabled: bool,
    pub turn_server_enabled: bool,
    pub turn_server_url: String,
    pub turn_server_username: String,
    #[serde(skip_serializing)]
    pub turn_server_credential: String,
    pub udp_port: u16,
    pub tcp_port: u16,
    pub ice_host_override: Option<String>,
    pub stun_servers: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateCallsPluginConfig {
    pub enabled: bool,
    pub turn_server_enabled: bool,
    pub turn_server_url: String,
    pub turn_server_username: String,
    pub turn_server_credential: Option<String>,
    pub udp_port: u16,
    pub tcp_port: u16,
    pub ice_host_override: Option<String>,
    pub stun_servers: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CallsPluginConfigResponse {
    pub plugin_id: String,
    pub plugin_name: String,
    pub settings: CallsPluginConfig,
}

async fn get_calls_plugin_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<CallsPluginConfigResponse>> {
    require_admin(&auth)?;

    // Get config from database (server_config table, plugins column)
    let config: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT plugins->'calls' FROM server_config WHERE id = 'default'")
            .fetch_optional(&state.db)
            .await?;

    let calls_config = config
        .and_then(|(json,)| json.as_object().cloned())
        .map(|obj| CallsPluginConfig {
            enabled: obj
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(state.config.calls.enabled),
            turn_server_enabled: obj
                .get("turn_server_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(state.config.calls.turn_server_enabled),
            turn_server_url: obj
                .get("turn_server_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| state.config.calls.turn_server_url.clone()),
            turn_server_username: obj
                .get("turn_server_username")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| state.config.calls.turn_server_username.clone()),
            turn_server_credential: obj
                .get("turn_server_credential")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| state.config.calls.turn_server_credential.clone()),
            udp_port: obj
                .get("udp_port")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16)
                .unwrap_or(state.config.calls.udp_port),
            tcp_port: obj
                .get("tcp_port")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16)
                .unwrap_or(state.config.calls.tcp_port),
            ice_host_override: obj
                .get("ice_host_override")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            stun_servers: obj
                .get("stun_servers")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_else(|| state.config.calls.stun_servers.clone()),
        })
        .unwrap_or_else(|| CallsPluginConfig {
            enabled: state.config.calls.enabled,
            turn_server_enabled: state.config.calls.turn_server_enabled,
            turn_server_url: state.config.calls.turn_server_url.clone(),
            turn_server_username: state.config.calls.turn_server_username.clone(),
            turn_server_credential: state.config.calls.turn_server_credential.clone(),
            udp_port: state.config.calls.udp_port,
            tcp_port: state.config.calls.tcp_port,
            ice_host_override: state.config.calls.ice_host_override.clone(),
            stun_servers: state.config.calls.stun_servers.clone(),
        });

    Ok(Json(CallsPluginConfigResponse {
        plugin_id: "com.rustchat.calls".to_string(),
        plugin_name: "RustChat Calls Plugin".to_string(),
        settings: calls_config,
    }))
}

async fn update_calls_plugin_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<Json<CallsPluginConfigResponse>> {
    require_admin(&auth)?;

    // Log the incoming payload for debugging
    tracing::info!("Received Calls Plugin config update: {}", payload);

    // Deserialize manually to get better error messages
    let payload: UpdateCallsPluginConfig = serde_json::from_value(payload).map_err(|e| {
        tracing::error!("Failed to deserialize Calls Plugin config: {}", e);
        AppError::BadRequest(format!("Invalid configuration data: {}", e))
    })?;

    // Get existing credential if not provided in update
    let credential = if let Some(ref cred) = payload.turn_server_credential {
        cred.clone()
    } else {
        // Fetch existing credential from database
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT plugins->'calls'->>'turn_server_credential' FROM server_config WHERE id = 'default'"
        )
        .fetch_optional(&state.db)
        .await?;
        existing
            .map(|(s,)| s)
            .unwrap_or_else(|| state.config.calls.turn_server_credential.clone())
    };

    // Build JSON object for calls config
    let calls_config_json = serde_json::json!({
        "enabled": payload.enabled,
        "turn_server_enabled": payload.turn_server_enabled,
        "turn_server_url": payload.turn_server_url,
        "turn_server_username": payload.turn_server_username,
        "turn_server_credential": credential,
        "udp_port": payload.udp_port,
        "tcp_port": payload.tcp_port,
        "ice_host_override": payload.ice_host_override,
        "stun_servers": payload.stun_servers,
    });

    // Update server_config table
    sqlx::query(
        r#"
        INSERT INTO server_config (id, plugins, updated_at, updated_by)
        VALUES ('default', jsonb_build_object('calls', $1::jsonb), NOW(), $2)
        ON CONFLICT (id) DO UPDATE SET
            plugins = jsonb_set(
                COALESCE(server_config.plugins, '{}'::jsonb),
                '{calls}',
                $1::jsonb,
                true
            ),
            updated_at = NOW(),
            updated_by = $2
        "#,
    )
    .bind(calls_config_json)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(CallsPluginConfigResponse {
        plugin_id: "com.rustchat.calls".to_string(),
        plugin_name: "RustChat Calls Plugin".to_string(),
        settings: CallsPluginConfig {
            enabled: payload.enabled,
            turn_server_enabled: payload.turn_server_enabled,
            turn_server_url: payload.turn_server_url,
            turn_server_username: payload.turn_server_username,
            turn_server_credential: credential,
            udp_port: payload.udp_port,
            tcp_port: payload.tcp_port,
            ice_host_override: payload.ice_host_override,
            stun_servers: payload.stun_servers,
        },
    }))
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

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
#[allow(dead_code)]
struct EmailEventRow {
    id: Uuid,
    action: String,
    created_at: chrono::DateTime<chrono::Utc>,
    metadata: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct EmailEventResponse {
    id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    kind: String,
    success: bool,
    recipient: Option<String>,
    smtp_host: Option<String>,
    smtp_port: Option<i32>,
    smtp_security: Option<String>,
    message: Option<String>,
    error_kind: Option<String>,
}

fn classify_email_error(message: &str) -> (&'static str, &'static str) {
    let lower = message.to_ascii_lowercase();
    if lower.contains("invalid peer certificate") || lower.contains("certificate") {
        (
            "tls_handshake_failed",
            "TLS certificate validation failed. Use the certificate hostname (not an IP), or enable certificate-skip only for testing.",
        )
    } else if lower.contains("authentication") || lower.contains("auth") {
        (
            "auth_failed",
            "SMTP authentication failed. Verify username/password and auth method.",
        )
    } else if lower.contains("dns")
        || lower.contains("name or service not known")
        || lower.contains("no such host")
    {
        (
            "dns_error",
            "SMTP hostname lookup failed. Verify the SMTP host value.",
        )
    } else if lower.contains("timed out") || lower.contains("timeout") {
        (
            "timeout",
            "SMTP connection timed out. Verify firewall, port, and SMTP host reachability.",
        )
    } else if lower.contains("relay") || lower.contains("denied") || lower.contains("not permitted")
    {
        ("relay_denied", "SMTP server rejected relaying. Verify from address, recipient policy, and account permissions.")
    } else if lower.contains("invalid from") || lower.contains("from address") {
        (
            "invalid_from",
            "The configured from address is invalid or rejected by the SMTP server.",
        )
    } else if lower.contains("invalid to address")
        || lower.contains("mailbox")
        || lower.contains("recipient")
    {
        (
            "invalid_recipient",
            "Recipient address is invalid or rejected by the SMTP server.",
        )
    } else if lower.contains("connection error") || lower.contains("refused") {
        (
            "connect_failed",
            "SMTP connection failed. Verify host, port, and TLS mode.",
        )
    } else {
        (
            "smtp_error",
            "SMTP request failed. Check server logs and SMTP settings.",
        )
    }
}

async fn record_provider_email_test_event(
    state: &AppState,
    actor_user_id: Uuid,
    provider: &crate::models::email::MailProviderSettings,
    recipient: &str,
    success: bool,
    message: Option<String>,
    error_kind: Option<&str>,
) {
    let metadata = serde_json::json!({
        "recipient": recipient,
        "success": success,
        "smtp_host": provider.host,
        "smtp_port": provider.port,
        "smtp_security": provider.tls_mode.as_str(),
        "from_address": provider.from_address,
        "provider_id": provider.id,
        "message": message,
        "error_kind": error_kind,
    });

    if let Err(e) = insert_admin_audit_log(
        &state.db,
        actor_user_id,
        if success {
            "email.test.success"
        } else {
            "email.test.failure"
        },
        "email",
        None,
        metadata,
    )
    .await
    {
        tracing::warn!(error = %e, "Failed to record email test audit event");
    }
}

#[allow(dead_code)]
async fn list_email_events(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<EmailEventResponse>>> {
    require_admin(&auth)?;

    let rows: Vec<EmailEventRow> = sqlx::query_as(
        r#"
        SELECT id, action, created_at, metadata
        FROM audit_logs
        WHERE target_type = 'email'
          AND action IN ('email.test.success', 'email.test.failure')
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let events = rows
        .into_iter()
        .map(|row| EmailEventResponse {
            id: row.id,
            created_at: row.created_at,
            kind: "test".to_string(),
            success: row.action == "email.test.success",
            recipient: row
                .metadata
                .get("recipient")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            smtp_host: row
                .metadata
                .get("smtp_host")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            smtp_port: row
                .metadata
                .get("smtp_port")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            smtp_security: row
                .metadata
                .get("smtp_security")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            message: row
                .metadata
                .get("message")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            error_kind: row
                .metadata
                .get("error_kind")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
        .collect();

    Ok(Json(events))
}

async fn test_email_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<TestEmailRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    // Get default provider from the new provider system
    let provider_settings: crate::models::email::MailProviderSettings = sqlx::query_as(
        r#"
        SELECT * FROM mail_provider_settings
        WHERE enabled = true AND is_default = true
        ORDER BY tenant_id NULLS LAST
        LIMIT 1
        "#,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
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

// ============================================================================
// Groups Management (for Membership Policies)
// ============================================================================

#[derive(Debug, serde::Serialize, FromRow)]
pub struct AdminGroupResponse {
    pub id: Uuid,
    pub name: Option<String>,
    pub display_name: String,
    pub description: String,
    pub source: String,
    pub remote_id: Option<String>,
    pub member_count: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListGroupsQuery {
    pub source: Option<String>,
    pub search: Option<String>,
}

/// List all groups for membership policy configuration
async fn list_admin_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListGroupsQuery>,
) -> ApiResult<Json<Vec<AdminGroupResponse>>> {
    require_admin(&auth)?;

    let groups: Vec<AdminGroupResponse> = sqlx::query_as(
        r#"
        SELECT 
            g.id,
            g.name,
            g.display_name,
            g.description,
            g.source,
            g.remote_id,
            (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) as member_count
        FROM groups g
        WHERE g.deleted_at IS NULL
          AND ($1::VARCHAR IS NULL OR g.source = $1)
          AND ($2::VARCHAR IS NULL OR 
               g.display_name ILIKE '%' || $2 || '%' OR 
               g.name ILIKE '%' || $2 || '%')
        ORDER BY g.display_name
        "#,
    )
    .bind(&query.source)
    .bind(&query.search)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(groups))
}
