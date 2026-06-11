//! Application state module for rustchat
//!
//! Provides the [`AppState`] struct that is shared across HTTP handlers.

use std::sync::Arc;

use sqlx::PgPool;

use crate::calls::sfu::SFUManager;
use crate::calls::state::CallStateManager;
use crate::config::Config;
use crate::middleware::reliability::ServiceCircuitBreakers;
use crate::realtime::{ConnectionStore, WsHub};
use crate::services::agent_runtime::AgentRuntime;
use crate::storage::S3Client;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub jwt_secret: String,
    pub jwt_issuer: Option<String>,
    pub jwt_audience: Option<String>,
    pub jwt_expiry_hours: u64,
    pub ws_hub: Arc<WsHub>,
    pub connection_store: Arc<ConnectionStore>,
    pub s3_client: S3Client,
    pub http_client: reqwest::Client,
    pub start_time: std::time::Instant,
    pub config: Config,
    pub sfu_manager: Arc<SFUManager>,
    pub call_state_manager: Arc<CallStateManager>,
    pub circuit_breakers: Arc<ServiceCircuitBreakers>,
    pub reconciliation_tx: Option<
        async_channel::Sender<crate::services::membership_reconciliation::ReconciliationTask>,
    >,
    pub agent_runtime: Option<Arc<AgentRuntime>>,
    pub shutdown: tokio_util::sync::CancellationToken,
}
