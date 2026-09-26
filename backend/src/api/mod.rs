//! API module for rustchat
//!
//! Provides HTTP routes and handlers.
//!
//! Router construction ([`build_router`]) is pure composition: routes,
//! request limits, middleware, CORS, and tracing. It never initializes
//! databases, providers, or long-lived workers — runtime assembly lives in
//! [`crate::bootstrap`].

pub mod admin;
mod admin_audit;
mod admin_email;
mod admin_integrations;
mod admin_membership_policies;
mod admin_permissions;
mod admin_plugins;
mod admin_retention;
mod admin_sso;
mod admin_stats;
mod admin_teams;
mod admin_users;
mod auth;
mod calls;
mod channels;
mod file_validation;
mod files;
mod health;
mod integrations;
mod oauth;
mod playbooks;
mod posts;
mod preferences;
mod search;
mod site;
mod teams;
mod unreads;
mod users;
// Public for integration tests - contains submodules needed for test harness
pub mod v1;
pub mod v4;
mod websocket_core;
mod ws;

use std::time::Duration;

use axum::body::Body;
use axum::{
    extract::DefaultBodyLimit,
    extract::MatchedPath,
    http::Request,
    http::{HeaderValue, Method},
    Router,
};
use tower_http::{
    catch_panic::CatchPanicLayer,
    classify::ServerErrorsFailureClass,
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::Level;

/// Handle panics by converting them to 500 responses
fn handle_panic(
    err: Box<dyn std::any::Any + Send + 'static>,
) -> axum::http::Response<axum::body::Body> {
    let panic_message = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic".to_string()
    };

    tracing::error!("PANIC: {}", panic_message);

    axum::http::Response::builder()
        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            r#"{"error":{"code":"PANIC","message":"Internal server error"}}"#.to_string(),
        ))
        .unwrap()
}

use crate::config::Config;
use crate::middleware::security_headers::{cors_compatible_config, SecurityHeadersLayer};

fn parse_cors_allowed_origins(raw: &str) -> Vec<HeaderValue> {
    raw.split(',')
        .filter_map(|origin| {
            let trimmed = origin.trim();
            if trimmed.is_empty() {
                return None;
            }
            match HeaderValue::from_str(trimmed) {
                Ok(hv) => Some(hv),
                Err(e) => {
                    tracing::warn!(origin = %trimmed, error = %e, "Dropping invalid CORS allowed origin");
                    None
                }
            }
        })
        .collect()
}

pub(crate) fn build_cors_layer(config: &Config) -> CorsLayer {
    let cors = CorsLayer::new().allow_methods([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::OPTIONS,
    ]);

    if let Some(raw_origins) = config.cors_allowed_origins.as_deref() {
        let origins = parse_cors_allowed_origins(raw_origins);
        if !origins.is_empty() {
            return cors.allow_origin(origins).allow_headers(Any);
        }

        tracing::warn!(
            "RUSTCHAT_CORS_ALLOWED_ORIGINS is set but no valid origins were parsed; CORS is restricted"
        );
        return cors;
    }

    if config.allow_dev_cors {
        tracing::warn!(
            "No CORS allowlist configured; permissive Access-Control-Allow-Origin: * is enabled because allow_dev_cors is true"
        );
        return cors.allow_origin(Any).allow_headers(Any);
    }

    tracing::warn!("No CORS allowlist configured; cross-origin browser requests are blocked");
    cors
}

pub use crate::state::AppState;

/// Build the main application router.
///
/// Pure composition: routes, request body limits, middleware (panic
/// catching, compression, security headers, tracing, CORS). The state (and
/// everything it references — databases, providers, workers) is constructed
/// by [`crate::bootstrap::build_application`] before this is called.
pub fn build_router(state: AppState) -> Router {
    // CORS configuration
    let cors = build_cors_layer(&state.config);

    // Body size limits (in bytes)
    const SMALL_BODY_LIMIT: usize = 64 * 1024; // 64KB - for most JSON APIs
    const MEDIUM_BODY_LIMIT: usize = 1024 * 1024; // 1MB - for larger payloads
    const LARGE_BODY_LIMIT: usize = 50 * 1024 * 1024; // 50MB - for file uploads

    // API v1 routes with appropriate body limits
    // Routes that don't handle file uploads get smaller limits
    let api_v1 = Router::new()
        .nest("/health", health::router())
        .nest(
            "/auth",
            auth::router(state.clone()).layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)),
        )
        .nest(
            "/users",
            users::router().layer(DefaultBodyLimit::max(MEDIUM_BODY_LIMIT)),
        )
        .nest(
            "/teams",
            teams::router().layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)),
        )
        .nest(
            "/channels",
            channels::router().layer(DefaultBodyLimit::max(MEDIUM_BODY_LIMIT)),
        )
        .nest(
            "/unreads",
            unreads::router().layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)),
        )
        .merge(posts::router().layer(DefaultBodyLimit::max(MEDIUM_BODY_LIMIT)))
        // Files router gets large limit for uploads
        .merge(files::router(state.clone()).layer(DefaultBodyLimit::max(LARGE_BODY_LIMIT)))
        .merge(search::router(state.clone()).layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)))
        .merge(integrations::router().layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)))
        .merge(admin::router().layer(DefaultBodyLimit::max(MEDIUM_BODY_LIMIT)))
        .merge(preferences::router().layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)))
        .merge(playbooks::router().layer(DefaultBodyLimit::max(MEDIUM_BODY_LIMIT)))
        .merge(calls::router().layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)))
        .merge(oauth::router(state.clone()).layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)))
        .merge(site::router().layer(DefaultBodyLimit::max(SMALL_BODY_LIMIT)))
        // WebSocket endpoint doesn't need body limit
        .merge(ws::router(state.clone()));

    // API v4 with route-specific limits
    let api_v4 = v4::router_with_body_limits(
        state.clone(),
        SMALL_BODY_LIMIT,
        MEDIUM_BODY_LIMIT,
        LARGE_BODY_LIMIT,
    );

    // Configure security headers based on environment
    let security_config = if state.config.is_production() {
        cors_compatible_config()
    } else {
        crate::middleware::security_headers::SecurityHeadersConfig::development()
    };

    Router::new()
        .merge(oauth::web_compat_router())
        .nest("/api/v1", api_v1)
        .nest("/api/v1", v1::router()) // Phase 1 entity endpoints
        .nest("/api/v4", api_v4)
        .layer(CatchPanicLayer::custom(handle_panic))
        .layer(CompressionLayer::new())
        .layer(SecurityHeadersLayer::new(security_config))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str)
                        .unwrap_or("<unknown>");
                    tracing::span!(
                        Level::INFO,
                        "http.request",
                        method = %request.method(),
                        uri = %request.uri(),
                        matched_path = matched_path
                    )
                })
                .on_request(|_request: &Request<Body>, _span: &tracing::Span| {
                    tracing::debug!("request started");
                })
                .on_response(
                    |response: &axum::http::Response<Body>,
                     latency: Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            status = %response.status(),
                            latency_ms = latency.as_millis(),
                            "request completed"
                        );
                    },
                )
                .on_failure(
                    |failure: ServerErrorsFailureClass,
                     latency: Duration,
                     _span: &tracing::Span| {
                        tracing::error!(
                            classification = %failure,
                            latency_ms = latency.as_millis(),
                            "request failed"
                        );
                    },
                ),
        )
        .layer(cors)
        .with_state(state)
}
