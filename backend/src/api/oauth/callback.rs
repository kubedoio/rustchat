use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Redirect},
};
use deadpool_redis::redis::AsyncCommands;
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::error::{ApiResult, AppError};
use crate::models::SsoConfig;
use crate::repositories::OAuthRepository;
use crate::services::membership_policies::apply_auto_membership_for_new_user;
use crate::services::oauth_token_exchange::{
    create_exchange_code, create_exchange_code_with_sso, SsoExchangeChallenge,
};

use super::{
    OAuthCallbackQuery, OAuthStatePayload, UserInfo, OAUTH_STATE_TTL_SECONDS,
};
use super::providers::{exchange_github_token, exchange_oidc_token};
use super::utils::{
    append_query_param, build_exchange_code_cookie, clear_exchange_code_cookie, get_site_url,
    oauth_state_key,
};

/// Handle OAuth callback from provider
pub async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider_key): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<axum::response::Response, AppError> {
    // Handle provider error
    if let Some(error) = query.error {
        let desc = query.error_description.unwrap_or_else(|| error.clone());
        tracing::warn!(
            provider = %provider_key,
            error = %error,
            "OAuth provider returned error"
        );
        return Ok(
            Redirect::temporary(&format!("/login?error={}", urlencoding::encode(&desc)))
                .into_response(),
        );
    }

    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest("Missing authorization code".to_string()))?;
    let oauth_state = query
        .state
        .ok_or_else(|| AppError::BadRequest("Missing OAuth state parameter".to_string()))?;

    // Validate and consume state from Redis (one-time use)
    let mut redis_conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("Redis connection failed: {}", e)))?;

    let state_key = oauth_state_key(&oauth_state);
    let stored_state_json: Option<String> =
        redis_conn
            .get::<_, Option<String>>(&state_key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read OAuth state: {}", e)))?;

    // Delete state immediately (one-time use)
    let _: () = redis_conn
        .del(&state_key)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete OAuth state: {}", e)))?;

    let stored_state_json = stored_state_json
        .ok_or_else(|| AppError::BadRequest("Invalid or expired OAuth state".to_string()))?;

    let stored_state: OAuthStatePayload = serde_json::from_str(&stored_state_json)
        .map_err(|e| AppError::Internal(format!("Invalid OAuth state payload: {}", e)))?;

    if stored_state.provider_key != provider_key {
        return Err(AppError::BadRequest(
            "OAuth state provider mismatch".to_string(),
        ));
    }

    // Check state age (prevent replay attacks with stolen states)
    let state_age = chrono::Utc::now().timestamp() - stored_state.created_at;
    if state_age > OAUTH_STATE_TTL_SECONDS as i64 {
        return Err(AppError::BadRequest("OAuth state expired".to_string()));
    }

    // Load provider config
    let config = OAuthRepository::new(&state.db)
        .get_active_sso_config_by_provider_key(&provider_key)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("OAuth provider '{}' not found", provider_key)))?;

    let provider_type =
        crate::models::SsoProviderType::from_str(&config.provider_type).ok_or_else(|| {
            AppError::Internal(format!("Unknown provider type: {}", config.provider_type))
        })?;

    let client_id = config.client_id.clone().ok_or_else(|| {
        AppError::BadRequest("OAuth provider client_id not configured".to_string())
    })?;

    // Decrypt client secret
    let secret_raw = config.client_secret_encrypted.clone().ok_or_else(|| {
        AppError::BadRequest("OAuth provider client_secret not configured".to_string())
    })?;

    let client_secret = match crypto::decrypt(&secret_raw, &state.config.encryption_key) {
        Ok(secret) => secret,
        Err(_) if state.config.is_production() => {
            return Err(AppError::Internal(
                "Failed to decrypt OAuth client secret".to_string(),
            ));
        }
        Err(e) => {
            tracing::warn!(
                provider = %provider_key,
                error = %e,
                "Failed to decrypt secret, using raw value (dev mode)"
            );
            secret_raw
        }
    };

    let callback_url = format!(
        "{}/api/v1/oauth2/{}/callback",
        get_site_url(&state.db).await,
        provider_key
    );

    // Exchange code for token and get user info based on provider type
    let (email, user_info) = match provider_type {
        crate::models::SsoProviderType::GitHub => {
            exchange_github_token(&code, &client_id, &client_secret, &callback_url, &config).await?
        }
        crate::models::SsoProviderType::Google | crate::models::SsoProviderType::Oidc => {
            let issuer = config.issuer_url.clone().ok_or_else(|| {
                AppError::BadRequest("OIDC provider issuer_url not configured".to_string())
            })?;

            exchange_oidc_token(
                &code,
                &client_id,
                &client_secret,
                &callback_url,
                &issuer,
                stored_state.code_verifier.as_deref(),
                stored_state.nonce.as_deref(),
                &config,
            )
            .await?
        }
        crate::models::SsoProviderType::Saml => {
            return Err(AppError::BadRequest("SAML not supported".to_string()));
        }
    };

    // Find or create user
    let user = find_or_create_user(&state, &email, &user_info, &config, &provider_key).await?;

    // Sync OIDC groups for the user (store for membership policy evaluation)
    if !user_info.groups.is_empty() {
        if let Err(e) = OAuthRepository::new(&state.db)
            .sync_oidc_groups(user.id, &provider_key, &user_info.groups)
            .await
        {
            tracing::warn!("Failed to sync OIDC groups for user {}: {}", user.id, e);
            // Don't fail login if group sync fails
        }
    }

    // Apply auto-membership policies for newly created OAuth users
    // Note: This is also called for existing users but is idempotent (skips if already member)
    match apply_auto_membership_for_new_user(&state, user.id).await {
        Ok(audit_entries) => {
            let success_count = audit_entries
                .iter()
                .filter(|e| e.status == "success" && e.action == "add")
                .count();
            if success_count > 0 {
                tracing::info!(
                    "Applied auto-membership policies for OAuth user {}: {} memberships added",
                    user.id,
                    success_count
                );
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to apply auto-membership policies for OAuth user {}: {}",
                user.id,
                e
            );
        }
    }

    // Secure default: one-time code exchange, never token in URL.
    let sso_challenge = if stored_state.is_mobile {
        match (
            stored_state.mobile_sso_state.clone(),
            stored_state.mobile_sso_code_challenge.clone(),
        ) {
            (Some(expected_state), Some(code_challenge)) => Some(SsoExchangeChallenge {
                expected_state,
                code_challenge,
                code_challenge_method: stored_state
                    .mobile_sso_code_challenge_method
                    .clone()
                    .unwrap_or_else(|| "S256".to_string()),
            }),
            _ => None,
        }
    } else {
        None
    };

    let exchange_code = if sso_challenge.is_some() {
        create_exchange_code_with_sso(
            &state.redis,
            user.id,
            user.email.clone(),
            user.role.clone(),
            user.org_id,
            sso_challenge,
        )
        .await?
    } else {
        create_exchange_code(
            &state.redis,
            user.id,
            user.email.clone(),
            user.role.clone(),
            user.org_id,
        )
        .await?
    };

    tracing::info!(
        user_id = %user.id,
        "OAuth callback using secure exchange code"
    );

    let site_url = get_site_url(&state.db).await;
    let redirect_url = if stored_state.is_mobile {
        // Mobile apps also use exchange codes
        let mobile_redirect_base = stored_state
            .mobile_redirect_to
            .clone()
            .unwrap_or_else(|| "rustchat://callback".to_string());
        let with_login_code =
            append_query_param(&mobile_redirect_base, "login_code", &exchange_code);
        let with_legacy_code = append_query_param(&with_login_code, "code", &exchange_code);
        append_query_param(&with_legacy_code, "srv", &site_url)
    } else {
        append_query_param(&stored_state.redirect_after, "oauth", "1")
    };
    if stored_state.is_mobile {
        return Ok(Redirect::temporary(&redirect_url).into_response());
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&build_exchange_code_cookie(
            &exchange_code,
            state.config.is_production(),
        ))
        .map_err(|e| AppError::Internal(format!("Failed to set exchange cookie: {}", e)))?,
    );

    Ok((response_headers, Redirect::temporary(&redirect_url)).into_response())
}

/// Find or create user from OAuth info
async fn find_or_create_user(
    state: &AppState,
    email: &str,
    user_info: &UserInfo,
    config: &SsoConfig,
    provider_key: &str,
) -> Result<crate::models::User, AppError> {
    use crate::models::User;

    let desired_role = determine_user_role(config, user_info);
    let external_id = user_info
        .external_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    let repo = OAuthRepository::new(&state.db);

    // 1) External-ID match takes precedence.
    if let Some(ext_id) = external_id {
        if let Some(user) = repo
            .get_user_by_auth_provider_and_id(provider_key, ext_id)
            .await?
        {
            let should_sync_role = config.provider_type == "oidc" && !desired_role.is_empty();
            let updated_user = repo
                .update_user_login(user.id, should_sync_role, &desired_role)
                .await?;
            return Ok(updated_user);
        }
    }

    // 2) Fallback to email match for first trusted link.
    if let Some(user) = repo.get_user_by_email(email).await? {
        let current_link = repo.get_user_auth_link_by_id(user.id).await?;

        if let Some(existing_external_id) = current_link.as_ref().and_then(|l| l.1.as_deref()) {
            let same_provider = current_link.as_ref().and_then(|l| l.0.as_deref()) == Some(provider_key);
            let same_external = external_id == Some(existing_external_id);
            if !same_provider || !same_external {
                return Err(AppError::Conflict(
                    "Account is already linked to a different SSO identity".to_string(),
                ));
            }
        }

        let should_link = external_id.is_some() && current_link.as_ref().and_then(|l| l.1.as_ref()).is_none();
        let should_sync_role = config.provider_type == "oidc" && !desired_role.is_empty();
        let updated_user = repo
            .update_user_login_and_link(
                user.id,
                should_link,
                provider_key,
                external_id,
                should_sync_role,
                &desired_role,
            )
            .await?;

        return Ok(updated_user);
    }

    // 3) Create user if auto-provisioning is enabled.
    if !config.auto_provision {
        return Err(AppError::Forbidden(
            "Account does not exist and auto-provisioning is disabled".to_string(),
        ));
    }

    let role = if desired_role.is_empty() {
        config
            .default_role
            .clone()
            .unwrap_or_else(|| "member".to_string())
    } else {
        desired_role
    };

    // Generate username from preferred_username, name, or email
    let username = user_info
        .preferred_username
        .clone()
        .or_else(|| {
            user_info.name.as_ref().map(|n| {
                n.to_lowercase()
                    .replace(' ', "_")
                    .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "")
            })
        })
        .unwrap_or_else(|| {
            email
                .split('@')
                .next()
                .unwrap_or("user")
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "")
        });

    // Ensure username is unique by appending numbers if needed
    let unique_username = generate_unique_username(&state.db, &username).await?;

    // Create new user (OAuth users have NULL password_hash)
    let user = repo
        .create_oauth_user(
            &unique_username,
            email,
            user_info.name.as_deref(),
            &role,
            provider_key,
            external_id,
            config.org_id,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create user: {}", e)))?;

    Ok(user)
}

fn determine_user_role(config: &SsoConfig, user_info: &UserInfo) -> String {
    let mut assigned_role = config
        .default_role
        .clone()
        .unwrap_or_else(|| "member".to_string());

    if let Some(ref mappings) = config.role_mappings {
        if let Some(mappings_obj) = mappings.as_object() {
            for group in &user_info.groups {
                if let Some(role_val) = mappings_obj.get(group).and_then(|v| v.as_str()) {
                    assigned_role = role_val.to_string();
                    break;
                }
            }
        }
    }

    assigned_role
}

/// Generate a unique username by appending numbers if needed
async fn generate_unique_username(
    db: &sqlx::PgPool,
    base_username: &str,
) -> Result<String, AppError> {
    let repo = OAuthRepository::new(db);

    if !repo.username_exists(base_username).await? {
        return Ok(base_username.to_string());
    }

    for i in 1..1000 {
        let candidate = format!("{}{}", base_username, i);
        if !repo.username_exists(&candidate).await? {
            return Ok(candidate);
        }
    }

    // Fallback to UUID suffix
    let unique_suffix = Uuid::new_v4()
        .to_string()
        .split('-')
        .next()
        .unwrap_or("user")
        .to_string();
    Ok(format!("{}_{}", base_username, unique_suffix))
}


