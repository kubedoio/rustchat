//! Shared application dependencies constructed once during bootstrap.
//!
//! This module owns the construction of every long-lived dependency that is
//! not already supplied by the process entry point (`main`): the HTTP client,
//! WebSocket connection state, the calls runtime (SFU + call state + voice
//! event channel), and circuit breakers.
//!
//! Nothing in this module spawns tasks; see [`crate::bootstrap::workers`]
//! for background worker lifecycle.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::calls::sfu::{SFUManager, VOICE_EVENT_CHANNEL_CAPACITY};
use crate::calls::state::{CallStateBackend, CallStateManager};
use crate::config::Config;
use crate::middleware::reliability::ServiceCircuitBreakers;
use crate::realtime::ConnectionStore;

use super::BootstrapInputs;

/// Calls runtime dependencies.
///
/// The voice event channel pairs the SFU's event sender (consumed by
/// [`SFUManager`]) with the receiver that the voice event listener worker
/// consumes; both halves are created together here so bootstrap can wire
/// them without a temporary state.
pub struct CallsRuntime {
    pub sfu_manager: Arc<SFUManager>,
    pub call_state_manager: Arc<CallStateManager>,
    /// Receiver for SFU voice events; passed to the voice event listener
    /// worker by [`crate::bootstrap::workers`].
    pub voice_events: mpsc::Receiver<crate::calls::sfu::VoiceEvent>,
}

/// All shared runtime dependencies held by [`crate::state::AppState`].
pub struct AppDependencies {
    pub http_client: reqwest::Client,
    pub connection_store: Arc<ConnectionStore>,
    pub circuit_breakers: Arc<ServiceCircuitBreakers>,
    pub calls: CallsRuntime,
}

impl AppDependencies {
    /// Construct shared dependencies from bootstrap inputs.
    ///
    /// Database, Redis, S3, and the WebSocket hub are supplied by the caller
    /// (they are owned by [`BootstrapInputs`]); this function constructs the
    /// remaining shared state exactly once.
    pub fn build(inputs: &BootstrapInputs) -> Self {
        let calls = build_calls_runtime(&inputs.config, inputs.redis.clone());

        Self {
            http_client: reqwest::Client::new(),
            connection_store: ConnectionStore::new(inputs.shutdown.clone()),
            circuit_breakers: Arc::new(ServiceCircuitBreakers::new()),
            calls,
        }
    }
}

/// Construct the calls runtime.
///
/// The SFU requires a voice event sender at construction time, and the
/// matching receiver must reach the voice event listener worker, so both
/// halves of the channel are created here.
fn build_calls_runtime(config: &Config, redis: deadpool_redis::Pool) -> CallsRuntime {
    let (voice_event_tx, voice_events) = mpsc::channel(VOICE_EVENT_CHANNEL_CAPACITY);
    let sfu_manager = SFUManager::new(config.calls.clone(), voice_event_tx);
    let call_state_manager = Arc::new(CallStateManager::with_backend(
        Some(redis),
        CallStateBackend::parse(&config.calls.state_backend),
    ));

    CallsRuntime {
        sfu_manager,
        call_state_manager,
        voice_events,
    }
}
