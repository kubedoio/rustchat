//! Cloudflare Turnstile verification service
//!
//! Provides bot protection for public forms like registration and password reset.

use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};

use crate::middleware::reliability::{
    with_resilience, CircuitBreaker, CircuitError, RetryCondition, RetryConfig,
};

// ponytail: use standard library LazyLock instead of once_cell
static TURNSTILE_CIRCUIT_BREAKER: std::sync::LazyLock<std::sync::Arc<CircuitBreaker>> =
    std::sync::LazyLock::new(|| CircuitBreaker::default_config("turnstile"));
static TURNSTILE_RETRY_CONFIG: std::sync::LazyLock<RetryConfig> =
    std::sync::LazyLock::new(|| RetryConfig {
        retry_if: RetryCondition::Default,
        ..Default::default()
    });

/// Turnstile verification error
#[derive(Debug, Clone, PartialEq)]
pub enum TurnstileError {
    InvalidToken,
    ExpiredToken,
    InvalidSecretKey,
    BadRequest,
    Timeout,
    UnknownHost,
    InternalError,
}

impl std::fmt::Display for TurnstileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnstileError::InvalidToken => write!(f, "Invalid verification token"),
            TurnstileError::ExpiredToken => write!(f, "Verification token expired"),
            TurnstileError::InvalidSecretKey => write!(f, "Invalid secret key"),
            TurnstileError::BadRequest => write!(f, "Bad request"),
            TurnstileError::Timeout => write!(f, "Verification timeout"),
            TurnstileError::UnknownHost => write!(f, "Unknown host"),
            TurnstileError::InternalError => write!(f, "Internal verification error"),
        }
    }
}

impl std::error::Error for TurnstileError {}

/// Turnstile verification request
#[derive(Debug, Serialize)]
struct VerifyRequest<'a> {
    secret: &'a str,
    response: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    remoteip: Option<&'a str>,
}

/// Turnstile verification response
#[derive(Debug, Deserialize)]
struct VerifyResponse {
    success: bool,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

/// Verify a Turnstile token
pub async fn verify_token(
    secret_key: &str,
    token: &str,
    remote_ip: Option<&str>,
) -> Result<(), TurnstileError> {
    if secret_key.is_empty() {
        warn!("Turnstile secret key not configured");
        return Err(TurnstileError::InvalidSecretKey);
    }

    if token.is_empty() {
        debug!("Empty Turnstile token");
        return Err(TurnstileError::InvalidToken);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| {
            error!("Failed to build Turnstile HTTP client: {}", e);
            TurnstileError::InternalError
        })?;
    let secret = secret_key.to_string();
    let response_token = token.to_string();
    let remote_ip_owned = remote_ip.map(|ip| ip.to_string());

    let response = with_resilience(&TURNSTILE_CIRCUIT_BREAKER, &TURNSTILE_RETRY_CONFIG, {
        let client = client.clone();
        let secret = secret.clone();
        let response_token = response_token.clone();
        let remote_ip_owned = remote_ip_owned.clone();
        move || {
            let client = client.clone();
            let secret = secret.clone();
            let response_token = response_token.clone();
            let remote_ip_owned = remote_ip_owned.clone();
            async move {
                let request = VerifyRequest {
                    secret: &secret,
                    response: &response_token,
                    remoteip: remote_ip_owned.as_deref(),
                };
                client
                    .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
                    .form(&request)
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Turnstile verification request failed: {}", e);
                        TurnstileError::InternalError
                    })
            }
        }
    })
    .await
    .map_err(map_circuit_error)?;

    if !response.status().is_success() {
        error!(
            "Turnstile returned non-success status: {}",
            response.status()
        );
        return Err(TurnstileError::InternalError);
    }

    let result: VerifyResponse = response.json().await.map_err(|e| {
        error!("Failed to parse Turnstile response: {}", e);
        TurnstileError::InternalError
    })?;

    if result.success {
        debug!("Turnstile verification successful");
        Ok(())
    } else {
        let error_codes = result.error_codes.unwrap_or_default();
        warn!("Turnstile verification failed: {:?}", error_codes);

        // Map first error code to our error type
        Err(error_codes
            .into_iter()
            .next()
            .map_or(TurnstileError::InvalidToken, |code| match code.as_str() {
                "bad-request" => TurnstileError::BadRequest,
                "timeout-or-duplicate" => TurnstileError::Timeout,
                "invalid-input-secret" => TurnstileError::InvalidSecretKey,
                "invalid-input-response" => TurnstileError::InvalidToken,
                _ => TurnstileError::InvalidToken,
            }))
    }
}

/// Check if Turnstile is properly configured
pub fn is_configured(secret_key: &str) -> bool {
    !secret_key.trim().is_empty()
}

fn map_circuit_error(err: CircuitError<TurnstileError>) -> TurnstileError {
    match err {
        CircuitError::Open => {
            error!("Turnstile circuit breaker is open");
            TurnstileError::Timeout
        }
        CircuitError::Inner(inner) => inner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turnstile_error_display() {
        assert_eq!(
            TurnstileError::InvalidToken.to_string(),
            "Invalid verification token"
        );
        assert_eq!(
            TurnstileError::ExpiredToken.to_string(),
            "Verification token expired"
        );
    }

    #[test]
    fn test_is_configured() {
        assert!(!is_configured(""));
        assert!(!is_configured("   "));
        assert!(is_configured("1x0000000000000000000000000000000AA"));
    }
}
