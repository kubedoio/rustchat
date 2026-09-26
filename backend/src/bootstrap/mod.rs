//! Application bootstrap.
//!
//! This module owns runtime composition: it constructs shared dependencies
//! ([`dependencies`]), the optional AI agent runtime ([`agent_runtime`]),
//! exactly one [`AppState`], and the supervised background workers
//! ([`workers`]). The HTTP router itself is pure composition and lives in
//! [`crate::api::build_router`].
//!
//! The previous design grew these responsibilities inside
//! `api::router()`, including a temporary `AppState` used only to spawn the
//! reconciliation worker. That is no longer the case:
//!
//! - [`build_application`] creates **one** authoritative state instance.
//! - [`api::build_router`] only wires routes, limits, and middleware — it
//!   never initializes databases, providers, or workers.

pub mod agent_runtime;
pub mod dependencies;
pub mod workers;

use std::sync::Arc;

use axum::Router;
use tokio_util::sync::CancellationToken;

use crate::api;
use crate::config::Config;
use crate::realtime::WsHub;
use crate::state::AppState;
use crate::storage::S3Client;

use dependencies::AppDependencies;
use workers::RuntimeSupervisor;

/// Inputs the process entry point provides to bootstrap.
///
/// Database, Redis, S3, and the WebSocket hub (with its cluster broadcast)
/// are connected by `main` before bootstrap runs; bootstrap constructs
/// everything else.
pub struct BootstrapInputs {
    pub db: sqlx::PgPool,
    pub redis: deadpool_redis::Pool,
    pub s3_client: S3Client,
    pub ws_hub: Arc<WsHub>,
    pub config: Config,
    pub shutdown: CancellationToken,
}

/// A fully assembled application: HTTP router, shared state, and the
/// supervisor owning the background workers.
pub struct Application {
    pub router: Router,
    pub state: Arc<AppState>,
    pub supervisor: RuntimeSupervisor,
}

/// Build the complete application from bootstrap inputs.
///
/// Construction order:
///
/// 1. shared dependencies (HTTP client, connection store, calls runtime,
///    circuit breakers),
/// 2. the optional agent runtime (from typed config only),
/// 3. the reconciliation channel,
/// 4. **one** [`AppState`] carrying the reconciliation sender,
/// 5. all application-lifetime workers, tracked by a
///    [`RuntimeSupervisor`],
/// 6. the pure HTTP router.
pub fn build_application(inputs: BootstrapInputs) -> Application {
    let deps = AppDependencies::build(&inputs);
    let agent_runtime =
        agent_runtime::build_agent_runtime(&inputs.config, &inputs.db, &inputs.ws_hub);

    // Destructure dependencies: state takes the shared halves, the voice
    // event receiver goes to the voice event listener worker.
    let AppDependencies {
        http_client,
        connection_store,
        circuit_breakers,
        calls,
    } = deps;
    let dependencies::CallsRuntime {
        sfu_manager,
        call_state_manager,
        voice_events,
    } = calls;

    // The reconciliation channel is created here so the single AppState can
    // carry the sender while the worker consumes the receiver. This removes
    // the previous temporary-state bootstrap.
    let (reconciliation_tx, reconciliation_rx) = async_channel::bounded(1000);

    let state = Arc::new(AppState {
        db: inputs.db,
        redis: inputs.redis,
        jwt_secret: inputs.config.jwt_secret.clone(),
        jwt_issuer: inputs.config.jwt_issuer.clone(),
        jwt_audience: inputs.config.jwt_audience.clone(),
        jwt_expiry_hours: inputs.config.jwt_expiry_hours,
        ws_hub: inputs.ws_hub,
        connection_store,
        s3_client: inputs.s3_client,
        http_client,
        start_time: std::time::Instant::now(),
        config: inputs.config,
        sfu_manager,
        call_state_manager,
        circuit_breakers,
        reconciliation_tx: Some(reconciliation_tx),
        agent_runtime,
        shutdown: inputs.shutdown.clone(),
    });

    let supervisor =
        workers::spawn_application_workers((*state).clone(), voice_events, reconciliation_rx);

    let router = api::build_router((*state).clone());

    Application {
        router,
        state,
        supervisor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_inputs() -> BootstrapInputs {
        let config: Config =
        serde_json::from_str(
        r#"{"database_url":"postgres://fake:fake@localhost:5432/fake","jwt_secret":"test-secret","encryption_key":"test-encryption-key"}"#,
    )
    .expect("minimal config");
        BootstrapInputs {
            db: sqlx::PgPool::connect_lazy("postgres://fake:fake@localhost:5432/fake")
                .expect("lazy pool"),
            redis: deadpool_redis::Config::default()
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("redis pool"),
            s3_client: S3Client::new(
                Some("http://localhost:9000".to_string()),
                None,
                "test-bucket".to_string(),
                Some("testaccesskey".to_string()),
                Some("testsecretkey".to_string()),
                "us-east-1".to_string(),
            ),
            ws_hub: WsHub::new(),
            config,
            shutdown: CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn bootstrap_creates_one_state_and_supervised_workers() {
        let app = build_application(test_inputs());

        // One authoritative state: the reconciliation sender is present
        // (previously the temporary state carried `None`).
        assert!(app.state.reconciliation_tx.is_some());

        // All application-lifetime workers are supervised.
        assert!(app.supervisor.worker_count() >= 6, "expected membership, periodic, voice, status-expiry, retention, and email workers, got {:?}",
            app.supervisor.worker_names());

        // Clean shutdown joins all workers.
        app.supervisor.shutdown().await;
        assert!(app.state.shutdown.is_cancelled());
    }

    #[tokio::test]
    async fn disabled_optional_integrations_do_not_prevent_startup() {
        // No LLM provider key, no Keycloak sync, no unread v2: the
        // application must still assemble completely.
        let inputs = test_inputs();
        assert!(inputs.config.agents.openai_api_key.is_none());
        assert!(!inputs.config.keycloak_sync.enabled);

        let app = build_application(inputs);
        assert!(app.state.agent_runtime.is_none());
        let names = app.supervisor.worker_names();
        assert!(!names.contains(&"keycloak-sync"));
        assert!(!names.contains(&"unread-v2-reconciler"));
        app.supervisor.shutdown().await;
    }

    #[tokio::test]
    async fn router_can_be_constructed_without_spawning_workers() {
        // Pure router composition: no bootstrap, no workers, no runtime
        // initialization beyond the state itself.
        let state = crate::testsupport::minimal_app_state();
        let router = api::build_router(state);
        // The router is usable as an axum service (type-level proof).
        let _service = router;
    }

    #[tokio::test]
    async fn current_api_routes_remain_mounted() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = api::build_router(crate::testsupport::minimal_app_state());

        // Representative routes from every mounted surface. We assert the
        // route exists (not 404); handlers that need infrastructure may
        // fail, but routing must succeed.
        for uri in [
            "/api/v1/health/live",
            "/api/v4/config/client",
            "/api/v4/license/client",
            "/api/v4/system/ping",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("request executed");
            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "route {uri} is no longer mounted"
            );
        }
    }
}
