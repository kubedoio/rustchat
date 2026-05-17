use chrono::Utc;
use flate2::read::ZlibDecoder;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use uuid::Uuid;
use tracing::{debug, error, info, warn};

use crate::api::AppState;
use crate::error::AppError;
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

use super::broadcast::{broadcast_call_event, broadcast_call_state_event, broadcast_ringing_event};
use super::helpers::{channel_calls_enabled, check_channel_permission, resolve_channel_id};
use super::posts::ensure_call_thread_id;
use super::state::{CallState, Participant};
use super::state_helpers::{reconcile_after_participant_left, schedule_empty_call_timeout, schedule_unanswered_call_timeout};

/// Handle websocket actions used by Mattermost mobile calls.
/// Returns `true` when the action is recognized and handled.
pub async fn handle_ws_action(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    action: &str,
    data: Option<&Value>,
) -> bool {
    let Some(call_action) = action.strip_prefix("custom_com.mattermost.calls_") else {
        return false;
    };

    let result = match call_action {
        "join" | "reconnect" => handle_ws_join_call(state, user_id, connection_id, data).await,
        "leave" => handle_ws_leave_call(state, user_id, connection_id).await,
        "sdp" => handle_ws_sdp(state, user_id, connection_id, data).await,
        "ice" => handle_ws_ice(state, user_id, connection_id, data).await,
        "mute" => handle_ws_mute(state, user_id, connection_id, data, true).await,
        "unmute" => handle_ws_mute(state, user_id, connection_id, data, false).await,
        "raise_hand" => handle_ws_raise_hand(state, user_id, connection_id, true).await,
        "unraise_hand" => handle_ws_raise_hand(state, user_id, connection_id, false).await,
        "react" => handle_ws_reaction(state, user_id, connection_id, data).await,
        "metric" => {
            debug!(
                user_id = %user_id,
                connection_id = connection_id,
                data = ?data,
                "calls.ws metric received"
            );
            Ok(())
        }
        other => {
            warn!(
                user_id = %user_id,
                connection_id = connection_id,
                action = other,
                "calls.ws unsupported action"
            );
            Ok(())
        }
    };

    if let Err(err) = result {
        error!(
            user_id = %user_id,
            connection_id = connection_id,
            action = action,
            error = %err,
            "calls.ws action failed"
        );
        send_ws_plugin_error(state, user_id, connection_id, &err).await;
    }

    true
}
async fn handle_ws_join_call(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    data: Option<&Value>,
) -> Result<(), String> {
    let data = data.ok_or_else(|| "Missing join payload".to_string())?;
    let conn_uuid = resolve_ws_session_uuid(connection_id, Some(data))?;
    let channel_uuid = parse_join_channel_id(data)?;

    check_channel_permission(state, user_id, channel_uuid)
        .await
        .map_err(|e| e.to_string())?;
    if !channel_calls_enabled(channel_uuid) {
        return Err("Calls are disabled in this channel".to_string());
    }

    let call_manager = state.call_state_manager.as_ref();
    let now = Utc::now().timestamp_millis();

    let mut created_call = false;
    let call = if let Some(call) = call_manager.get_call_by_channel(&channel_uuid).await {
        call
    } else {
        created_call = true;
        let call = CallState {
            call_id: Uuid::new_v4(),
            channel_id: channel_uuid,
            owner_id: user_id,
            host_id: user_id,
            started_at: now,
            participants: HashMap::new(),
            screen_sharer: None,
            thread_id: data
                .get("threadID")
                .and_then(|v| v.as_str())
                .and_then(parse_mm_or_uuid),
            dismissed_users: HashSet::new(),
        };
        call_manager.add_call(call.clone()).await;
        call
    };

    let mut should_add_participant = true;
    if let Some(existing) = call_manager.get_participant(call.call_id, user_id).await {
        if existing.session_id == conn_uuid {
            should_add_participant = false;
        } else if let Some(sfu) = state.sfu_manager.get_sfu(call.call_id).await {
            let _ = sfu.remove_participant(existing.session_id).await;
        }
    }

    if should_add_participant {
        call_manager
            .add_participant(
                call.call_id,
                Participant {
                    user_id,
                    session_id: conn_uuid,
                    joined_at: now,
                    muted: true,
                    screen_sharing: false,
                    hand_raised: false,
                },
            )
            .await;
    }

    let sfu = state
        .sfu_manager
        .get_or_create_sfu(call.call_id)
        .await
        .map_err(|e| format!("Failed to get or create SFU: {e}"))?;

    if !sfu.has_participant(conn_uuid).await {
        let _ = sfu
            .add_participant(user_id, conn_uuid)
            .await
            .map_err(|e| format!("Failed to add participant to SFU: {e}"))?;
    }

    let thread_id = ensure_call_thread_id(state, &call).await;

    if created_call {
        schedule_unanswered_call_timeout(state, call.call_id, channel_uuid);
        broadcast_call_event(
            state,
            "custom_com.mattermost.calls_call_start",
            &channel_uuid,
            serde_json::json!({
                "id": encode_mm_id(call.call_id),
                "channelID": encode_mm_id(channel_uuid),
                "start_at": call.started_at,
                "owner_id": encode_mm_id(call.owner_id),
                "host_id": encode_mm_id(call.owner_id),
                "thread_id": thread_id.map(encode_mm_id),
                "call_id": encode_mm_id(call.call_id),
                "channel_id": encode_mm_id(channel_uuid),
            }),
            None,
        )
        .await;

        // Send ringing notifications via push for mobile apps
        // WebSocket join doesn't go through HTTP /start endpoint, so we need to trigger ringing here
        broadcast_ringing_event(state, channel_uuid, call.call_id, user_id, Some(user_id)).await;
    }

    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_user_joined",
        &channel_uuid,
        serde_json::json!({
            "user_id": encode_mm_id(user_id),
            "session_id": conn_uuid.to_string(),
            "muted": true,
            "raised_hand": 0,
        }),
        None,
    )
    .await;

    broadcast_call_state_event(state, channel_uuid, None).await;

    send_ws_plugin_event(
        state,
        user_id,
        "custom_com.mattermost.calls_join",
        serde_json::json!({
            "connID": connection_id,
            "conn_id": connection_id,
            "channelID": encode_mm_id(channel_uuid),
            "channel_id": encode_mm_id(channel_uuid),
            "channel_id_raw": channel_uuid.to_string(),
            "callID": encode_mm_id(call.call_id),
            "call_id": encode_mm_id(call.call_id),
            "call_id_raw": call.call_id.to_string(),
            "sessionID": conn_uuid.to_string(),
            "session_id": conn_uuid.to_string(),
        }),
    )
    .await;

    info!(
        user_id = %user_id,
        connection_id = connection_id,
        channel_id = %channel_uuid,
        call_id = %call.call_id,
        created_call = created_call,
        "calls.ws join handled"
    );

    Ok(())
}
async fn handle_ws_sdp(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    data: Option<&Value>,
) -> Result<(), String> {
    let requested_session_id = resolve_ws_session_uuid(connection_id, data)?;

    info!(
        user_id = %user_id,
        connection_id = connection_id,
        "calls.ws sdp received"
    );

    let sdp = parse_ws_sdp_payload(data).map_err(|e| {
        error!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %e,
            "Failed to parse SDP payload"
        );
        format!("Invalid SDP payload: {e}")
    })?;

    let (call, session_id) =
        resolve_call_for_ws_connection(state, user_id, requested_session_id).await?;

    let sfu = state
        .sfu_manager
        .get_or_create_sfu(call.call_id)
        .await
        .map_err(|e| format!("Failed to get or create SFU: {e}"))?;

    if !sfu.has_participant(session_id).await {
        info!(
            user_id = %user_id,
            session_id = %session_id,
            "Adding participant to SFU for SDP handling"
        );
        let _ = sfu
            .add_participant(user_id, session_id)
            .await
            .map_err(|e| format!("Failed to add participant to SFU: {e}"))?;
    }

    let offer = RTCSessionDescription::offer(sdp).map_err(|e| format!("Invalid offer SDP: {e}"))?;

    info!(
        user_id = %user_id,
        session_id = %session_id,
        "Processing SDP offer"
    );

    let answer = sfu
        .handle_offer(session_id, offer)
        .await
        .map_err(|e| format!("Failed to handle offer: {e}"))?;

    info!(
        user_id = %user_id,
        session_id = %session_id,
        sdp_length = answer.sdp.len(),
        "Sending SDP answer"
    );

    send_ws_plugin_signal(
        state,
        user_id,
        connection_id,
        serde_json::json!({
            "type": "answer",
            "sdp": answer.sdp,
        }),
    )
    .await;

    Ok(())
}
async fn handle_ws_ice(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    data: Option<&Value>,
) -> Result<(), String> {
    let requested_session_id = resolve_ws_session_uuid(connection_id, data)?;

    debug!(
        user_id = %user_id,
        connection_id = connection_id,
        "calls.ws ice received"
    );

    let (candidate, sdp_mid, sdp_mline_index) = parse_ws_ice_payload(data).map_err(|e| {
        error!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %e,
            "Failed to parse ICE payload"
        );
        format!("Invalid ICE payload: {e}")
    })?;

    let (call, session_id) =
        resolve_call_for_ws_connection(state, user_id, requested_session_id).await?;

    let sfu = state
        .sfu_manager
        .get_or_create_sfu(call.call_id)
        .await
        .map_err(|e| format!("Failed to get or create SFU: {e}"))?;

    if !sfu.has_participant(session_id).await {
        info!(
            user_id = %user_id,
            session_id = %session_id,
            "Adding participant to SFU for ICE handling"
        );
        let _ = sfu
            .add_participant(user_id, session_id)
            .await
            .map_err(|e| format!("Failed to add participant to SFU: {e}"))?;
    }

    info!(
        user_id = %user_id,
        session_id = %session_id,
        candidate_len = candidate.len(),
        sdp_mid = ?sdp_mid,
        sdp_mline_index = ?sdp_mline_index,
        "Processing ICE candidate"
    );

    sfu.handle_ice_candidate(session_id, candidate, sdp_mid, sdp_mline_index)
        .await
        .map_err(|e| format!("Failed to handle ICE candidate: {e}"))?;

    Ok(())
}
async fn handle_ws_leave_call(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
) -> Result<(), String> {
    let requested_session_id = resolve_ws_session_uuid(connection_id, None)?;
    let Ok((call, session_id)) =
        resolve_call_for_ws_connection(state, user_id, requested_session_id).await
    else {
        return Ok(());
    };

    let call_manager = state.call_state_manager.as_ref();
    call_manager.remove_participant(call.call_id, user_id).await;
    if let Some(sfu) = state.sfu_manager.get_sfu(call.call_id).await {
        let _ = sfu.remove_participant(session_id).await;
    }

    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_user_left",
        &call.channel_id,
        serde_json::json!({
            "user_id": encode_mm_id(user_id),
            "session_id": session_id.to_string(),
        }),
        None,
    )
    .await;

    let remaining =
        reconcile_after_participant_left(state, call.call_id, call.channel_id, user_id).await;
    if remaining <= 1 {
        schedule_empty_call_timeout(state, call.call_id, call.channel_id);
    }

    Ok(())
}
/// Best-effort cleanup for abrupt websocket disconnects where no explicit
/// calls_leave websocket action was delivered.
pub async fn handle_ws_connection_closed(state: &AppState, user_id: Uuid, connection_id: &str) {
    let Ok(session_id) = Uuid::parse_str(connection_id) else {
        return;
    };
    let Some(call) = find_call_for_session(state, user_id, session_id).await else {
        return;
    };

    state
        .call_state_manager
        .remove_participant(call.call_id, user_id)
        .await;
    if let Some(sfu) = state.sfu_manager.get_sfu(call.call_id).await {
        let _ = sfu.remove_participant(session_id).await;
    }

    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_user_left",
        &call.channel_id,
        serde_json::json!({
            "user_id": encode_mm_id(user_id),
            "session_id": session_id.to_string(),
        }),
        None,
    )
    .await;

    let remaining =
        reconcile_after_participant_left(state, call.call_id, call.channel_id, user_id).await;
    if remaining <= 1 {
        schedule_empty_call_timeout(state, call.call_id, call.channel_id);
    }

    info!(
        user_id = %user_id,
        session_id = %session_id,
        call_id = %call.call_id,
        channel_id = %call.channel_id,
        remaining_participants = remaining,
        "calls.ws cleaned up disconnected participant"
    );
}
async fn handle_ws_mute(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    data: Option<&Value>,
    muted: bool,
) -> Result<(), String> {
    let requested_session_id = resolve_ws_session_uuid(connection_id, data)?;
    let (call, session_id) =
        resolve_call_for_ws_connection(state, user_id, requested_session_id).await?;

    if state
        .call_state_manager
        .get_participant(call.call_id, user_id)
        .await
        .is_none()
    {
        // Recover from transient reconnect races where mute/unmute arrives before join/reconnect
        // has re-associated the user participant state.
        state
            .call_state_manager
            .add_participant(
                call.call_id,
                Participant {
                    user_id,
                    session_id,
                    joined_at: Utc::now().timestamp_millis(),
                    muted: true,
                    screen_sharing: false,
                    hand_raised: false,
                },
            )
            .await;

        broadcast_call_event(
            state,
            "custom_com.mattermost.calls_user_joined",
            &call.channel_id,
            serde_json::json!({
                "user_id": encode_mm_id(user_id),
                "session_id": session_id.to_string(),
                "muted": true,
                "raised_hand": 0,
            }),
            None,
        )
        .await;
    }

    state
        .call_state_manager
        .set_muted(call.call_id, user_id, muted)
        .await;
    broadcast_call_event(
        state,
        if muted {
            "custom_com.mattermost.calls_user_muted"
        } else {
            "custom_com.mattermost.calls_user_unmuted"
        },
        &call.channel_id,
        serde_json::json!({
            "user_id": encode_mm_id(user_id),
            "session_id": session_id.to_string(),
            "muted": muted,
        }),
        None,
    )
    .await;

    Ok(())
}
async fn handle_ws_raise_hand(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    raised: bool,
) -> Result<(), String> {
    let requested_session_id = resolve_ws_session_uuid(connection_id, None)?;
    let (call, session_id) =
        resolve_call_for_ws_connection(state, user_id, requested_session_id).await?;

    state
        .call_state_manager
        .set_hand_raised(call.call_id, user_id, raised)
        .await;
    broadcast_call_event(
        state,
        if raised {
            "custom_com.mattermost.calls_user_raise_hand"
        } else {
            "custom_com.mattermost.calls_user_unraise_hand"
        },
        &call.channel_id,
        serde_json::json!({
            "user_id": encode_mm_id(user_id),
            "session_id": session_id.to_string(),
            "raised_hand": if raised { Utc::now().timestamp_millis() } else { 0 },
        }),
        None,
    )
    .await;

    Ok(())
}
async fn handle_ws_reaction(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    data: Option<&Value>,
) -> Result<(), String> {
    let requested_session_id = resolve_ws_session_uuid(connection_id, data)?;
    let (call, session_id) =
        resolve_call_for_ws_connection(state, user_id, requested_session_id).await?;
    let data = data.ok_or_else(|| "Missing reaction payload".to_string())?;
    let emoji = data
        .get("data")
        .and_then(|v| v.as_str())
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .or_else(|| data.get("data").cloned())
        .unwrap_or_else(|| serde_json::json!({}));
    let reaction = emoji
        .get("literal")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
        .or_else(|| {
            emoji
                .get("name")
                .and_then(|value| value.as_str())
                .map(|name| format!(":{name}:"))
        })
        .unwrap_or_default();
    let timestamp = Utc::now().timestamp_millis();

    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_user_reacted",
        &call.channel_id,
        serde_json::json!({
            "user_id": encode_mm_id(user_id),
            "session_id": session_id.to_string(),
            "reaction": reaction,
            "timestamp": timestamp,
            "emoji": emoji,
        }),
        None,
    )
    .await;

    Ok(())
}
async fn find_call_for_session(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
) -> Option<CallState> {
    let calls = state.call_state_manager.get_all_calls().await;
    calls.into_iter().find(|call| {
        call.participants
            .get(&user_id)
            .map(|p| p.session_id == session_id)
            .unwrap_or(false)
    })
}
async fn resolve_call_for_ws_connection(
    state: &AppState,
    user_id: Uuid,
    requested_session_id: Uuid,
) -> Result<(CallState, Uuid), String> {
    if let Some(call) = find_call_for_session(state, user_id, requested_session_id).await {
        return Ok((call, requested_session_id));
    }

    let user_calls: Vec<(CallState, Uuid)> = state
        .call_state_manager
        .get_all_calls()
        .await
        .into_iter()
        .filter_map(|call| {
            let participant_session_id = call.participants.get(&user_id).map(|p| p.session_id);
            participant_session_id.map(|session_id| (call, session_id))
        })
        .collect();

    if user_calls.len() == 1 {
        let (call, participant_session_id) =
            user_calls.into_iter().next().expect("len checked above");
        warn!(
            user_id = %user_id,
            requested_session_id = %requested_session_id,
            participant_session_id = %participant_session_id,
            call_id = %call.call_id,
            "calls.ws session mismatch recovered using existing participant session"
        );
        Ok((call, participant_session_id))
    } else if user_calls.is_empty() {
        let member_calls = find_member_calls_for_user(state, user_id).await?;
        if member_calls.len() == 1 {
            let call = member_calls.into_iter().next().expect("len checked above");
            warn!(
                user_id = %user_id,
                requested_session_id = %requested_session_id,
                call_id = %call.call_id,
                "calls.ws session lookup recovered using channel membership fallback"
            );
            Ok((call, requested_session_id))
        } else if member_calls.is_empty() {
            Err("No active call found for connection".to_string())
        } else {
            Err("Multiple active calls found for user session resolution".to_string())
        }
    } else {
        Err("Multiple active calls found for user session resolution".to_string())
    }
}
async fn find_member_calls_for_user(
    state: &AppState,
    user_id: Uuid,
) -> Result<Vec<CallState>, String> {
    let calls = state.call_state_manager.get_all_calls().await;
    let mut member_calls = Vec::new();

    for call in calls {
        let member: Option<(Uuid,)> = sqlx::query_as(
            "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id = $2",
        )
        .bind(call.channel_id)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("Database error while resolving call membership: {e}"))?;

        if member.is_some() {
            member_calls.push(call);
        }
    }

    Ok(member_calls)
}
fn resolve_ws_session_uuid(connection_id: &str, data: Option<&Value>) -> Result<Uuid, String> {
    let default_session_id = Uuid::parse_str(connection_id)
        .map_err(|_| format!("Invalid connection ID: {connection_id}"))?;

    let Some(data) = data else {
        return Ok(default_session_id);
    };

    let original_session_id = data
        .get("originalConnID")
        .or_else(|| data.get("original_conn_id"))
        .or_else(|| data.get("originalConnId"))
        .and_then(|value| value.as_str())
        .and_then(|raw| Uuid::parse_str(raw).ok());

    Ok(original_session_id.unwrap_or(default_session_id))
}
fn parse_ws_sdp_payload(data: Option<&Value>) -> Result<String, String> {
    let data = data.ok_or_else(|| "missing payload".to_string())?;
    let data_field = data
        .get("data")
        .ok_or_else(|| "missing payload.data".to_string())?;

    // Try parsing as string first (uncompressed JSON)
    if let Some(text) = data_field.as_str() {
        let parsed = serde_json::from_str::<Value>(text).map_err(|e| e.to_string())?;
        let sdp = parsed
            .get("sdp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing sdp".to_string())?;
        return Ok(sdp.to_string());
    }

    // Parse binary data (compressed)
    let bytes = parse_ws_binary_data(data_field)?;

    // Try as uncompressed UTF-8 first
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
            if let Some(sdp) = parsed.get("sdp").and_then(|v| v.as_str()) {
                return Ok(sdp.to_string());
            }
        }
    }

    // Try zlib decompression (mobile clients send compressed SDP)
    let mut decoder = ZlibDecoder::new(bytes.as_slice());
    let mut decoded = String::new();
    match decoder.read_to_string(&mut decoded) {
        Ok(_) => {
            let parsed = serde_json::from_str::<Value>(&decoded).map_err(|e| e.to_string())?;
            let sdp = parsed
                .get("sdp")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "missing sdp in decompressed data".to_string())?;
            Ok(sdp.to_string())
        }
        Err(e) => Err(format!(
            "zlib decode failed: {e}. Data may not be compressed."
        )),
    }
}
fn parse_ws_ice_payload(
    data: Option<&Value>,
) -> Result<(String, Option<String>, Option<u16>), String> {
    let data = data.ok_or_else(|| "missing payload".to_string())?;
    let raw = data
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing payload.data".to_string())?;
    let parsed = serde_json::from_str::<Value>(raw).map_err(|e| e.to_string())?;

    let candidate = parsed
        .get("candidate")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing candidate".to_string())?
        .to_string();
    let sdp_mid = parsed
        .get("sdpMid")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .or_else(|| {
            parsed
                .get("sdp_mid")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        });
    let sdp_mline_index = parsed
        .get("sdpMLineIndex")
        .and_then(|v| v.as_u64())
        .or_else(|| parsed.get("sdp_mline_index").and_then(|v| v.as_u64()))
        .and_then(|v| u16::try_from(v).ok());

    Ok((candidate, sdp_mid, sdp_mline_index))
}
fn parse_ws_binary_data(value: &Value) -> Result<Vec<u8>, String> {
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_u64()
                    .and_then(|v| u8::try_from(v).ok())
                    .ok_or_else(|| "binary payload contains non-byte value".to_string())
            })
            .collect(),
        Value::Object(map) if map.get("type").and_then(|v| v.as_str()) == Some("Buffer") => map
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "buffer payload missing data array".to_string())?
            .iter()
            .map(|item| {
                item.as_u64()
                    .and_then(|v| u8::try_from(v).ok())
                    .ok_or_else(|| "buffer payload contains non-byte value".to_string())
            })
            .collect(),
        _ => Err("unsupported binary payload shape".to_string()),
    }
}
async fn send_ws_plugin_event(state: &AppState, user_id: Uuid, event: &str, data: Value) {
    let envelope = WsEnvelope {
        msg_type: "event".to_string(),
        event: event.to_string(),
        seq: None,
        channel_id: None,
        data,
        broadcast: Some(WsBroadcast {
            channel_id: None,
            team_id: None,
            user_id: Some(user_id),
            exclude_user_id: None,
        }),
    };
    state.ws_hub.broadcast(envelope).await;
}
async fn send_ws_plugin_error(state: &AppState, user_id: Uuid, connection_id: &str, message: &str) {
    send_ws_plugin_event(
        state,
        user_id,
        "custom_com.mattermost.calls_error",
        serde_json::json!({
            "connID": connection_id,
            "conn_id": connection_id,
            "error": message,
        }),
    )
    .await;
}
async fn send_ws_plugin_signal(
    state: &AppState,
    user_id: Uuid,
    connection_id: &str,
    signal: Value,
) {
    send_ws_plugin_event(
        state,
        user_id,
        "custom_com.mattermost.calls_signal",
        serde_json::json!({
            "connID": connection_id,
            "conn_id": connection_id,
            "data": signal.to_string(),
            "signal": signal,
        }),
    )
    .await;
}
fn parse_join_channel_id(data: &Value) -> Result<Uuid, String> {
    let raw = data
        .get("channelID")
        .or_else(|| data.get("channel_id"))
        .or_else(|| data.get("channelId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing channel ID in join payload".to_string())?;

    parse_mm_or_uuid(raw).ok_or_else(|| format!("Invalid channel ID: {raw}"))
}
