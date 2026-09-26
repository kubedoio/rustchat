//! Connector boundary for the Buzz relay.
//!
//! [`BuzzConnector`] is the single seam between RustChat and a remote Buzz
//! relay. All delivery logic (dispatcher, outbox, retries) is written against
//! the trait; [`http::HttpBuzzConnector`] is the production implementation and
//! [`mock::MockBuzzConnector`] backs the failure-isolation tests. The
//! trait is intentionally minimal — phase one needs submit, an authed probe
//! for connection testing, and health.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::integrations::outbox::DeliveryOutcome;

/// A fully resolved connection: everything needed to talk to one relay.
///
/// The decrypted signing key exists only inside this struct for the duration
/// of a delivery cycle; it is never stored in the clear, logged, or returned
/// by the admin API.
pub struct BuzzConnectionRuntime {
    pub id: Uuid,
    pub relay_url: String,
    pub keys: nostr::key::Keys,
}

impl std::fmt::Debug for BuzzConnectionRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never include key material.
        f.debug_struct("BuzzConnectionRuntime")
            .field("id", &self.id)
            .field("relay_url", &self.relay_url)
            .field("keys", &"<redacted>")
            .finish()
    }
}

/// A signed Nostr event plus its stable id, ready to submit.
#[derive(Debug, Clone)]
pub struct SignedBuzzEvent {
    /// Nostr event id (hex) — deterministic for a given outbox row.
    pub event_id: String,
    /// Serialized event JSON (the `POST /events` body).
    pub body: Vec<u8>,
}

/// Client for a single Buzz relay connection.
#[async_trait]
pub trait BuzzConnector: Send + Sync {
    /// Submit a signed event via `POST /events`.
    ///
    /// Returns [`DeliveryOutcome`]:
    /// * `Delivered` on HTTP 200 with an accepted/duplicate response (a
    ///   duplicate of an already-stored event id is a successful,
    ///   idempotent delivery).
    /// * `Retryable` on 429/5xx, network errors and timeouts (timeouts are
    ///   safe to retry: the event id is stable, so the relay deduplicates),
    ///   and on malformed 200 responses (ambiguous outcome).
    /// * `Terminal` on 400/401/403/404 (bad event, bad credentials, no
    ///   membership/unknown host).
    async fn submit_event(&self, event: &SignedBuzzEvent) -> DeliveryOutcome;

    /// Authenticated round-trip probe (`POST /count` for our own pubkey).
    ///
    /// Verifies URL reachability, NIP-98 signing, and credential acceptance
    /// without writing anything. Used by the admin connection test.
    async fn probe(&self) -> Result<(), String>;
}

/// Factory mapping a connection to a connector instance.
///
/// The dispatcher holds one provider; the production provider builds
/// SSRF-pinned HTTP connectors, tests inject mock providers.
#[async_trait]
pub trait BuzzConnectorProvider: Send + Sync {
    async fn connector_for(
        &self,
        conn: &BuzzConnectionRuntime,
    ) -> Result<Arc<dyn BuzzConnector>, String>;
}
