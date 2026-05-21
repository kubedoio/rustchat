use tokio::sync::mpsc;
use tracing::debug;
use uuid::Uuid;

use crate::api::AppState;
use crate::mattermost_compat::id::encode_mm_id;
use crate::realtime::{WsBroadcast, WsEnvelope};

use super::sfu::signaling::SignalingMessage;
pub(crate) const CALLS_SIGNAL_EVENT: &str = "custom_com.mattermost.calls_signal";

pub(crate) fn spawn_signaling_forwarder(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
    mut rx: mpsc::Receiver<SignalingMessage>,
) {
    let state = state.clone();
    tokio::spawn(async move {
        send_signaling_event(
            &state,
            channel_id,
            user_id,
            session_id,
            SignalingMessage::ConnectionState {
                state: "ready".to_string(),
            },
        )
        .await;

        while let Some(signal) = rx.recv().await {
            send_signaling_event(&state, channel_id, user_id, session_id, signal).await;
        }
    });
}
pub(crate) async fn send_signaling_event(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
    signal: SignalingMessage,
) {
    let signal_kind = match &signal {
        SignalingMessage::Offer { .. } => "offer",
        SignalingMessage::Answer { .. } => "answer",
        SignalingMessage::IceCandidate { .. } => "ice-candidate",
        SignalingMessage::IceConnectionState { .. } => "ice-state",
        SignalingMessage::ConnectionState { .. } => "connection-state",
        SignalingMessage::Error { .. } => "error",
    };
    debug!(
        channel_id = %channel_id,
        user_id = %user_id,
        session_id = %session_id,
        signal_kind = signal_kind,
        "calls.send_signaling_event"
    );
    let signal_payload = serde_json::to_value(signal).unwrap_or_else(|_| {
        serde_json::json!({
            "type": "error",
            "message": "failed to serialize signaling payload"
        })
    });

    let envelope = WsEnvelope {
        msg_type: "event".to_string(),
        event: CALLS_SIGNAL_EVENT.to_string(),
        seq: None,
        channel_id: Some(channel_id),
        data: serde_json::json!({
            "connID": session_id.to_string(),
            "conn_id": session_id.to_string(),
            "data": signal_payload.to_string(),
            "channel_id": encode_mm_id(channel_id),
            "channel_id_raw": channel_id.to_string(),
            "user_id": encode_mm_id(user_id),
            "user_id_raw": user_id.to_string(),
            "session_id": session_id.to_string(),
            "session_id_raw": session_id.to_string(),
            "signal": signal_payload,
        }),
        broadcast: Some(WsBroadcast {
            channel_id: None,
            team_id: None,
            user_id: Some(user_id),
            exclude_user_id: None,
        }),
    };

    state.ws_hub.broadcast(envelope).await;
}
