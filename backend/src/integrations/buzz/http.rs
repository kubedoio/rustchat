//! Production connector: Buzz relay over its public HTTP bridge.
//!
//! Endpoints used (verified against upstream block/buzz
//! @ b65cff31a4c5f4a0af63952b60a21fd73195321a — see docs/integrations/buzz.md):
//!
//! * `POST {relay_url}/events` — submit a signed Nostr event
//! * `POST {relay_url}/count`  — authed probe (read-only)
//! * `GET  {relay_url}/health` — unauthenticated liveness (unused by default)
//!
//! Authentication is NIP-98: every request carries
//! `Authorization: Nostr <base64(kind-27235 event JSON)>`, freshly signed per
//! request (Buzz enforces ±60s freshness and replays auth event ids), with
//! `u` (exact request URL), `method` and `payload` (SHA-256 of the body) tags.
//!
//! The HTTP client is obtained from the same SSRF-safe, DNS-pinned builder
//! used for outgoing webhooks: schemes restricted, private/loopback/metadata
//! addresses rejected, redirects disabled, no proxies, bounded timeout.

use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use nostr::event::{EventBuilder, FinalizeEvent, Kind, Tag};
use nostr::types::time::Timestamp;
use sha2::{Digest, Sha256};

use crate::integrations::buzz::connector::{
    BuzzConnectionRuntime, BuzzConnector, BuzzConnectorProvider, SignedBuzzEvent,
};
use crate::integrations::outbox::DeliveryOutcome;

/// Maximum response body read from the relay (bytes). Responses beyond this
/// are treated as ambiguous (retryable) failures, never buffered whole.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Request timeout for bridge calls.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Production connector for one Buzz relay connection.
pub struct HttpBuzzConnector {
    client: reqwest::Client,
    base_url: reqwest::Url,
    keys: nostr::key::Keys,
}

impl HttpBuzzConnector {
    /// Build the production connector.
    ///
    /// Returns `Err` when `relay_url` fails the SSRF-safe URL policy or its
    /// host does not resolve to public addresses — the same rules as
    /// outgoing webhooks. Production deployments must additionally use
    /// HTTPS (enforced at connection-creation time by the admin API).
    pub async fn new(relay_url: &str, keys: nostr::key::Keys) -> Result<Self, String> {
        let (client, base_url) = crate::services::webhooks::callback_http_client(relay_url)
            .await
            .ok_or_else(|| "relay URL rejected by the SSRF-safe URL policy".to_string())?;
        Ok(Self {
            client,
            base_url,
            keys,
        })
    }

    /// Test/alternative constructor with an explicit client.
    ///
    /// Skips the SSRF-safe client construction (used by unit tests that run
    /// against a local listener); the request logic — auth, status mapping,
    /// response caps — is identical.
    pub fn with_client(
        client: reqwest::Client,
        base_url: reqwest::Url,
        keys: nostr::key::Keys,
    ) -> Self {
        Self {
            client,
            base_url,
            keys,
        }
    }

    fn url(&self, path: &str) -> Result<reqwest::Url, String> {
        self.base_url
            .join(path)
            .map_err(|e| format!("invalid relay URL path: {e}"))
    }

    /// NIP-98 `Authorization` header for a request.
    ///
    /// Kind 27235 event, signed with the connection key, carrying `u`
    /// (request URL), `method`, and `payload` (SHA-256 hex of the body)
    /// tags — exactly the tag set Buzz's `verify_nip98_event` checks.
    fn nip98_authorization(&self, url: &str, method: &str, body: &[u8]) -> Result<String, String> {
        let payload_hash = hex::encode(Sha256::digest(body));
        let tags = vec![
            Tag::custom("u", [url]),
            Tag::custom("method", [method]),
            Tag::custom("payload", [payload_hash]),
        ];
        let event = EventBuilder::new(Kind::Custom(27_235), "")
            .tags(tags)
            .custom_created_at(Timestamp::now())
            .finalize(&self.keys)
            .map_err(|e| format!("NIP-98 signing failed: {e}"))?;
        let json = serde_json::to_string(&event)
            .map_err(|e| format!("NIP-98 serialization failed: {e}"))?;
        Ok(format!("Nostr {}", BASE64.encode(json)))
    }

    async fn read_body_capped(
        response: reqwest::Response,
    ) -> Result<serde_json::Value, &'static str> {
        if let Some(len) = response.content_length() {
            if len as usize > MAX_RESPONSE_BYTES {
                return Err("response too large");
            }
        }
        let mut bytes = Vec::new();
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(|_| "read error")? {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err("response too large");
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| "malformed response body")
    }
}

/// Map an HTTP status to a delivery outcome (shared with tests).
fn outcome_for_status(status: reqwest::StatusCode) -> DeliveryOutcome {
    let code = status.as_u16();
    match code {
        200..=299 => DeliveryOutcome::Delivered { remote_id: None },
        // Rate limited: back off and retry.
        429 => DeliveryOutcome::Retryable {
            reason: format!("relay responded {code} (rate limited)"),
        },
        // Permanent: the event or credentials are not acceptable.
        400 | 401 | 403 | 404 => DeliveryOutcome::Terminal {
            reason: format!("relay responded {code}"),
        },
        // Server errors and anything unexpected: retry.
        _ => DeliveryOutcome::Retryable {
            reason: format!("relay responded {code}"),
        },
    }
}

#[async_trait::async_trait]
impl BuzzConnector for HttpBuzzConnector {
    async fn submit_event(&self, event: &SignedBuzzEvent) -> DeliveryOutcome {
        let url = match self.url("events") {
            Ok(u) => u,
            Err(e) => return DeliveryOutcome::Terminal { reason: e },
        };
        let auth = match self.nip98_authorization(url.as_str(), "POST", &event.body) {
            Ok(a) => a,
            Err(e) => return DeliveryOutcome::Terminal { reason: e },
        };

        let request = self
            .client
            .post(url)
            .header(reqwest::header::AUTHORIZATION, auth)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(event.body.clone())
            .timeout(REQUEST_TIMEOUT);

        let response = match request.send().await {
            Ok(r) => r,
            // Network error or timeout. The outcome is ambiguous (the relay
            // may have persisted the event), but retries are idempotent
            // because the event id is stable — so retry.
            Err(e) => {
                return DeliveryOutcome::Retryable {
                    reason: format!("network error: {e}"),
                }
            }
        };

        let status = response.status();
        if !status.is_success() {
            return outcome_for_status(status);
        }

        // 200: parse {"event_id", "accepted", "message"}. A malformed body is
        // ambiguous (the event may be stored) — retry, never trust partial
        // success.
        match Self::read_body_capped(response).await {
            Ok(body) => {
                // The relay's `event_id` is relay-controlled data. Accept it
                // only when it is a plausible short identifier (<=64 chars);
                // otherwise fall back to the locally computed id (always a
                // 64-char Nostr event id hex). This prevents a hostile/oversized
                // relay value overflowing the stored column and livelocking a
                // row (deliver -> store-fail -> reclaim -> redeliver).
                let remote_id = body
                    .get("event_id")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && s.len() <= 64)
                    .map(str::to_string);
                DeliveryOutcome::Delivered {
                    // Prefer the relay's event id; fall back to the locally
                    // computed one (they are equal for a conforming relay).
                    remote_id: remote_id.or_else(|| Some(event.event_id.clone())),
                }
            }
            Err(reason) => DeliveryOutcome::Retryable {
                reason: format!("unreadable success response: {reason}"),
            },
        }
    }

    async fn probe(&self) -> Result<(), String> {
        let url = self
            .url("count")
            .map_err(|e| format!("invalid relay URL: {e}"))?;
        // Count our own events: read-only, exercises URL + NIP-98 auth.
        let body =
            serde_json::json!([{ "authors": [self.keys.public_key().to_hex()], "limit": 0 }]);
        let body = serde_json::to_vec(&body).map_err(|e| format!("probe body: {e}"))?;
        let auth = self.nip98_authorization(url.as_str(), "POST", &body)?;
        let request = self
            .client
            .post(url)
            .header(reqwest::header::AUTHORIZATION, auth)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .timeout(REQUEST_TIMEOUT);
        let response = request
            .send()
            .await
            .map_err(|e| format!("relay unreachable: {e}"))?;
        match response.status().as_u16() {
            200..=299 => Ok(()),
            401 | 403 => Err("relay rejected the bridge credentials".to_string()),
            404 => Err("relay has no community for this host".to_string()),
            code => Err(format!("relay responded {code}")),
        }
    }
}

/// Production provider: builds [`HttpBuzzConnector`] instances.
pub struct HttpBuzzConnectorProvider;

#[async_trait::async_trait]
impl BuzzConnectorProvider for HttpBuzzConnectorProvider {
    async fn connector_for(
        &self,
        conn: &BuzzConnectionRuntime,
    ) -> Result<Arc<dyn BuzzConnector>, String> {
        let connector = HttpBuzzConnector::new(&conn.relay_url, conn.keys.clone()).await?;
        Ok(Arc::new(connector))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;
    use axum::Router;
    use nostr::key::Keys;

    async fn spawn_relay(
        handler: axum::routing::Router,
    ) -> (String, tokio::sync::oneshot::Sender<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, handler)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
        });
        (format!("http://{addr}"), tx)
    }

    fn signed_event() -> SignedBuzzEvent {
        let keys = Keys::generate();
        let event = EventBuilder::new(Kind::Custom(9), "test")
            .tags(vec![Tag::custom(
                "h",
                ["00000000-0000-0000-0000-000000000000"],
            )])
            .custom_created_at(Timestamp::from(1_700_000_000))
            .finalize(&keys)
            .unwrap();
        SignedBuzzEvent {
            event_id: event.id.to_hex(),
            body: serde_json::to_vec(&event).unwrap(),
        }
    }

    fn local_connector(base_url: &str) -> HttpBuzzConnector {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        HttpBuzzConnector::with_client(client, base_url.parse().unwrap(), Keys::generate())
    }

    #[tokio::test]
    async fn maps_success_to_delivered() {
        let app = Router::new().route(
            "/events",
            post(|| async {
                axum::Json(serde_json::json!({
                    "event_id": "aa".repeat(32),
                    "accepted": true,
                    "message": "stored"
                }))
            }),
        );
        let (base, _tx) = spawn_relay(app).await;
        let connector = local_connector(&base);
        let outcome = connector.submit_event(&signed_event()).await;
        assert!(matches!(outcome, DeliveryOutcome::Delivered { .. }));
    }

    #[tokio::test]
    async fn ignores_oversized_relay_event_id() {
        // A relay-controlled event_id longer than the 64-char column must be
        // dropped in favour of the locally computed id, so a hostile/oversized
        // value can never overflow the store and livelock a row.
        let app = Router::new().route(
            "/events",
            post(|| async {
                axum::Json(serde_json::json!({
                    "event_id": "x".repeat(200),
                    "accepted": true
                }))
            }),
        );
        let (base, _tx) = spawn_relay(app).await;
        let connector = local_connector(&base);
        let ev = signed_event();
        let outcome = connector.submit_event(&ev).await;
        match outcome {
            DeliveryOutcome::Delivered { remote_id } => {
                let remote_id = remote_id.expect("delivered with a remote id");
                assert_eq!(
                    remote_id.len(),
                    64,
                    "must fall back to the 64-char local id"
                );
                assert_eq!(remote_id, ev.event_id);
            }
            other => panic!("expected Delivered, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn maps_client_errors_to_terminal() {
        for code in [400u16, 401, 403, 404] {
            let app = Router::new().route(
                "/events",
                post(move || async move {
                    (
                        axum::http::StatusCode::from_u16(code).unwrap(),
                        axum::Json(serde_json::json!({"error": "no"})),
                    )
                }),
            );
            let (base, _tx) = spawn_relay(app).await;
            let connector = local_connector(&base);
            let outcome = connector.submit_event(&signed_event()).await;
            assert!(
                matches!(outcome, DeliveryOutcome::Terminal { .. }),
                "status {code} should be terminal, got {outcome:?}"
            );
        }
    }

    #[tokio::test]
    async fn maps_server_errors_and_rate_limits_to_retryable() {
        for code in [429u16, 500, 502, 503] {
            let app = Router::new().route(
                "/events",
                post(move || async move {
                    (
                        axum::http::StatusCode::from_u16(code).unwrap(),
                        axum::Json(serde_json::json!({"error": "no"})),
                    )
                }),
            );
            let (base, _tx) = spawn_relay(app).await;
            let connector = local_connector(&base);
            let outcome = connector.submit_event(&signed_event()).await;
            assert!(
                matches!(outcome, DeliveryOutcome::Retryable { .. }),
                "status {code} should be retryable, got {outcome:?}"
            );
        }
    }

    #[tokio::test]
    async fn maps_malformed_success_body_to_retryable() {
        let app = Router::new().route("/events", post(|| async { "this is not json" }));
        let (base, _tx) = spawn_relay(app).await;
        let connector = local_connector(&base);
        let outcome = connector.submit_event(&signed_event()).await;
        assert!(matches!(outcome, DeliveryOutcome::Retryable { .. }));
    }

    #[tokio::test]
    async fn maps_unreachable_relay_to_retryable() {
        // Reserve a port and never listen on it.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let connector = local_connector(&format!("http://{addr}"));
        let outcome = connector.submit_event(&signed_event()).await;
        assert!(matches!(outcome, DeliveryOutcome::Retryable { .. }));
    }

    #[tokio::test]
    async fn maps_oversized_response_to_retryable() {
        let app = Router::new().route(
            "/events",
            post(|| async {
                (
                    [(reqwest::header::CONTENT_TYPE, "application/json")],
                    "x".repeat(MAX_RESPONSE_BYTES + 1024),
                )
            }),
        );
        let (base, _tx) = spawn_relay(app).await;
        let connector = local_connector(&base);
        let outcome = connector.submit_event(&signed_event()).await;
        assert!(matches!(outcome, DeliveryOutcome::Retryable { .. }));
    }

    #[tokio::test]
    async fn nip98_header_is_well_formed() {
        let connector = local_connector("https://buzz.example.com");
        let body = br#"{"kind":9}"#;
        let header = connector
            .nip98_authorization("https://buzz.example.com/events", "POST", body)
            .unwrap();
        let rest = header.strip_prefix("Nostr ").expect("Nostr scheme");
        let json: serde_json::Value =
            serde_json::from_slice(&BASE64.decode(rest).unwrap()).unwrap();
        assert_eq!(json["kind"], 27235);
        assert_eq!(
            json["tags"][0],
            serde_json::json!(["u", "https://buzz.example.com/events"])
        );
        assert_eq!(json["tags"][1], serde_json::json!(["method", "POST"]));
        // payload tag = sha256 hex of body
        assert_eq!(
            json["tags"][2],
            serde_json::json!(["payload", hex::encode(Sha256::digest(body))])
        );
        assert!(!json["sig"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn ssrf_policy_rejects_private_and_loopback_urls() {
        let keys = Keys::generate();
        assert!(
            HttpBuzzConnector::new("http://127.0.0.1:9000", keys.clone())
                .await
                .is_err()
        );
        assert!(HttpBuzzConnector::new("http://10.0.0.5", keys.clone())
            .await
            .is_err());
        assert!(
            HttpBuzzConnector::new("http://169.254.169.254", keys.clone())
                .await
                .is_err()
        );
        assert!(HttpBuzzConnector::new("ftp://buzz.example.com", keys)
            .await
            .is_err());
    }
}
