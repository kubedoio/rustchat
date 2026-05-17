use axum::http::{header, HeaderMap};
use std::time::Duration;

use crate::error::{ApiResult, AppError};
use crate::middleware::reliability::{send_reqwest_with_retry, RetryCondition, RetryConfig};
use crate::repositories::OAuthRepository;

use super::{
    DEFAULT_APP_CUSTOM_URL_SCHEMES, DEFAULT_OAUTH_REDIRECT_PATH,
    OAUTH_EXCHANGE_COOKIE, OAUTH_EXCHANGE_COOKIE_MAX_AGE_SECONDS, OAUTH_STATE_PREFIX,
};

/// Generate Redis key for OAuth state
pub fn oauth_state_key(state: &str) -> String {
    format!("{}{}", OAUTH_STATE_PREFIX, state)
}

/// Sanitize redirect path - only allow relative paths starting with /
pub fn sanitize_redirect_path(redirect_uri: Option<String>) -> String {
    match redirect_uri {
        Some(path) => {
            // URL-decode before validation to catch encoded bypasses like %2e%2e or %2f%2f
            let decoded = urlencoding::decode(&path).unwrap_or_else(|_| path.into());
            let decoded = decoded.as_ref();
            // Must start with / and not be // or contain ..
            if decoded.starts_with('/')
                && !decoded.starts_with("//")
                && !decoded.contains("..")
                && !decoded.contains('\0')
            {
                path
            } else {
                DEFAULT_OAUTH_REDIRECT_PATH.to_string()
            }
        }
        _ => DEFAULT_OAUTH_REDIRECT_PATH.to_string(),
    }
}

pub fn append_query_param(path: &str, key: &str, value: &str) -> String {
    let encoded_value = urlencoding::encode(value);
    if path.contains('?') {
        format!("{}&{}={}", path, key, encoded_value)
    } else {
        format!("{}?{}={}", path, key, encoded_value)
    }
}

pub fn read_cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE).and_then(|v| v.to_str().ok())?;
    cookie_header.split(';').find_map(|pair| {
        let mut parts = pair.trim().splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key == name && !value.is_empty() {
            Some(value.to_string())
        } else {
            None
        }
    })
}

pub fn build_exchange_code_cookie(code: &str, secure: bool) -> String {
    format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{}",
        OAUTH_EXCHANGE_COOKIE,
        code,
        OAUTH_EXCHANGE_COOKIE_MAX_AGE_SECONDS,
        if secure { "; Secure" } else { "" }
    )
}

pub fn clear_exchange_code_cookie(secure: bool) -> String {
    format!(
        "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{}",
        OAUTH_EXCHANGE_COOKIE,
        if secure { "; Secure" } else { "" }
    )
}

pub async fn send_with_retry(
    request: reqwest::RequestBuilder,
    context: &'static str,
) -> Result<reqwest::Response, AppError> {
    let retry_config = RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(150),
        max_delay: Duration::from_secs(2),
        backoff_multiplier: 2.0,
        retry_if: RetryCondition::Default,
    };

    send_reqwest_with_retry(
        request,
        &retry_config,
        move |e| AppError::ExternalService(format!("{}: {}", context, e)),
        move || AppError::Internal(format!("Failed to clone request builder: {}", context)),
    )
    .await
}

/// Generate PKCE code verifier (43-128 chars per RFC 7636)
pub fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::thread_rng();
    (0..128)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

/// Generate PKCE code challenge from verifier (S256 method)
pub fn generate_code_challenge(verifier: &str) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use sha2::{Digest, Sha256};

    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generate nonce for OIDC
pub fn generate_nonce() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

/// Get site URL from database (server config) with fallback to environment variable
pub async fn get_site_url(db: &sqlx::PgPool) -> String {
    let db_url = OAuthRepository::new(db)
        .get_site_config()
        .await
        .ok()
        .flatten()
        .and_then(|site| {
            let url = site.site_url;
            if url.is_empty() { None } else { Some(url) }
        });

    // Fall back to environment variable if not set in database
    db_url.unwrap_or_else(|| {
        std::env::var("RUSTCHAT_SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
    })
}

pub fn validate_mobile_redirect_to(redirect_to: &str, allowed_schemes: &[String]) -> ApiResult<String> {
    let trimmed = redirect_to.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(
            "Invalid mobile redirect URL".to_string(),
        ));
    }

    let parsed = url::Url::parse(trimmed)
        .map_err(|_| AppError::BadRequest("Invalid mobile redirect URL".to_string()))?;

    let normalized = trimmed.to_ascii_lowercase();
    let effective_schemes: Vec<String> = if allowed_schemes.is_empty() {
        DEFAULT_APP_CUSTOM_URL_SCHEMES
            .iter()
            .map(|value| value.to_string())
            .collect()
    } else {
        allowed_schemes.to_vec()
    };

    let is_allowed_scheme = effective_schemes
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .any(|value| normalized.starts_with(&value.to_ascii_lowercase()));

    if !is_allowed_scheme {
        return Err(AppError::BadRequest(
            "Invalid mobile redirect URL scheme".to_string(),
        ));
    }

    if parsed.host_str().unwrap_or_default() != "callback"
        || !parsed.path().is_empty()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AppError::BadRequest(
            "Invalid mobile redirect callback host".to_string(),
        ));
    }

    Ok(trimmed.to_string())
}

pub async fn get_mobile_custom_url_schemes(db: &sqlx::PgPool) -> Vec<String> {
    let config = OAuthRepository::new(db)
        .get_site_config()
        .await
        .ok()
        .flatten();

    let schemes = config.map(|site| site.app_custom_url_schemes).unwrap_or_default();

    if schemes.is_empty() {
        DEFAULT_APP_CUSTOM_URL_SCHEMES
            .iter()
            .map(|value| value.to_string())
            .collect()
    } else {
        schemes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_cookie_value_extracts_named_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            axum::http::HeaderValue::from_static("foo=bar; RCOAUTHCODE=abc123; baz=qux"),
        );

        let value = read_cookie_value(&headers, "RCOAUTHCODE");
        assert_eq!(value.as_deref(), Some("abc123"));
    }

    #[test]
    fn read_cookie_value_returns_none_when_missing() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, axum::http::HeaderValue::from_static("foo=bar; baz=qux"));

        let value = read_cookie_value(&headers, "RCOAUTHCODE");
        assert!(value.is_none());
    }

    #[test]
    fn exchange_cookie_builders_include_security_attributes() {
        let set_cookie = build_exchange_code_cookie("code123", true);
        assert!(set_cookie.contains("RCOAUTHCODE=code123"));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Lax"));
        assert!(set_cookie.contains("Secure"));

        let clear_cookie = clear_exchange_code_cookie(true);
        assert!(clear_cookie.contains("RCOAUTHCODE="));
        assert!(clear_cookie.contains("Max-Age=0"));
    }
}
