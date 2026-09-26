//! Nostr event construction for the Buzz bridge.
//!
//! Buzz is event-driven: every message is a signed Nostr event. Chat messages
//! in a channel ("stream") are kind 9 events carrying an `h` tag with the
//! channel UUID (NIP-29 group chat), as produced by Buzz's own SDK
//! (`buzz-sdk::builders::build_message`, verified against upstream
//! block/buzz @ b65cff31a4c5f4a0af63952b60a21fd73195321a).
//!
//! Two invariants matter for delivery correctness:
//!
//! 1. **Deterministic event ids.** The Nostr event id is a hash over
//!    `(pubkey, created_at, kind, tags, content)` — the signature is not part
//!    of it. We therefore fix `created_at` to the *enqueue* timestamp (stored
//!    on the outbox row) instead of the delivery attempt time, so every retry
//!    of a row produces the same event id and the relay deduplicates it.
//! 2. **Origin markers.** Every bridge event carries namespaced
//!    `rustchat:bridge` / `rustchat:post` tags (non-indexed, matching Buzz's
//!    own namespaced-tag convention such as `buzz:workflow`). Any future
//!    inbound bridge must ignore events carrying our origin marker — that is
//!    the loop-prevention contract.

use nostr::event::{EventBuilder, FinalizeEvent, Kind, Tag};
use nostr::types::time::Timestamp;
use uuid::Uuid;

use crate::integrations::buzz::connector::{BuzzConnectionRuntime, SignedBuzzEvent};

/// Nostr kind for a stream (channel) chat message — `KIND_STREAM_MESSAGE`.
pub const KIND_STREAM_MESSAGE: u16 = 9;

/// Namespaced origin-marker tag names (loop prevention).
pub const ORIGIN_TAG_BRIDGE: &str = "rustchat:bridge";
pub const ORIGIN_TAG_POST: &str = "rustchat:post";

/// Payload stored in the outbox for a `message.created` event.
///
/// Everything needed to deterministically rebuild the Nostr event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageCreatedPayload {
    pub post_id: Uuid,
    pub rustchat_channel_id: Uuid,
    pub buzz_channel_id: Uuid,
    /// Human-readable author label prefixed to the content (the event itself
    /// is signed by the bridge key, so author attribution lives in content).
    pub author_label: String,
    /// Message content (plain text, bounded by the RustChat post limits).
    pub content: String,
    /// Root post id when this message is a thread reply.
    pub root_post_id: Option<Uuid>,
}

impl MessageCreatedPayload {
    pub fn idempotency_key(connection_id: Uuid, post_id: Uuid) -> String {
        format!("buzz:{connection_id}:message:{post_id}")
    }

    /// The kind-9 content: author attribution plus the message text.
    pub fn rendered_content(&self) -> String {
        format!("{}: {}", self.author_label, self.content)
    }
}

/// Whether an event (as JSON) carries our bridge origin marker.
///
/// Inbound consumers use this to recognize (and skip) events the bridge
/// itself emitted, preventing message loops. Checked against the raw tag
/// array so it works on any conforming Nostr event JSON.
pub fn has_origin_marker(event_json: &serde_json::Value) -> bool {
    event_json
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|tags| {
            tags.iter().any(|tag| {
                tag.as_array()
                    .and_then(|parts| parts.first())
                    .and_then(|name| name.as_str())
                    == Some(ORIGIN_TAG_BRIDGE)
            })
        })
        .unwrap_or(false)
}

/// Build and sign the kind-9 stream message event for an outbox row.
///
/// `created_at_unix` must be the *enqueue* time of the outbox row (not "now")
/// so that retries produce an identical event id.
pub fn build_stream_message_event(
    conn: &BuzzConnectionRuntime,
    payload: &MessageCreatedPayload,
    created_at_unix: u64,
) -> Result<SignedBuzzEvent, String> {
    let content = payload.rendered_content();

    let mut tags = vec![
        Tag::custom("h", [payload.buzz_channel_id.to_string()]),
        // Origin markers (loop prevention) — namespaced like Buzz's own
        // `buzz:workflow` tag; non-indexed, carried verbatim by the relay.
        Tag::custom(ORIGIN_TAG_BRIDGE, [conn.id.to_string()]),
        Tag::custom(ORIGIN_TAG_POST, [payload.post_id.to_string()]),
    ];
    if let Some(root) = payload.root_post_id {
        tags.push(Tag::custom("root", [format!("rcpost:{root}")]));
    }

    let event = EventBuilder::new(Kind::Custom(KIND_STREAM_MESSAGE), content)
        .tags(tags)
        .custom_created_at(Timestamp::from(created_at_unix))
        .finalize(&conn.keys)
        .map_err(|e| format!("event signing failed: {e}"))?;

    let event_id = event.id.to_hex();
    let body =
        serde_json::to_vec(&event).map_err(|e| format!("event serialization failed: {e}"))?;
    Ok(SignedBuzzEvent { event_id, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> BuzzConnectionRuntime {
        BuzzConnectionRuntime {
            id: Uuid::new_v4(),
            relay_url: "https://buzz.example.com".to_string(),
            keys: nostr::key::Keys::generate(),
        }
    }

    fn payload() -> MessageCreatedPayload {
        MessageCreatedPayload {
            post_id: Uuid::new_v4(),
            rustchat_channel_id: Uuid::new_v4(),
            buzz_channel_id: Uuid::new_v4(),
            author_label: "alice".to_string(),
            content: "hello buzz".to_string(),
            root_post_id: None,
        }
    }

    #[test]
    fn event_id_is_stable_across_signings() {
        // The core idempotency invariant: same connection + payload + enqueue
        // timestamp => same Nostr event id (relay-side deduplication key).
        let conn = runtime();
        let payload = payload();
        let a = build_stream_message_event(&conn, &payload, 1_700_000_000).unwrap();
        let b = build_stream_message_event(&conn, &payload, 1_700_000_000).unwrap();
        assert_eq!(a.event_id, b.event_id);
        // A different created_at produces a different id.
        let c = build_stream_message_event(&conn, &payload, 1_700_000_060).unwrap();
        assert_ne!(a.event_id, c.event_id);
    }

    #[test]
    fn event_carries_channel_and_origin_tags() {
        let conn = runtime();
        let payload = payload();
        let signed = build_stream_message_event(&conn, &payload, 1_700_000_000).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&signed.body).unwrap();

        assert_eq!(json["kind"], 9);
        let tags = json["tags"].as_array().unwrap();
        assert!(tags.contains(&serde_json::json!([
            "h",
            payload.buzz_channel_id.to_string()
        ])));
        assert!(tags
            .iter()
            .any(|t| t[0] == ORIGIN_TAG_BRIDGE && t[1] == conn.id.to_string()));
        assert!(tags
            .iter()
            .any(|t| t[0] == ORIGIN_TAG_POST && t[1] == payload.post_id.to_string()));
        assert_eq!(json["content"], "alice: hello buzz");
    }

    #[test]
    fn origin_marker_detection() {
        let conn = runtime();
        let payload = payload();
        let signed = build_stream_message_event(&conn, &payload, 1_700_000_000).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&signed.body).unwrap();
        assert!(has_origin_marker(&json));

        // A native Buzz event (no origin tag) is not recognized as ours.
        let native = serde_json::json!({
            "kind": 9,
            "tags": [["h", "00000000-0000-0000-0000-000000000000"]],
            "content": "native message"
        });
        assert!(!has_origin_marker(&native));
    }
}
