//! Deterministic mock connector for failure-isolation tests.
//!
//! Scripts a sequence of [`DeliveryOutcome`]s per submit and records every
//! submitted event, so dispatcher tests can assert exactly what was (or was
//! not) delivered, how many attempts were made, and that retries reuse the
//! identical event bytes (idempotency).

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::integrations::buzz::connector::{
    BuzzConnectionRuntime, BuzzConnector, BuzzConnectorProvider, SignedBuzzEvent,
};
use crate::integrations::outbox::DeliveryOutcome;

/// Scripted connector.
pub struct MockBuzzConnector {
    /// Outcomes returned per submit, in order. When the script is exhausted,
    /// every further submit returns [`DeliveryOutcome::Delivered`].
    script: Mutex<Vec<DeliveryOutcome>>,
    /// Every event submitted so far, in order.
    submitted: Mutex<Vec<SignedBuzzEvent>>,
    /// Probe result (`Err` message is returned verbatim).
    probe_result: Mutex<Result<(), String>>,
}

impl MockBuzzConnector {
    pub fn new(script: Vec<DeliveryOutcome>) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(script),
            submitted: Mutex::new(Vec::new()),
            probe_result: Mutex::new(Ok(())),
        })
    }

    pub async fn set_probe_result(&self, result: Result<(), String>) {
        *self.probe_result.lock().await = result;
    }

    pub async fn submitted(&self) -> Vec<SignedBuzzEvent> {
        self.submitted.lock().await.clone()
    }

    /// Number of submit calls so far.
    pub async fn submit_count(&self) -> usize {
        self.submitted.lock().await.len()
    }

    /// Whether retries reused the identical relay-dedup event id and content.
    ///
    /// Nostr relays deduplicate on the event **id** (a hash over pubkey /
    /// created_at / kind / tags / content) rather than the signature. nostr
    /// 0.45's `os-rng` signing uses a random nonce, so the signature
    /// legitimately varies between signings of the same event while the id and
    /// payload stay deterministic — exactly the idempotency we rely on.
    pub async fn all_submissions_identical(&self) -> bool {
        let submitted = self.submitted.lock().await;
        let Some(first) = submitted.first() else {
            return true;
        };
        submitted.iter().all(|event| {
            if event.event_id != first.event_id {
                return false;
            }
            // Compare the dedup-relevant fields (ignore the random signature).
            match (
                serde_json::from_slice::<serde_json::Value>(&event.body),
                serde_json::from_slice::<serde_json::Value>(&first.body),
            ) {
                (Ok(a), Ok(b)) => {
                    a.get("kind") == b.get("kind")
                        && a.get("content") == b.get("content")
                        && a.get("tags") == b.get("tags")
                        && a.get("created_at") == b.get("created_at")
                }
                _ => false,
            }
        })
    }
}

#[async_trait]
impl BuzzConnector for MockBuzzConnector {
    async fn submit_event(&self, event: &SignedBuzzEvent) -> DeliveryOutcome {
        self.submitted.lock().await.push(event.clone());
        let mut script = self.script.lock().await;
        if script.is_empty() {
            DeliveryOutcome::Delivered {
                remote_id: Some(event.event_id.clone()),
            }
        } else {
            script.remove(0)
        }
    }

    async fn probe(&self) -> Result<(), String> {
        self.probe_result.lock().await.clone()
    }
}

/// Provider returning the same (shared) mock for every connection.
pub struct MockBuzzConnectorProvider {
    pub connector: Arc<MockBuzzConnector>,
}

#[async_trait]
impl BuzzConnectorProvider for MockBuzzConnectorProvider {
    async fn connector_for(
        &self,
        _conn: &BuzzConnectionRuntime,
    ) -> Result<Arc<dyn BuzzConnector>, String> {
        Ok(self.connector.clone())
    }
}
