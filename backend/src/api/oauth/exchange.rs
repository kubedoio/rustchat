use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::services::oauth_token_exchange::{exchange_code, ExchangeError};

use super::{ExchangeRequest, ExchangeResponse, OAUTH_EXCHANGE_COOKIE};
use super::utils::{clear_exchange_code_cookie, read_cookie_value};

/// Exchange a one-time code for a JWT token
pub async fn exchange_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ExchangeRequest>,
) -> ApiResult<impl IntoResponse> {
    let code = input
        .code
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .or_else(|| read_cookie_value(&headers, OAUTH_EXCHANGE_COOKIE))
        .ok_or_else(|| AppError::BadRequest("Missing exchange code".to_string()))?;

    // Validate code length to prevent unnecessary Redis calls
    if code.len() < 10 {
        return Err(AppError::BadRequest("Invalid exchange code".to_string()));
    }

    // Exchange the code for user data
    let payload = match exchange_code(&state.redis, &code).await {
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
                "Exchange code requires additional SSO verification".to_string(),
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
            tracing::error!("Exchange code error: {}", msg);
            return Err(AppError::Internal(
                "Failed to process exchange code".to_string(),
            ));
        }
    };

    // Generate JWT token
    let token = crate::auth::create_token_with_policy(
        payload.user_id,
        &payload.email,
        &payload.role,
        payload.org_id,
        &state.jwt_secret,
        state.jwt_issuer.as_deref(),
        state.jwt_audience.as_deref(),
        state.jwt_expiry_hours,
    )
    .map_err(|e| AppError::Internal(format!("Failed to create token: {}", e)))?;

    tracing::info!(
        user_id = %payload.user_id,
        email = %payload.email,
        "OAuth token exchanged successfully"
    );

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_exchange_code_cookie(state.config.is_production()))
            .map_err(|e| AppError::Internal(format!("Failed to clear exchange cookie: {}", e)))?,
    );

    Ok((
        response_headers,
        Json(ExchangeResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_expiry_hours * 3600,
        }),
    ))
}
