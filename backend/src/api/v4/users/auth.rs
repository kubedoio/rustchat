use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use serde::Deserialize;

use super::parse_request_body;
use crate::api::AppState;
use crate::auth::{create_token_with_policy, verify_password};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::models as mm;
use crate::middleware::rate_limit::{self, RateLimitConfig};
use crate::models::User;
use crate::services::auth_config;
use crate::services::oauth_token_exchange::{exchange_code_with_sso_verification, ExchangeError};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub login_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub password: String,
    #[allow(dead_code)]
    pub device_id: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginTypeRequest {
    #[allow(dead_code)]
    pub id: Option<String>,
    #[allow(dead_code)]
    pub login_id: Option<String>,
    #[allow(dead_code)]
    pub device_id: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginSwitchRequest {
    #[allow(dead_code)]
    pub current_service: Option<String>,
    #[allow(dead_code)]
    pub new_service: Option<String>,
    #[allow(dead_code)]
    pub email: Option<String>,
    #[allow(dead_code)]
    pub password: Option<String>,
    #[allow(dead_code)]
    pub mfa_code: Option<String>,
    #[allow(dead_code)]
    pub ldap_id: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginCwsRequest {
    #[allow(dead_code)]
    pub login_id: Option<String>,
    #[allow(dead_code)]
    pub cws_token: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginSsoCodeExchangeRequest {
    #[allow(dead_code)]
    pub login_code: Option<String>,
    #[allow(dead_code)]
    pub code_verifier: Option<String>,
    #[allow(dead_code)]
    pub state: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<impl IntoResponse> {
    let input = parse_login_request(&headers, &body)?;
    let login_id = input
        .login_id
        .or(input.email)
        .ok_or_else(|| AppError::BadRequest("Missing login_id".to_string()))?;

    let user: Option<User> = sqlx::query_as(
        "SELECT * FROM users WHERE (email = $1 OR username = $1) AND is_active = true AND deleted_at IS NULL",
    )
    .bind(&login_id)
    .fetch_optional(&state.db)
    .await?;

    let user =
        user.ok_or_else(|| AppError::Unauthorized("Invalid login credentials".to_string()))?;

    enforce_password_login_allowed(&state, &user.email).await?;

    if state.config.security.rate_limit_enabled {
        let config =
            RateLimitConfig::auth_per_minute(state.config.security.rate_limit_auth_per_minute);
        let user_key = format!("user:{}", user.id);
        let user_result = rate_limit::check_rate_limit(&state.redis, &config, &user_key).await?;
        if !user_result.allowed {
            tracing::warn!(
                user_id = %user.id,
                "Rate limit exceeded for v4 user login"
            );
            return Err(AppError::TooManyRequests(
                "Too many login attempts. Please try again later.".to_string(),
                Some(config.window_secs),
            ));
        }
    }

    // Verify password (OAuth users without password cannot login with password)
    let password_hash = user
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::Unauthorized("Please use SSO to login".to_string()))?;

    if !verify_password(&input.password, password_hash)? {
        return Err(AppError::Unauthorized(
            "Invalid login credentials".to_string(),
        ));
    }

    // Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    // Generate token
    let token = create_token_with_policy(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_issuer.as_deref(),
        state.jwt_audience.as_deref(),
        state.jwt_expiry_hours,
    )?;

    let mm_user: mm::User = user.into();

    let mut headers = axum::http::HeaderMap::new();
    let token_header = axum::http::HeaderValue::from_str(&token)
        .map_err(|_| AppError::Internal("Invalid token characters".into()))?;
    headers.insert("Token", token_header.clone());
    headers.insert("token", token_header);
    let auth_header = axum::http::HeaderValue::from_str(&format!("Token {}", token))
        .map_err(|_| AppError::Internal("Invalid token characters".into()))?;
    headers.insert(axum::http::header::AUTHORIZATION, auth_header);
    let max_age = state.jwt_expiry_hours.saturating_mul(3600);
    let cookie_header = axum::http::HeaderValue::from_str(&format!(
        "MMAUTHTOKEN={}; Path=/; Max-Age={}; HttpOnly{}; SameSite=Lax",
        token,
        max_age,
        if state.config.is_production() {
            "; Secure"
        } else {
            ""
        }
    ))
    .map_err(|_| AppError::Internal("Invalid cookie characters".into()))?;
    headers.insert(axum::http::header::SET_COOKIE, cookie_header);

    Ok((headers, Json(mm_user)))
}

pub async fn login_type(headers: HeaderMap, body: Bytes) -> ApiResult<Json<serde_json::Value>> {
    let _input: LoginTypeRequest = parse_request_body(&headers, &body)?;

    Ok(Json(serde_json::json!({
        "auth_service": ""
    })))
}

pub async fn login_cws(headers: HeaderMap, body: Bytes) -> ApiResult<Json<serde_json::Value>> {
    let _input: LoginCwsRequest = parse_request_body(&headers, &body)?;

    Err(AppError::BadRequest(
        "CWS login is not supported".to_string(),
    ))
}

pub async fn login_sso_code_exchange(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    if !state.config.compatibility.mobile_sso_code_exchange {
        return Err(AppError::BadRequest(
            "Mobile SSO code exchange is disabled".to_string(),
        ));
    }

    let input: LoginSsoCodeExchangeRequest = parse_request_body(&headers, &body)?;
    let code = input
        .login_code
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("Missing login_code".to_string()))?;
    let code_verifier = input
        .code_verifier
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("Missing code_verifier".to_string()))?;
    let exchange_state = input
        .state
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("Missing state".to_string()))?;

    let payload = match exchange_code_with_sso_verification(
        &state.redis,
        code,
        code_verifier,
        exchange_state,
    )
    .await
    {
        Ok(payload) => payload,
        Err(ExchangeError::InvalidCode) => {
            return Err(AppError::BadRequest(
                "Invalid or already used exchange code".to_string(),
            ));
        }
        Err(ExchangeError::CodeExpired) => {
            return Err(AppError::BadRequest(
                "Exchange code has expired".to_string(),
            ));
        }
        Err(ExchangeError::SsoVerificationRequired) => {
            return Err(AppError::BadRequest(
                "Exchange code is missing SSO verification metadata".to_string(),
            ));
        }
        Err(ExchangeError::StateMismatch) => {
            return Err(AppError::BadRequest("SSO state mismatch".to_string()));
        }
        Err(ExchangeError::ChallengeMismatch) => {
            return Err(AppError::BadRequest("SSO challenge mismatch".to_string()));
        }
        Err(ExchangeError::UnsupportedChallengeMethod) => {
            return Err(AppError::BadRequest(
                "Unsupported SSO challenge method".to_string(),
            ));
        }
        Err(ExchangeError::Internal(msg)) => {
            tracing::error!("v4 SSO code exchange failed: {}", msg);
            return Err(AppError::Internal(
                "Failed to process exchange code".to_string(),
            ));
        }
    };

    let token = create_token_with_policy(
        payload.user_id,
        &payload.email,
        &payload.role,
        payload.org_id,
        &state.jwt_secret,
        state.jwt_issuer.as_deref(),
        state.jwt_audience.as_deref(),
        state.jwt_expiry_hours,
    )?;

    Ok(Json(serde_json::json!({
        "token": token,
        "csrf": ""
    })))
}

pub async fn login_switch(headers: HeaderMap, body: Bytes) -> ApiResult<Json<serde_json::Value>> {
    let _input: LoginSwitchRequest = parse_request_body(&headers, &body)?;

    Err(AppError::BadRequest(
        "Login method switching is not supported".to_string(),
    ))
}

pub fn parse_login_request(headers: &HeaderMap, body: &Bytes) -> ApiResult<LoginRequest> {
    parse_request_body(headers, body)
}

pub async fn enforce_password_login_allowed(state: &AppState, user_email: &str) -> ApiResult<()> {
    let auth_config = auth_config::get_password_rules(&state.db).await?;
    if !auth_config.require_sso {
        return Ok(());
    }

    let email_lc = user_email.trim().to_ascii_lowercase();
    let allowed = auth_config
        .sso_break_glass_emails
        .iter()
        .any(|email| email.trim().eq_ignore_ascii_case(&email_lc));
    if allowed {
        return Ok(());
    }

    Err(AppError::BadRequest(
        "Password login is disabled because SSO is required".to_string(),
    ))
}
