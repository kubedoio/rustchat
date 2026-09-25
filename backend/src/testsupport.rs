//! Test support: minimal [`AppState`] construction.
//!
//! Shared by unit tests and integration tests so that neither needs to
//! build a router (and thereby spawn background workers) just to obtain
//! application state. The previous test harness built a throwaway router
//! for this purpose; [`minimal_app_state`] constructs the state directly.

use tokio_util::sync::CancellationToken;

use crate::bootstrap::BootstrapInputs;
use crate::config::Config;
use crate::realtime::WsHub;
use crate::state::AppState;
use crate::storage::S3Client;

/// Construct a minimal [`AppState`] for tests.
///
/// Connections are lazy or fake: no database, Redis, or S3 server is
/// contacted, and no background workers are spawned. Tests that need real
/// infrastructure should build their own state against their own pools.
pub fn minimal_app_state() -> AppState {
    let config: Config =
        serde_json::from_str(
        r#"{"database_url":"postgres://fake:fake@localhost:5432/fake","jwt_secret":"test-secret","encryption_key":"test-encryption-key"}"#,
    )
    .expect("minimal config");
    let shutdown = CancellationToken::new();

    let inputs = BootstrapInputs {
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
        config: config.clone(),
        shutdown: shutdown.clone(),
    };

    let deps = crate::bootstrap::dependencies::AppDependencies::build(&inputs);

    AppState {
        db: inputs.db,
        redis: inputs.redis,
        jwt_secret: "test-secret".to_string(),
        jwt_issuer: config.jwt_issuer.clone(),
        jwt_audience: config.jwt_audience.clone(),
        jwt_expiry_hours: 1,
        ws_hub: inputs.ws_hub,
        connection_store: deps.connection_store,
        s3_client: inputs.s3_client,
        http_client: deps.http_client,
        start_time: std::time::Instant::now(),
        config,
        sfu_manager: deps.calls.sfu_manager,
        call_state_manager: deps.calls.call_state_manager,
        circuit_breakers: deps.circuit_breakers,
        reconciliation_tx: None,
        agent_runtime: None,
        shutdown,
    }
}
