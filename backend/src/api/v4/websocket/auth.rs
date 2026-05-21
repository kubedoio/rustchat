use std::time::Duration;

use crate::api::websocket_core;
use crate::api::AppState;
use axum::extract::ws::{Message, WebSocket};
use serde_json::json;
use tokio::time::timeout;

/// Authenticate via WebSocket message exchange
pub(crate) async fn authenticate_via_websocket(
    mut socket: WebSocket,
    state: &AppState,
) -> Option<(websocket_core::WebSocketAuth, WebSocket)> {
    // Wait for authentication challenge
    let timeout_duration = Duration::from_secs(30);

    loop {
        match timeout(timeout_duration, socket.recv()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                if let Some(challenge) = parse_authentication_challenge(&text) {
                    let valid_user = websocket_core::normalize_auth_token(&challenge.token)
                        .and_then(|t| websocket_core::validate_auth_token(&t, state));

                    if let Some(auth) = valid_user {
                        let resp = json!({
                            "status": "OK",
                            "seq_reply": challenge.seq_reply
                        });
                        let _ = socket.send(Message::Text(resp.to_string().into())).await;
                        return Some((auth, socket));
                    }

                    let resp = json!({
                        "status": "FAIL",
                        "seq_reply": challenge.seq_reply,
                        "error": "Invalid token"
                    });
                    let _ = socket.send(Message::Text(resp.to_string().into())).await;
                }
            }
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                return None;
            }
            Ok(Some(Err(_))) => {
                return None;
            }
            Err(_) => {
                // Timeout
                return None;
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
struct AuthenticationChallenge {
    token: String,
    seq_reply: serde_json::Value,
}

fn parse_authentication_challenge(text: &str) -> Option<AuthenticationChallenge> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    if value.get("action").and_then(|v| v.as_str()) != Some("authentication_challenge") {
        return None;
    }
    let token = value
        .get("data")
        .and_then(|v| v.get("token"))
        .and_then(|v| v.as_str())?
        .to_string();
    let seq_reply = value.get("seq").cloned().unwrap_or(serde_json::Value::Null);
    Some(AuthenticationChallenge { token, seq_reply })
}

#[cfg(test)]
mod tests {
    use super::parse_authentication_challenge;

    #[test]
    fn parse_authentication_challenge_accepts_valid_payload() {
        let msg = r#"{
            "action":"authentication_challenge",
            "seq":7,
            "data":{"token":"abc.def.ghi"}
        }"#;

        let challenge = parse_authentication_challenge(msg).expect("challenge should parse");
        assert_eq!(challenge.token, "abc.def.ghi");
        assert_eq!(challenge.seq_reply, serde_json::json!(7));
    }

    #[test]
    fn parse_authentication_challenge_rejects_non_challenge() {
        let msg = r#"{"action":"ping","data":{"token":"abc.def.ghi"}}"#;
        assert!(parse_authentication_challenge(msg).is_none());
    }

    #[test]
    fn parse_authentication_challenge_requires_token() {
        let msg = r#"{"action":"authentication_challenge","seq":3,"data":{}}"#;
        assert!(parse_authentication_challenge(msg).is_none());
    }
}
