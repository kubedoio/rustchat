use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    Json,
};
use deadpool_redis::redis::AsyncCommands;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::models::{OAuthProviderInfo, SsoProviderType};
use crate::repositories::OAuthRepository;
use crate::services::oidc_discovery::OidcDiscoveryService;

use super::utils::{
    generate_code_challenge, generate_code_verifier, generate_nonce, get_mobile_custom_url_schemes,
    get_site_url, oauth_state_key, sanitize_redirect_path, validate_mobile_redirect_to,
};
use super::{
    LegacyOAuthLoginQuery, LegacyOAuthMobileLoginQuery, OAuthLoginQuery, OAuthStatePayload,
    GITHUB_AUTH_URL, OAUTH_STATE_TTL_SECONDS,
};

pub async fn oauth_login(
    State(state): State<AppState>,
    Path(provider_key): Path<String>,
    Query(query): Query<OAuthLoginQuery>,
) -> Result<Redirect, AppError> {
    // Load provider config
    let config = OAuthRepository::new(&state.db)
        .get_active_sso_config_by_provider_key(&provider_key)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "OAuth provider '{}' not found or disabled",
                provider_key
            ))
        })?;

    let client_id = config.client_id.clone().ok_or_else(|| {
        AppError::BadRequest("OAuth provider client_id not configured".to_string())
    })?;

    let provider_type = SsoProviderType::from_str(&config.provider_type).ok_or_else(|| {
        AppError::Internal(format!("Unknown provider type: {}", config.provider_type))
    })?;

    // Generate state parameter
    let oauth_state = Uuid::new_v4().to_string();
    let is_mobile = query.mobile.unwrap_or(false);
    let redirect_after = sanitize_redirect_path(query.redirect_uri.clone());
    let mobile_redirect_to = if is_mobile {
        let app_custom_url_schemes = get_mobile_custom_url_schemes(&state.db).await;
        query
            .redirect_to
            .clone()
            .or_else(|| query.redirect_uri.clone())
            .as_deref()
            .map(|redirect_to| validate_mobile_redirect_to(redirect_to, &app_custom_url_schemes))
            .transpose()?
    } else {
        None
    };

    // Generate PKCE and nonce for OIDC providers
    let (code_verifier, code_challenge, nonce) = match provider_type {
        SsoProviderType::GitHub => (None, None, None),
        _ => {
            let verifier = generate_code_verifier();
            let challenge = generate_code_challenge(&verifier);
            (Some(verifier), Some(challenge), Some(generate_nonce()))
        }
    };

    let mobile_sso_state = query
        .state
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mobile_sso_code_challenge = query
        .code_challenge
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mobile_sso_code_challenge_method = query
        .code_challenge_method
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase());

    let state_payload = OAuthStatePayload {
        provider_key: provider_key.clone(),
        redirect_after,
        created_at: chrono::Utc::now().timestamp(),
        nonce: nonce.clone(),
        code_verifier: code_verifier.clone(),
        code_challenge_method: code_challenge.as_ref().map(|_| "S256".to_string()),
        is_mobile,
        mobile_sso_state,
        mobile_sso_code_challenge,
        mobile_sso_code_challenge_method,
        mobile_redirect_to,
    };

    // Store state in Redis
    let serialized_state = serde_json::to_string(&state_payload)
        .map_err(|e| AppError::Internal(format!("Failed to serialize OAuth state: {}", e)))?;

    let mut redis_conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("Redis connection failed: {}", e)))?;

    let _: () = redis_conn
        .set_ex(
            oauth_state_key(&oauth_state),
            serialized_state,
            OAUTH_STATE_TTL_SECONDS,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to store OAuth state: {}", e)))?;

    let callback_url = format!(
        "{}/api/v1/oauth2/{}/callback",
        get_site_url(&state.db).await,
        provider_key
    );
    let scopes = if config.scopes.is_empty() {
        provider_type.default_scopes()
    } else {
        config.scopes.clone()
    };
    let scope_str = scopes.join(" ");

    // Build authorization URL based on provider type
    let auth_url = match provider_type {
        SsoProviderType::GitHub => {
            format!(
                "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                GITHUB_AUTH_URL,
                urlencoding::encode(&client_id),
                urlencoding::encode(&callback_url),
                urlencoding::encode(&scope_str),
                oauth_state
            )
        }
        SsoProviderType::Google | SsoProviderType::Oidc => {
            let issuer = config.issuer_url.clone().ok_or_else(|| {
                AppError::BadRequest("OIDC provider issuer_url not configured".to_string())
            })?;

            // Use OIDC discovery to get authorization endpoint
            let discovery = OidcDiscoveryService::new();
            let discovery_result = discovery.discover(&issuer).await.map_err(|e| {
                AppError::Internal(format!("OIDC discovery failed for '{}': {}", issuer, e))
            })?;

            let mut url = format!(
                "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                discovery_result.authorization_endpoint,
                urlencoding::encode(&client_id),
                urlencoding::encode(&callback_url),
                urlencoding::encode(&scope_str),
                oauth_state
            );

            // Add PKCE code challenge
            if let Some(challenge) = code_challenge {
                url.push_str(&format!(
                    "&code_challenge={}&code_challenge_method=S256",
                    urlencoding::encode(&challenge)
                ));
            }

            // Add nonce for ID token validation
            if let Some(n) = nonce {
                url.push_str(&format!("&nonce={}", urlencoding::encode(&n)));
            }

            url
        }
        SsoProviderType::Saml => {
            return Err(AppError::BadRequest(
                "SAML is not supported via OAuth endpoints".to_string(),
            ));
        }
    };

    Ok(Redirect::temporary(&auth_url))
}

pub async fn legacy_oauth_login(
    State(state): State<AppState>,
    Path(service): Path<String>,
    Query(query): Query<LegacyOAuthLoginQuery>,
) -> ApiResult<Redirect> {
    let provider_key = resolve_legacy_service_provider_key(&state, &service).await?;

    let mut params = Vec::new();
    if let Some(redirect_to) = query.redirect_to.as_deref() {
        let trimmed = redirect_to.trim();
        if !trimmed.is_empty() {
            params.push(format!("redirect_uri={}", urlencoding::encode(trimmed)));
        }
    }

    let target = if params.is_empty() {
        format!("/api/v1/oauth2/{provider_key}/login")
    } else {
        format!("/api/v1/oauth2/{provider_key}/login?{}", params.join("&"))
    };

    Ok(Redirect::temporary(&target))
}

pub async fn legacy_oauth_mobile_login(
    State(state): State<AppState>,
    Path(service): Path<String>,
    Query(query): Query<LegacyOAuthMobileLoginQuery>,
) -> ApiResult<Redirect> {
    let provider_key = resolve_legacy_service_provider_key(&state, &service).await?;
    let app_custom_url_schemes = get_mobile_custom_url_schemes(&state.db).await;

    let mut params = vec!["mobile=true".to_string()];
    if let Some(redirect_to) = query.redirect_to.as_deref() {
        let validated = validate_mobile_redirect_to(redirect_to, &app_custom_url_schemes)?;
        params.push(format!("redirect_to={}", urlencoding::encode(&validated)));
    }
    if let Some(state_value) = query.state.as_deref() {
        let trimmed = state_value.trim();
        if !trimmed.is_empty() {
            params.push(format!("state={}", urlencoding::encode(trimmed)));
        }
    }
    if let Some(code_challenge) = query.code_challenge.as_deref() {
        let trimmed = code_challenge.trim();
        if !trimmed.is_empty() {
            params.push(format!("code_challenge={}", urlencoding::encode(trimmed)));
        }
    }
    if let Some(method) = query.code_challenge_method.as_deref() {
        let trimmed = method.trim();
        if !trimmed.is_empty() {
            params.push(format!(
                "code_challenge_method={}",
                urlencoding::encode(trimmed)
            ));
        }
    }

    let target = format!("/api/v1/oauth2/{provider_key}/login?{}", params.join("&"));
    Ok(Redirect::temporary(&target))
}

async fn resolve_legacy_service_provider_key(state: &AppState, service: &str) -> ApiResult<String> {
    use crate::repositories::oauth_repository::LegacyProviderRow;

    let normalized = service.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AppError::BadRequest("Invalid OAuth service".to_string()));
    }

    let providers: Vec<LegacyProviderRow> = OAuthRepository::new(&state.db)
        .list_legacy_providers()
        .await?;

    if let Some(exact) = providers.iter().find(|provider| {
        provider.provider_key.eq_ignore_ascii_case(&normalized)
            || provider.provider.eq_ignore_ascii_case(&normalized)
    }) {
        return Ok(exact.provider_key.clone());
    }

    let mapped_provider_type = match normalized.as_str() {
        "google" => Some("google"),
        "github" => Some("github"),
        "gitlab" | "office365" | "openid" => Some("oidc"),
        _ => None,
    };

    if let Some(provider_type) = mapped_provider_type {
        if provider_type == "oidc" {
            if let Some(preferred) = providers.iter().find(|provider| {
                provider.provider_type == "oidc"
                    && provider.provider_key == state.config.keycloak_sync.provider_key
            }) {
                return Ok(preferred.provider_key.clone());
            }
        }

        if let Some(first_match) = providers
            .iter()
            .find(|provider| provider.provider_type == provider_type)
        {
            return Ok(first_match.provider_key.clone());
        }
    }

    Err(AppError::NotFound(format!(
        "OAuth provider '{}' not found or disabled",
        service
    )))
}

/// List available OAuth providers for login
pub async fn list_providers(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<OAuthProviderInfo>>> {
    // Check if SSO is enabled globally
    let sso_enabled = OAuthRepository::new(&state.db)
        .get_authentication_config()
        .await?
        .and_then(|auth| auth.get("enable_sso").and_then(|v| v.as_bool()))
        .unwrap_or(false);

    if !sso_enabled {
        return Ok(Json(vec![]));
    }

    // Query active SSO configs
    let configs = OAuthRepository::new(&state.db)
        .list_active_sso_configs()
        .await?;

    let site_url = get_site_url(&state.db).await;
    let providers: Vec<OAuthProviderInfo> = configs
        .into_iter()
        .map(|c| {
            let display_name =
                c.display_name
                    .clone()
                    .unwrap_or_else(|| match c.provider_type.as_str() {
                        "github" => "GitHub".to_string(),
                        "google" => "Google".to_string(),
                        "oidc" => "SSO".to_string(),
                        _ => c.provider_key.clone(),
                    });

            OAuthProviderInfo {
                id: c.id.to_string(),
                provider_key: c.provider_key.clone(),
                provider_type: c.provider_type.clone(),
                display_name,
                login_url: format!("{}/api/v1/oauth2/{}/login", site_url, c.provider_key),
            }
        })
        .collect();

    Ok(Json(providers))
}
