use chrono::Utc;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;
use tracing::{debug, info, warn};

use crate::api::AppState;
use crate::mattermost_compat::id::encode_mm_id;
use crate::realtime::WsEnvelope;

use super::helpers::build_call_state_response;
use super::VoiceEvent;

pub(crate) async fn broadcast_screen_share_event(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
    is_on: bool,
) {
    let call = state
        .call_state_manager
        .get_call_by_channel(&channel_id)
        .await;
    let call_id = call.map(|c| c.call_id).unwrap_or_default();

    let payload = serde_json::json!({
        "user_id": encode_mm_id(user_id),
        "user_id_raw": user_id.to_string(),
        "session_id": session_id.to_string(),
        "session_id_raw": session_id.to_string(),
    });

    debug!(
        call_id = %call_id,
        channel_id = %channel_id,
        user_id = %user_id,
        session_id = %session_id,
        is_on = is_on,
        "calls.broadcast_screen_share_event"
    );

    broadcast_call_event(
        state,
        if is_on {
            "custom_com.mattermost.calls_user_screen_on"
        } else {
            "custom_com.mattermost.calls_user_screen_off"
        },
        &channel_id,
        payload.clone(),
        None,
    )
    .await;

    // Legacy aliases kept for compatibility with existing rustchat consumers.
    broadcast_call_event(
        state,
        if is_on {
            "custom_com.mattermost.calls_screen_on"
        } else {
            "custom_com.mattermost.calls_screen_off"
        },
        &channel_id,
        payload,
        None,
    )
    .await;
}
pub(crate) async fn broadcast_raise_hand_event(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
    raised: bool,
) {
    let raised_hand = if raised {
        Utc::now().timestamp_millis()
    } else {
        0
    };
    let payload = serde_json::json!({
        "user_id": encode_mm_id(user_id),
        "session_id": session_id.to_string(),
        "raised_hand": raised_hand,
    });

    broadcast_call_event(
        state,
        if raised {
            "custom_com.mattermost.calls_user_raise_hand"
        } else {
            "custom_com.mattermost.calls_user_unraise_hand"
        },
        &channel_id,
        payload.clone(),
        None,
    )
    .await;

    // Legacy aliases kept for compatibility with existing rustchat consumers.
    broadcast_call_event(
        state,
        if raised {
            "custom_com.mattermost.calls_raise_hand"
        } else {
            "custom_com.mattermost.calls_lower_hand"
        },
        &channel_id,
        payload,
        None,
    )
    .await;
}
pub(crate) async fn broadcast_host_changed_event(state: &AppState, channel_id: Uuid, new_host_id: Uuid) {
    let encoded_host_id = encode_mm_id(new_host_id);
    let event_payload = serde_json::json!({
        "hostID": encoded_host_id,
        "host_id": encoded_host_id,
    });

    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_call_host_changed",
        &channel_id,
        event_payload.clone(),
        None,
    )
    .await;
    // Legacy alias kept for compatibility with existing rustchat consumers.
    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_host_changed",
        &channel_id,
        event_payload,
        None,
    )
    .await;
}
pub(crate) async fn broadcast_call_event(
    state: &AppState,
    event_name: &str,
    channel_id: &Uuid,
    mut data: Value,
    exclude_user_id: Option<Uuid>,
) {
    if let Some(obj) = data.as_object_mut() {
        obj.entry("channelID".to_string())
            .or_insert_with(|| Value::String(encode_mm_id(*channel_id)));
        obj.entry("channel_id".to_string())
            .or_insert_with(|| Value::String(encode_mm_id(*channel_id)));
        obj.entry("channel_id_raw".to_string())
            .or_insert_with(|| Value::String(channel_id.to_string()));
    }

    debug!(
        event = event_name,
        channel_id = %channel_id,
        exclude_user_id = ?exclude_user_id,
        "calls.broadcast_call_event"
    );
    let envelope = WsEnvelope {
        msg_type: "event".to_string(),
        event: event_name.to_string(),
        seq: None,
        channel_id: Some(*channel_id),
        data,
        broadcast: Some(crate::realtime::WsBroadcast {
            channel_id: Some(*channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id,
        }),
    };

    state.ws_hub.broadcast(envelope).await;
}
pub(crate) async fn broadcast_ringing_event(
    state: &AppState,
    channel_id: Uuid,
    call_id: Uuid,
    sender_id: Uuid,
    exclude_user_id: Option<Uuid>,
) {
    // Fetch sender info for better mobile client support
    let sender_info: Option<(String, String)> = sqlx::query_as(
        "SELECT username, COALESCE(display_name, '') as display_name FROM users WHERE id = $1",
    )
    .bind(sender_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let (username, display_name) =
        sender_info.unwrap_or_else(|| (encode_mm_id(sender_id), String::new()));

    info!(
        call_id = %call_id,
        channel_id = %channel_id,
        sender_id = %sender_id,
        exclude_user_id = ?exclude_user_id,
        "calls.broadcast_ringing_event STARTED - will send push notifications"
    );

    // Broadcast WebSocket event
    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_ringing",
        &channel_id,
        serde_json::json!({
            "call_id": encode_mm_id(call_id),
            "call_id_raw": call_id.to_string(),
            "sender_id": encode_mm_id(sender_id),
            "sender_id_raw": sender_id.to_string(),
            "username": username,
            "display_name": display_name,
        }),
        exclude_user_id,
    )
    .await;

    // Also send push notifications to offline/mobile users
    // Get channel members to notify
    let members: Vec<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM channel_members WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    info!(
        member_count = members.len(),
        "Found channel members for push notification"
    );

    let caller_name = if !display_name.is_empty() {
        display_name.clone()
    } else {
        username.clone()
    };

    for (user_id,) in members {
        // Skip the sender
        if Some(user_id) == exclude_user_id {
            info!(user_id = %user_id, "Skipping sender for push notification");
            continue;
        }

        // Skip users who have dismissed this notification
        if state
            .call_state_manager
            .is_notification_dismissed(call_id, user_id)
            .await
        {
            info!(user_id = %user_id, "Skipping user who dismissed notification");
            continue;
        }

        info!(user_id = %user_id, caller_name = %caller_name, "Sending push notification to user");

        // Send push notification asynchronously (don't block)
        let state_clone = state.clone();
        let caller_name_clone = caller_name.clone();
        tokio::spawn(async move {
            match crate::services::push_notifications::send_call_ringing_notification(
                &state_clone,
                user_id,
                channel_id,
                call_id,
                caller_name_clone,
            )
            .await
            {
                Ok(count) if count > 0 => {
                    info!(
                        user_id = %user_id,
                        count = count,
                        "Sent push notification for incoming call"
                    );
                }
                Ok(_) => {
                    // No devices to notify
                }
                Err(e) => {
                    let error_message = e.to_string();
                    debug!(
                        user_id = %user_id,
                        error = %error_message,
                        "Failed to send push notification for call"
                    );
                }
            }
        });
    }
}
pub(crate) async fn broadcast_call_state_event(
    state: &AppState,
    channel_id: Uuid,
    exclude_user_id: Option<Uuid>,
) {
    let Some(call) = state
        .call_state_manager
        .get_call_by_channel(&channel_id)
        .await
    else {
        return;
    };

    let call_state =
        match build_call_state_response(state, &call, encode_mm_id(channel_id), channel_id).await {
            Ok(state_payload) => state_payload,
            Err(err) => {
                warn!(
                    call_id = %call.call_id,
                    channel_id = %channel_id,
                    error = %err,
                    "calls.call_state failed to build call state payload"
                );
                return;
            }
        };

    let call_json = match serde_json::to_string(&call_state) {
        Ok(payload) => payload,
        Err(err) => {
            warn!(
                call_id = %call.call_id,
                channel_id = %channel_id,
                error = %err,
                "calls.call_state failed to serialize call state payload"
            );
            return;
        }
    };

    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_call_state",
        &channel_id,
        serde_json::json!({
            "call": call_json,
            "call_id": encode_mm_id(call.call_id),
            "callID": encode_mm_id(call.call_id),
        }),
        exclude_user_id,
    )
    .await;
}
/// Start a background task to listen for voice events from the SFU and broadcast them via WebSockets
pub async fn start_voice_event_listener(state: AppState, mut rx: mpsc::Receiver<VoiceEvent>) {
    info!("Starting Calls Voice Event Listener");
    while let Some(event) = rx.recv().await {
        match event {
            VoiceEvent::VoiceOn {
                call_id,
                session_id,
            } => {
                let Some(call) = state.call_state_manager.get_call(call_id).await else {
                    continue;
                };
                broadcast_call_event(
                    &state,
                    "custom_com.mattermost.calls_user_voice_on",
                    &call.channel_id,
                    serde_json::json!({
                        "session_id": session_id.to_string(),
                    }),
                    None,
                )
                .await;
            }
            VoiceEvent::VoiceOff {
                call_id,
                session_id,
            } => {
                let Some(call) = state.call_state_manager.get_call(call_id).await else {
                    continue;
                };
                broadcast_call_event(
                    &state,
                    "custom_com.mattermost.calls_user_voice_off",
                    &call.channel_id,
                    serde_json::json!({
                        "session_id": session_id.to_string(),
                    }),
                    None,
                )
                .await;
            }
        }
    }
    warn!("Calls Voice Event Listener stopped");
}
