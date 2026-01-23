//! OAuth2/OIDC authentication handlers

use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppState;
use crate::error::{ApiResult, AppError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/oauth2/{provider}/login", get(oauth_login))
        .route("/oauth2/{provider}/callback", get(oauth_callback))
        .route("/oauth2/providers", get(list_providers))
}

/// List available OAuth providers
async fn list_providers(State(state): State<AppState>) -> ApiResult<Json<Vec<OAuthProvider>>> {
    // Query enabled SSO configs from DB
    let configs: Vec<SsoConfigRow> =
        sqlx::query_as("SELECT * FROM sso_configs WHERE enabled = true")
            .fetch_all(&state.db)
            .await?;

    let providers: Vec<OAuthProvider> = configs
        .into_iter()
        .map(|c| OAuthProvider {
            id: c.provider.clone(),
            name: c.display_name.unwrap_or(c.provider),
            icon_url: None,
        })
        .collect();

    Ok(Json(providers))
}

#[derive(Debug, Serialize)]
pub struct OAuthProvider {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct SsoConfigRow {
    _id: Uuid,
    provider: String,
    display_name: Option<String>,
    issuer_url: Option<String>,
    client_id: String,
    client_secret_encrypted: String,
    _enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct OAuthLoginQuery {
    pub redirect_uri: Option<String>,
}

/// Initiate OAuth login - redirects to provider
async fn oauth_login(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(query): Query<OAuthLoginQuery>,
) -> Result<Redirect, AppError> {
    // Get SSO config for provider
    let config: SsoConfigRow =
        sqlx::query_as("SELECT * FROM sso_configs WHERE provider = $1 AND enabled = true")
            .bind(&provider)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "OAuth provider '{}' not found or disabled",
                    provider
                ))
            })?;

    // Generate state parameter for CSRF protection
    let oauth_state = Uuid::new_v4().to_string();
    let _redirect_after = query.redirect_uri.unwrap_or_else(|| "/".to_string());

    // Store state in Redis with short TTL (5 min)
    // TODO: Implement Redis state storage
    // For now, we'll use a simple in-memory approach via query params

    // Build authorization URL
    let issuer = config
        .issuer_url
        .unwrap_or_else(|| format!("https://{}.example.com", provider));

    let callback_url = format!(
        "{}/api/v1/oauth2/{}/callback",
        std::env::var("RUSTCHAT_SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()),
        provider
    );

    let auth_url = format!(
        "{}/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20profile%20email&state={}",
        issuer,
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&callback_url),
        oauth_state
    );

    Ok(Redirect::temporary(&auth_url))
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub _state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: Option<u64>,
    id_token: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserInfoResponse {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    preferred_username: Option<String>,
    picture: Option<String>,
}

/// Handle OAuth callback from provider
async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Redirect, AppError> {
    // Check for errors from provider
    if let Some(error) = query.error {
        let desc = query.error_description.unwrap_or_else(|| error.clone());
        return Ok(Redirect::temporary(&format!(
            "/login?error={}",
            urlencoding::encode(&desc)
        )));
    }

    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest("Missing authorization code".to_string()))?;

    // Get SSO config
    let config: SsoConfigRow =
        sqlx::query_as("SELECT * FROM sso_configs WHERE provider = $1 AND enabled = true")
            .bind(&provider)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("OAuth provider '{}' not found", provider))
            })?;

    let issuer = config
        .issuer_url
        .unwrap_or_else(|| format!("https://{}.example.com", provider));

    let callback_url = format!(
        "{}/api/v1/oauth2/{}/callback",
        std::env::var("RUSTCHAT_SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()),
        provider
    );

    // Exchange code for tokens
    let client = reqwest::Client::new();
    let token_url = format!("{}/token", issuer);

    let token_response = client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &callback_url),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret_encrypted), // TODO: decrypt
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Token exchange failed: {}", e)))?;

    if !token_response.status().is_success() {
        let error_text = token_response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Token exchange failed: {}",
            error_text
        )));
    }

    let tokens: TokenResponse = token_response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse token response: {}", e)))?;

    // Get user info
    let userinfo_url = format!("{}/userinfo", issuer);
    let userinfo_response: reqwest::Response = client
        .get(&userinfo_url)
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Userinfo request failed: {}", e)))?;

    let userinfo: UserInfoResponse = userinfo_response
        .json::<UserInfoResponse>()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse userinfo: {}", e)))?;

    // Find or create user
    let email = userinfo
        .email
        .ok_or_else(|| AppError::BadRequest("Email not provided by OAuth provider".to_string()))?;

    let user: Option<crate::models::User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.db)
        .await?;

    let user = match user {
        Some(u) => u,
        None => {
            // Create new user from OAuth info
            let username = userinfo
                .preferred_username
                .or(userinfo.name.clone())
                .unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());

            sqlx::query_as(
                r#"
                INSERT INTO users (username, email, display_name, role, is_active, auth_provider)
                VALUES ($1, $2, $3, 'member', true, $4)
                ON CONFLICT (email) DO UPDATE SET last_login_at = NOW()
                RETURNING *
                "#,
            )
            .bind(&username)
            .bind(&email)
            .bind(&userinfo.name)
            .bind(&provider)
            .fetch_one(&state.db)
            .await?
        }
    };

    // Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    // Generate JWT token
    let token = crate::auth::create_token(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_expiry_hours,
    )?;

    // Redirect to frontend with token
    Ok(Redirect::temporary(&format!(
        "/oauth/callback?token={}",
        token
    )))
}
