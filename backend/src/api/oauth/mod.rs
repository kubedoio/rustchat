//! OAuth2/OIDC authentication handlers
//!
//! Supports three provider types:
//! - github: OAuth2 with GitHub (no OIDC discovery)
//! - google: OIDC with discovery
//! - oidc: Generic OIDC with discovery (Keycloak, ZITADEL, Authentik, etc.)

mod callback;
mod exchange;
mod login;
mod providers;
mod utils;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::api::AppState;

/// Request to exchange a code for a token
#[derive(Debug, serde::Deserialize)]
pub struct ExchangeRequest {
    #[serde(default)]
    pub code: Option<String>,
}

/// Response containing the JWT token
#[derive(Debug, serde::Serialize)]
pub struct ExchangeResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

pub(crate) const OAUTH_STATE_PREFIX: &str = "rustchat:oauth:state:";
pub(crate) const OAUTH_STATE_TTL_SECONDS: u64 = 300; // 5 minutes
pub(crate) const OAUTH_EXCHANGE_COOKIE: &str = "RCOAUTHCODE";
pub(crate) const OAUTH_EXCHANGE_COOKIE_MAX_AGE_SECONDS: u64 = 120;
pub(crate) const DEFAULT_OAUTH_REDIRECT_PATH: &str = "/";
pub(crate) const DEFAULT_APP_CUSTOM_URL_SCHEMES: [&str; 2] = ["mmauth://", "mmauthbeta://"];

// GitHub OAuth endpoints (no OIDC discovery)
pub(crate) const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
pub(crate) const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub(crate) const GITHUB_API_URL: &str = "https://api.github.com";

/// State parameter stored in Redis
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OAuthStatePayload {
    pub(crate) provider_key: String,
    pub(crate) redirect_after: String,
    pub(crate) created_at: i64,
    // OIDC-specific fields
    pub(crate) nonce: Option<String>,
    // PKCE
    pub(crate) code_verifier: Option<String>,
    pub(crate) code_challenge_method: Option<String>,
    // Mobile app flag
    pub(crate) is_mobile: bool,
    // Mobile SSO code exchange challenge values from client.
    #[serde(default)]
    pub(crate) mobile_sso_state: Option<String>,
    #[serde(default)]
    pub(crate) mobile_sso_code_challenge: Option<String>,
    #[serde(default)]
    pub(crate) mobile_sso_code_challenge_method: Option<String>,
    #[serde(default)]
    pub(crate) mobile_redirect_to: Option<String>,
}

/// OAuth callback query parameters
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// OAuth login query parameters
#[derive(Debug, Deserialize)]
pub struct OAuthLoginQuery {
    pub redirect_uri: Option<String>,
    pub redirect_to: Option<String>,
    pub mobile: Option<bool>, // If true, redirect to mobile app scheme instead of web
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LegacyOAuthLoginQuery {
    redirect_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LegacyOAuthMobileLoginQuery {
    redirect_to: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

/// User info extracted from OAuth provider
pub(crate) struct UserInfo {
    pub(crate) email: String,
    pub(crate) name: Option<String>,
    pub(crate) preferred_username: Option<String>,
    pub(crate) groups: Vec<String>,
    pub(crate) external_id: Option<String>, // Provider's user ID (e.g., Google 'sub', GitHub 'id')
}

pub fn router(state: AppState) -> Router<AppState> {
    let auth_routes = Router::new()
        .route("/oauth2/{provider_key}/login", get(login::oauth_login))
        .route("/oauth2/{provider_key}/callback", get(callback::oauth_callback))
        .route("/oauth2/exchange", post(exchange::exchange_token))
        .layer(middleware::from_fn_with_state(
            state,
            crate::middleware::rate_limit::auth_ip_rate_limit,
        ));

    Router::new()
        .merge(auth_routes)
        .route("/oauth2/providers", get(login::list_providers))
}

pub fn web_compat_router() -> Router<AppState> {
    Router::new()
        .route("/oauth/{service}/login", get(login::legacy_oauth_login))
        .route(
            "/oauth/{service}/mobile_login",
            get(login::legacy_oauth_mobile_login),
        )
}
