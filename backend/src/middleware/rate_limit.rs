//! Rate limiting middleware
//!
//! ## Architecture
//!
//! Rate limiting in RustChat operates at two levels:
//!
//! ### 1. Entity-Level Rate Limiting (Implemented)
//! Authenticated requests (API keys) are rate limited per entity using Redis-backed
//! atomic counters. This is handled by:
//! - `services/rate_limit.rs` - RateLimitService with Lua scripts
//! - `auth/extractors.rs` - ApiKeyAuth and PolymorphicAuth call RateLimitService
//!
//! Tiers:
//! - HumanStandard: 1k req/hr
//! - AgentHigh: 10k req/hr
//! - ServiceUnlimited: no limit
//! - CIStandard: 5k req/hr
//!
//! ### 2. IP-Based Rate Limiting (Implemented)
//! Unauthenticated endpoints (login, registration, password reset) are rate limited
//! by IP address using Redis-backed fixed window counters.

use std::net::SocketAddr;

use crate::error::AppError;
use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use deadpool_redis::redis::AsyncCommands;

/// Extract the client IP from common proxy headers or connection info.
fn extract_client_ip(request: &Request) -> Option<String> {
    if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(ip) = s.split(',').next() {
                let ip = ip.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(s) = real_ip.to_str() {
            let ip = s.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }

    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
}

/// Check an IP-based rate limit using a fixed window counter in Redis.
async fn check_ip_rate_limit(
    redis: &deadpool_redis::Pool,
    action: &str,
    ip: &str,
    window_secs: u64,
    max_requests: u64,
) -> Result<bool, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("Redis connection error: {}", e)))?;

    let now = chrono::Utc::now().timestamp() as u64;
    let window = now / window_secs;
    let redis_key = format!("ratelimit:ip:{}:{}:{}", action, ip, window);

    let count: u64 = conn.incr(&redis_key, 1u64).await.map_err(AppError::Redis)?;
    let _: () = conn
        .expire(&redis_key, window_secs as i64)
        .await
        .map_err(AppError::Redis)?;

    Ok(count <= max_requests)
}

/// Rate limit middleware for registration endpoints
///
/// Limits registration attempts to 5 requests per 15 minutes per IP.
pub async fn register_ip_rate_limit(
    State(state): State<crate::api::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !state.config.security.rate_limit_enabled {
        return Ok(next.run(request).await);
    }

    const WINDOW_SECS: u64 = 15 * 60;
    const MAX_REQUESTS: u64 = 5;

    let ip = match extract_client_ip(&request) {
        Some(ip) => ip,
        None => {
            tracing::warn!("Unable to determine client IP for registration rate limiting");
            return Ok(next.run(request).await);
        }
    };

    if !check_ip_rate_limit(&state.redis, "register", &ip, WINDOW_SECS, MAX_REQUESTS).await? {
        return Err(AppError::TooManyRequests(
            "Too many registration attempts. Please try again later.".to_string(),
        ));
    }

    Ok(next.run(request).await)
}

/// Rate limit middleware for auth endpoints
///
/// Limits auth attempts to 10 requests per 15 minutes per IP.
pub async fn auth_ip_rate_limit(
    State(state): State<crate::api::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !state.config.security.rate_limit_enabled {
        return Ok(next.run(request).await);
    }

    const WINDOW_SECS: u64 = 15 * 60;
    const MAX_REQUESTS: u64 = 10;

    let ip = match extract_client_ip(&request) {
        Some(ip) => ip,
        None => {
            tracing::warn!("Unable to determine client IP for auth rate limiting");
            return Ok(next.run(request).await);
        }
    };

    if !check_ip_rate_limit(&state.redis, "auth", &ip, WINDOW_SECS, MAX_REQUESTS).await? {
        return Err(AppError::TooManyRequests(
            "Too many authentication attempts. Please try again later.".to_string(),
        ));
    }

    Ok(next.run(request).await)
}

/// Rate limit middleware for password reset endpoints
///
/// Limits password reset attempts to 3 requests per 15 minutes per IP.
pub async fn password_reset_ip_rate_limit(
    State(state): State<crate::api::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !state.config.security.rate_limit_enabled {
        return Ok(next.run(request).await);
    }

    const WINDOW_SECS: u64 = 15 * 60;
    const MAX_REQUESTS: u64 = 3;

    let ip = match extract_client_ip(&request) {
        Some(ip) => ip,
        None => {
            tracing::warn!("Unable to determine client IP for password reset rate limiting");
            return Ok(next.run(request).await);
        }
    };

    if !check_ip_rate_limit(
        &state.redis,
        "password_reset",
        &ip,
        WINDOW_SECS,
        MAX_REQUESTS,
    )
    .await?
    {
        return Err(AppError::TooManyRequests(
            "Too many password reset attempts. Please try again later.".to_string(),
        ));
    }

    Ok(next.run(request).await)
}

/// Rate limit middleware for WebSocket endpoints
///
/// Limits WebSocket connection attempts to 20 connections per minute per IP.
pub async fn websocket_ip_rate_limit(
    State(state): State<crate::api::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !state.config.security.rate_limit_enabled {
        return Ok(next.run(request).await);
    }

    const WINDOW_SECS: u64 = 60;
    const MAX_REQUESTS: u64 = 20;

    let ip = match extract_client_ip(&request) {
        Some(ip) => ip,
        None => {
            tracing::warn!("Unable to determine client IP for websocket rate limiting");
            return Ok(next.run(request).await);
        }
    };

    if !check_ip_rate_limit(&state.redis, "websocket", &ip, WINDOW_SECS, MAX_REQUESTS).await? {
        return Err(AppError::TooManyRequests(
            "Too many WebSocket connection attempts. Please try again later.".to_string(),
        ));
    }

    Ok(next.run(request).await)
}

// ============================================================================
// Legacy API - Kept for backward compatibility
// ============================================================================
// These types and functions are used by existing code (src/api/auth.rs, src/api/v4/users.rs).
// `check_rate_limit` provides per-account sliding window rate limiting.
// IP-level rate limiting is handled by the middleware functions above.

/// Per-account sliding-window rate limit configuration for auth endpoints
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub window_secs: u64,
    pub max_requests: u64,
}

impl RateLimitConfig {
    /// Create config for auth endpoints
    pub fn auth_per_minute(max_requests: u32) -> Self {
        Self {
            window_secs: 60,
            max_requests: max_requests as u64,
        }
    }
}

/// Rate limit check result for auth endpoints
#[derive(Debug, Clone, Copy)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u64,
    pub reset_at: u64,
}

/// Per-account rate limit check using a Redis sliding window.
///
/// Used by login handlers to throttle individual accounts independently of
/// IP-based limits enforced by the middleware above.
pub async fn check_rate_limit(
    redis: &deadpool_redis::Pool,
    config: &RateLimitConfig,
    key: &str,
) -> Result<RateLimitResult, AppError> {
    let now = chrono::Utc::now().timestamp();
    let window_start = now - config.window_secs as i64;
    let redis_key = format!("ratelimit:{}", key);

    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("Redis connection error: {}", e)))?;

    // Remove entries outside the sliding window
    let _: () = conn
        .zrembyscore(&redis_key, 0i64, window_start)
        .await
        .map_err(AppError::Redis)?;

    // Count remaining entries in the window
    let current_count: u64 = conn.zcard(&redis_key).await.map_err(AppError::Redis)?;

    if current_count >= config.max_requests {
        let reset_at = (now as u64) + config.window_secs;
        return Ok(RateLimitResult {
            allowed: false,
            remaining: 0,
            reset_at,
        });
    }

    // Record this attempt and set TTL
    let _: () = conn
        .zadd(&redis_key, now, now)
        .await
        .map_err(AppError::Redis)?;
    let _: () = conn
        .expire(&redis_key, config.window_secs as i64)
        .await
        .map_err(AppError::Redis)?;

    Ok(RateLimitResult {
        allowed: true,
        remaining: config.max_requests - current_count - 1,
        reset_at: (now as u64) + config.window_secs,
    })
}
