//! Admin SSO configuration endpoints

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::api::{admin::require_admin, AppState};
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::middleware::reliability::{send_reqwest_with_retry, RetryCondition, RetryConfig};
use crate::models::{
    CreateSsoConfig, SsoConfig, SsoConfigResponse, SsoProviderType, SsoTestResult, UpdateSsoConfig,
};
use crate::repositories::AdminRepository;
use crate::services::oidc_discovery::OidcDiscoveryService;
use std::time::Duration;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/sso", get(list_sso_configs).post(create_sso_config))
        .route(
            "/admin/sso/{id}",
            get(get_sso_config)
                .put(update_sso_config)
                .delete(delete_sso_config),
        )
        .route("/admin/sso/{id}/test", post(test_sso_config))
}

/// List all SSO configurations
pub async fn list_sso_configs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<SsoConfigResponse>>> {
    require_admin(&auth)?;

    let configs = AdminRepository::new(&state.db).list_sso_configs().await?;

    let responses: Vec<SsoConfigResponse> = configs.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

/// Get a single SSO configuration
pub async fn get_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SsoConfigResponse>> {
    require_admin(&auth)?;

    let config = AdminRepository::new(&state.db)
        .get_sso_config_by_id(id)
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
pub async fn create_sso_config(
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
    let existing = AdminRepository::new(&state.db)
        .get_sso_config_by_provider_key(&input.provider_key)
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

    let config = AdminRepository::new(&state.db)
        .insert_sso_config(org_id, &input, encrypted_secret, scopes)
        .await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    let config_id = config.id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            actor,
            crate::services::audit::AuditAction::SsoConfigCreate,
            "sso_config",
            Some(config_id),
            serde_json::Value::Null,
        )
        .await;
    });

    Ok(Json(config.into()))
}

/// Update an existing SSO configuration
pub async fn update_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateSsoConfig>,
) -> ApiResult<Json<SsoConfigResponse>> {
    require_admin(&auth)?;

    // Get existing config
    let existing = AdminRepository::new(&state.db)
        .get_sso_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("SSO configuration not found".to_string()))?;

    // Validate new provider_key if changing
    if let Some(ref new_key) = input.provider_key {
        if new_key != &existing.provider_key {
            validate_provider_key(new_key)?;
            // Check for duplicates
            let dup = AdminRepository::new(&state.db)
                .get_sso_config_by_provider_key(new_key)
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

    let config = AdminRepository::new(&state.db)
        .update_sso_config(id, &input, encrypted_secret)
        .await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    let config_id = id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            actor,
            crate::services::audit::AuditAction::SsoConfigUpdate,
            "sso_config",
            Some(config_id),
            serde_json::Value::Null,
        )
        .await;
    });

    Ok(Json(config.into()))
}

/// Delete an SSO configuration
pub async fn delete_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let deleted = AdminRepository::new(&state.db)
        .delete_sso_config(id)
        .await?;

    if !deleted {
        return Err(AppError::NotFound(
            "SSO configuration not found".to_string(),
        ));
    }

    let db = state.db.clone();
    let actor = auth.user_id;
    let config_id = id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            actor,
            crate::services::audit::AuditAction::SsoConfigDelete,
            "sso_config",
            Some(config_id),
            serde_json::Value::Null,
        )
        .await;
    });

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// Test an SSO configuration
pub async fn test_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SsoTestResult>> {
    require_admin(&auth)?;

    let config = AdminRepository::new(&state.db)
        .get_sso_config_by_id(id)
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
