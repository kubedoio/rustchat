//! API module for rustchat
//!
//! Provides HTTP routes and handlers.

pub mod admin;
mod admin_audit;
mod admin_email;
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

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::{
    extract::DefaultBodyLimit,
    extract::MatchedPath,
    http::Request,
    http::{HeaderValue, Method},
    Router,
};
use sqlx::PgPool;
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

use crate::api::v4::calls_plugin::start_voice_event_listener;
use crate::calls::sfu::{SFUManager, VOICE_EVENT_CHANNEL_CAPACITY};
use crate::calls::state::{CallStateBackend, CallStateManager};
use crate::config::Config;
use crate::middleware::reliability::ServiceCircuitBreakers;
use crate::middleware::security_headers::{cors_compatible_config, SecurityHeadersLayer};
use crate::realtime::{ConnectionStore, WsHub};
use crate::services::agent_runtime::AgentRuntime;
use crate::services::knowledge::embedder::{Embedder, OpenAiEmbedder};
use crate::services::knowledge::vector_store::{PgVectorStore, VectorStore};
use crate::services::llm::{OpenAiProvider, ProviderRegistry};
use crate::storage::S3Client;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

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

/// Build the main application router
///
/// Returns the router and the constructed [`AppState`] so callers can
/// perform graceful-shutdown cleanup (e.g. closing WebSockets).
#[allow(clippy::too_many_arguments)]
pub fn router(
    db: PgPool,
    redis: deadpool_redis::Pool,
    jwt_secret: String,
    jwt_expiry_hours: u64,
    ws_hub: Arc<WsHub>,
    s3_client: S3Client,
    config: Config,
    shutdown: CancellationToken,
) -> (Router, AppState) {
    let (voice_event_tx, voice_event_rx) = mpsc::channel(VOICE_EVENT_CHANNEL_CAPACITY);
    let sfu_manager = SFUManager::new(config.calls.clone(), voice_event_tx);
    let call_state_manager = Arc::new(CallStateManager::with_backend(
        Some(redis.clone()),
        CallStateBackend::parse(&config.calls.state_backend),
    ));
    let connection_store = ConnectionStore::new(shutdown.clone());

    // Initialize LLM provider registry if env vars are present
    let agent_runtime = {
        let mut provider_registry = ProviderRegistry::new();
        let mut has_provider = false;

        // Prefer RUSTCHAT_OPENAI_API_KEY, fall back to OPENAI_API_KEY for backward compatibility
        let openai_api_key = std::env::var("RUSTCHAT_OPENAI_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .or_else(|| {
                std::env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|k| !k.is_empty())
            });

        if let Some(api_key) = openai_api_key.clone() {
            match OpenAiProvider::new(api_key) {
                Ok(provider) => {
                    provider_registry.register("openai", Arc::new(provider));
                    has_provider = true;
                    tracing::info!("OpenAI LLM provider registered");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to initialize OpenAI provider");
                }
            }
        }

        if has_provider {
            let embedder = openai_api_key
                .map(|key| Arc::new(OpenAiEmbedder::new(key, None, None)) as Arc<dyn Embedder>);
            let vector_store = Some(Arc::new(PgVectorStore::new(&db)) as Arc<dyn VectorStore>);

            // Register tools if API keys are available
            let tool_registry = Arc::new(crate::services::tools::registry::ToolRegistry::new());
            if let Some(tavily_key) = std::env::var("TAVILY_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
            {
                tool_registry.register(Arc::new(
                    crate::services::tools::web_search::WebSearchTool::new(tavily_key),
                ));
                tracing::info!("Web search tool registered");
            }
            let tool_registry = if tool_registry.is_empty() {
                None
            } else {
                Some(tool_registry)
            };

            Some(Arc::new(AgentRuntime::new(
                db.clone(),
                ws_hub.clone(),
                Arc::new(provider_registry),
                embedder,
                vector_store,
                tool_registry,
            )))
        } else {
            tracing::info!("No LLM providers configured; agent runtime disabled");
            None
        }
    };

    // Create a temporary state for the reconciliation worker
    let temp_state = Arc::new(AppState {
        db: db.clone(),
        redis: redis.clone(),
        jwt_secret: jwt_secret.clone(),
        jwt_issuer: config.jwt_issuer.clone(),
        jwt_audience: config.jwt_audience.clone(),
        jwt_expiry_hours,
        ws_hub: ws_hub.clone(),
        connection_store: connection_store.clone(),
        s3_client: s3_client.clone(),
        http_client: reqwest::Client::new(),
        start_time: std::time::Instant::now(),
        config: config.clone(),
        sfu_manager: sfu_manager.clone(),
        call_state_manager: call_state_manager.clone(),
        circuit_breakers: Arc::new(ServiceCircuitBreakers::new()),
        reconciliation_tx: None,
        agent_runtime: agent_runtime.clone(),
        shutdown: shutdown.clone(),
    });

    // Spawn membership reconciliation worker
    let (_reconciliation_handle, reconciliation_tx) =
        crate::services::membership_reconciliation::spawn_reconciliation_worker(temp_state.clone());

    // Spawn periodic reconciliation
    let _periodic_handle =
        crate::services::membership_reconciliation::spawn_periodic_reconciliation(
            temp_state.clone(),
            reconciliation_tx.clone(),
        );

    let state = AppState {
        db,
        redis,
        jwt_secret,
        jwt_issuer: config.jwt_issuer.clone(),
        jwt_audience: config.jwt_audience.clone(),
        jwt_expiry_hours,
        ws_hub,
        connection_store,
        s3_client,
        http_client: reqwest::Client::new(),
        start_time: std::time::Instant::now(),
        config,
        sfu_manager,
        call_state_manager,
        circuit_breakers: Arc::new(ServiceCircuitBreakers::new()),
        reconciliation_tx: Some(reconciliation_tx),
        agent_runtime,
        shutdown,
    };

    let _keycloak_sync_handle = if state.config.keycloak_sync.enabled {
        Some(crate::services::keycloak_sync::spawn_periodic_keycloak_sync(Arc::new(state.clone())))
    } else {
        None
    };

    // Start Calls voice event listener
    tokio::spawn(start_voice_event_listener(state.clone(), voice_event_rx));

    if state.config.unread.unread_v2_enabled {
        tokio::spawn(crate::services::unreads::run_unread_v2_reconciler(
            state.clone(),
        ));
    }

    let _custom_status_expiry_handle =
        crate::jobs::spawn_custom_status_expiry_worker(Arc::new(state.clone()));

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

    let router = Router::new()
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
        .with_state(state.clone());

    (router, state)
}
