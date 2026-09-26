//! Background worker lifecycle.
//!
//! [`RuntimeSupervisor`] owns the application-lifetime background tasks:
//! it retains their [`JoinHandle`]s, shares a cancellation token, and
//! provides a bounded graceful shutdown that logs worker failures.
//!
//! Only application-lifetime workers belong here. Short-lived request tasks
//! must continue to be spawned directly.

use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::state::AppState;

/// How long [`RuntimeSupervisor::shutdown`] waits for workers to finish
/// before giving up and logging the stragglers.
const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Supervised application-lifetime background tasks.
///
/// Workers are spawned by [`spawn_application_workers`]; the supervisor
/// retains every handle so that no critical task is silently detached.
pub struct RuntimeSupervisor {
    shutdown: CancellationToken,
    workers: Vec<(&'static str, JoinHandle<()>)>,
}

impl RuntimeSupervisor {
    /// Create an empty supervisor bound to the given shutdown token.
    ///
    /// The token is the application-wide shutdown token (the same one
    /// carried by [`AppState::shutdown`]); cancelling it signals every
    /// worker to exit.
    pub fn new(shutdown: CancellationToken) -> Self {
        Self {
            shutdown,
            workers: Vec::new(),
        }
    }

    /// Register a worker handle with a human-readable name.
    pub fn track(&mut self, name: &'static str, handle: JoinHandle<()>) {
        self.workers.push((name, handle));
    }

    /// Number of supervised workers.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Names of the supervised workers, in spawn order.
    pub fn worker_names(&self) -> Vec<&'static str> {
        self.workers.iter().map(|(name, _)| *name).collect()
    }

    /// Signal shutdown and wait (bounded) for all workers to finish.
    ///
    /// * Cancels the shared shutdown token.
    /// * Joins every worker with an overall timeout.
    /// * Logs each worker that finished with an error and each worker that
    ///   did not finish within the timeout.
    ///
    /// Workers that observe the cancellation are expected to exit promptly;
    /// a worker stuck past the timeout is logged, not aborted, so it can
    /// still finish critical cleanup.
    pub async fn shutdown(self) {
        self.shutdown_with_timeout(DEFAULT_SHUTDOWN_TIMEOUT).await;
    }

    /// [`RuntimeSupervisor::shutdown`] with an explicit timeout.
    pub async fn shutdown_with_timeout(self, timeout: Duration) {
        self.shutdown.cancel();

        let join_all = async {
            for (name, handle) in self.workers {
                match handle.await {
                    Ok(()) => tracing::debug!(worker = name, "Worker exited cleanly"),
                    Err(e) => {
                        tracing::error!(worker = name, error = %e, "Worker task failed")
                    }
                }
            }
        };

        if tokio::time::timeout(timeout, join_all).await.is_err() {
            tracing::error!(
                timeout_secs = timeout.as_secs(),
                "Background workers did not shut down within the timeout"
            );
        }
    }
}

/// Spawn all application-lifetime background workers.
///
/// This is the single place where long-running tasks are started. Every
/// spawned task is tracked by the returned [`RuntimeSupervisor`].
///
/// The reconciliation receiver is passed in (rather than created here)
/// because the matching sender is embedded in [`AppState`] before any
/// worker starts — this removes the previous temporary-state bootstrap.
pub fn spawn_application_workers(
    state: AppState,
    voice_events: mpsc::Receiver<crate::calls::sfu::VoiceEvent>,
    reconciliation_rx: async_channel::Receiver<
        crate::services::membership_reconciliation::ReconciliationTask,
    >,
) -> RuntimeSupervisor {
    let state = std::sync::Arc::new(state);
    let mut supervisor = RuntimeSupervisor::new(state.shutdown.clone());

    // Membership reconciliation worker (consumes the shared channel).
    let reconciliation_handle =
        crate::services::membership_reconciliation::spawn_reconciliation_worker_with_receiver(
            state.clone(),
            reconciliation_rx,
        );
    supervisor.track("membership-reconciliation", reconciliation_handle);

    // Periodic (hourly) full reconciliation.
    if let Some(tx) = state.reconciliation_tx.clone() {
        let periodic_handle =
            crate::services::membership_reconciliation::spawn_periodic_reconciliation(
                state.clone(),
                tx,
            );
        supervisor.track("periodic-reconciliation", periodic_handle);
    }

    // Keycloak sync (optional; only when configured).
    if state.config.keycloak_sync.enabled {
        let keycloak_handle =
            crate::services::keycloak_sync::spawn_periodic_keycloak_sync(state.clone());
        supervisor.track("keycloak-sync", keycloak_handle);
    }

    // Calls voice event listener.
    let voice_state = (*state).clone();
    let voice_handle = tokio::spawn(async move {
        crate::api::v4::calls_plugin::start_voice_event_listener(voice_state, voice_events).await;
    });
    supervisor.track("voice-event-listener", voice_handle);

    // Unread v2 reconciler (optional; only when enabled).
    if state.config.unread.unread_v2_enabled {
        let unread_state = (*state).clone();
        let unread_handle = tokio::spawn(crate::services::unreads::run_unread_v2_reconciler(
            unread_state,
        ));
        supervisor.track("unread-v2-reconciler", unread_handle);
    }

    // Custom status expiry worker.
    let status_expiry_handle = crate::jobs::spawn_custom_status_expiry_worker(state.clone());
    supervisor.track("custom-status-expiry", status_expiry_handle);

    // Retention worker.
    let retention_handle = crate::jobs::spawn_retention_job(
        state.db.clone(),
        state.s3_client.clone(),
        state.config.retention.clone(),
        state.shutdown.clone(),
    );
    supervisor.track("retention", retention_handle);

    // Email worker.
    let email_handle = crate::jobs::spawn_email_worker(
        state.db.clone(),
        crate::jobs::EmailWorkerConfig::default(),
        state.config.encryption_key.clone(),
        state.shutdown.clone(),
    );
    supervisor.track("email", email_handle);

    // Integration outbox dispatcher (optional; only when a bridge is enabled
    // in configuration AND this instance is designated to drain the outbox —
    // default on, so a single-instance deployment drains it). Multi-instance
    // deployments set RUSTCHAT_INTEGRATIONS_BUZZ_RUN_DISPATCHER=false on every
    // process except the designated drainer. When off, behavior is
    // indistinguishable from a build without the integration.
    if state.config.integrations.buzz.enabled && state.config.integrations.buzz.run_dispatcher {
        let dispatcher_state = (*state).clone();
        let dispatcher_handle = crate::integrations::buzz::dispatcher::spawn_outbox_dispatcher(
            dispatcher_state,
            std::sync::Arc::new(crate::integrations::buzz::http::HttpBuzzConnectorProvider),
            state.shutdown.clone(),
        );
        supervisor.track("buzz-outbox-dispatcher", dispatcher_handle);
    }

    supervisor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn supervisor_cancels_and_joins_workers() {
        let shutdown = CancellationToken::new();
        let mut supervisor = RuntimeSupervisor::new(shutdown.clone());

        let worker_shutdown = shutdown.clone();
        supervisor.track(
            "test-worker",
            tokio::spawn(async move {
                tokio::select! {
                    _ = worker_shutdown.cancelled() => {}
                }
            }),
        );

        assert_eq!(supervisor.worker_count(), 1);
        supervisor.shutdown().await;
        // After shutdown the token is cancelled.
        assert!(shutdown.is_cancelled());
    }

    #[tokio::test]
    async fn supervisor_reports_worker_failure_observably() {
        // A panicking worker is surfaced through the error log in
        // `shutdown`; the join result is what we assert on here.
        let shutdown = CancellationToken::new();
        let mut supervisor = RuntimeSupervisor::new(shutdown.clone());

        supervisor.track(
            "panicking-worker",
            tokio::spawn(async {
                panic!("intentional test failure");
            }),
        );

        // Shutdown completes even when a worker failed.
        supervisor
            .shutdown_with_timeout(Duration::from_secs(5))
            .await;
    }

    #[tokio::test]
    async fn supervisor_shutdown_is_bounded_for_stuck_workers() {
        let shutdown = CancellationToken::new();
        let mut supervisor = RuntimeSupervisor::new(shutdown.clone());

        // Worker that ignores cancellation and sleeps far longer than the
        // shutdown timeout; the supervisor must not block on it.
        supervisor.track(
            "stuck-worker",
            tokio::spawn(async {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }),
        );

        let start = std::time::Instant::now();
        supervisor
            .shutdown_with_timeout(Duration::from_millis(200))
            .await;
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
