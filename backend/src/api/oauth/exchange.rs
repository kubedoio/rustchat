use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};

use crate::api::AppState;
use crate::error::AppError;
use crate::services::oauth_token_exchange::{exchange_code, ExchangeError};

use super::{ExchangeRequest, ExchangeResponse, OAUTH_EXCHANGE_COOKIE};
use super::utils::{clear_exchange_code_cookie, read_cookie_value};

/// Exchange a one-time code for a JWT token
pub async fn exchange_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ExchangeRequest>,
) -> Result<Response, AppError> {
    let clear_cookie = HeaderValue::from_str(&clear_exchange_code_cookie(state.config.is_production()))
        .map_err(|e| AppError::Internal(format!("Failed to clear exchange cookie: {}", e)))?;

    let code = match input
        .code
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .or_else(|| read_cookie_value(&headers, OAUTH_EXCHANGE_COOKIE))
    {
        Some(code) => code,
        None => {
            return Ok(error_with_clear_cookie(
                AppError::BadRequest("Missing exchange code".to_string()),
                clear_cookie,
            ));
        }
    };

    // Validate code length to prevent unnecessary Redis calls
    if code.len() < 10 {
        return Ok(error_with_clear_cookie(
            AppError::BadRequest("Invalid exchange code".to_string()),
            clear_cookie,
        ));
    }

    // Exchange the code for user data
    let payload = match exchange_code(&state.redis, &code).await {
        Ok(payload) => payload,
        Err(ExchangeError::InvalidCode) => {
            return Ok(error_with_clear_cookie(
                AppError::BadRequest("Invalid or already used exchange code".to_string()),
                clear_cookie,
            ));
        }
        Err(ExchangeError::CodeExpired) => {
            return Ok(error_with_clear_cookie(
                AppError::BadRequest("Exchange code has expired".to_string()),
                clear_cookie,
            ));
        }
        Err(ExchangeError::SsoVerificationRequired) => {
            return Ok(error_with_clear_cookie(
                AppError::BadRequest(
                    "Exchange code requires additional SSO verification".to_string(),
                ),
                clear_cookie,
            ));
        }
        Err(ExchangeError::StateMismatch) => {
            return Ok(error_with_clear_cookie(
                AppError::BadRequest("SSO state mismatch".to_string()),
                clear_cookie,
            ));
        }
        Err(ExchangeError::ChallengeMismatch) => {
            return Ok(error_with_clear_cookie(
                AppError::BadRequest("SSO challenge mismatch".to_string()),
                clear_cookie,
            ));
        }
        Err(ExchangeError::UnsupportedChallengeMethod) => {
            return Ok(error_with_clear_cookie(
                AppError::BadRequest("Unsupported SSO challenge method".to_string()),
                clear_cookie,
            ));
        }
        Err(ExchangeError::Internal(msg)) => {
            tracing::error!("Exchange code error: {}", msg);
            return Ok(error_with_clear_cookie(
                AppError::Internal("Failed to process exchange code".to_string()),
                clear_cookie,
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
    response_headers.insert(header::SET_COOKIE, clear_cookie);

    Ok((
        response_headers,
        Json(ExchangeResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_expiry_hours * 3600,
        }),
    )
        .into_response())
}

fn error_with_clear_cookie(error: AppError, clear_cookie: HeaderValue) -> Response {
    let mut response = error.into_response();
    response.headers_mut().insert(header::SET_COOKIE, clear_cookie);
    response
}
