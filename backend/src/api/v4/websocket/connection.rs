use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::WebSocket;
use chrono::{DateTime, Utc};
use serde_json::json;
use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use crate::api::v4::calls_plugin;
use crate::api::websocket_core::{self, EnvelopeCommandOptions};
use crate::api::AppState;
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::realtime::{
    websocket_actor::{close_codes, WebSocketActor, WsEvent},
    WsBroadcast, WsEnvelope,
};
use crate::telemetry::metrics;

use super::resumption::{send_reconnect_snapshot_if_needed, should_send_reconnect_snapshot};

pub(crate) async fn run_connection(
    socket: WebSocket,
    state: AppState,
    user_id: Uuid,
    token_expires_at: DateTime<Utc>,
    connection_id: Option<String>,
    sequence_number: Option<i64>,
    addr: Option<SocketAddr>,
) {
    if token_expires_at <= Utc::now() {
        warn!(
            user_id = %user_id,
            token_expires_at = %token_expires_at,
            "Rejecting websocket connection because token is already expired"
        );
        return;
    }

    // Check connection limits
    if let Err(limit) = websocket_core::enforce_connection_limit(&state, user_id).await {
        warn!(
            user_id = %user_id,
            current = limit.current,
            max = limit.max,
            "Too many connections for user"
        );

        // Send close frame and return
        // Note: In axum 0.8, we can't easily split the socket, so we just drop it
        // The client will see the connection close
        return;
    }

    // Get or create connection store
    let store = state.connection_store.clone();
    let replay_store = store.clone();

    // Treat empty connection IDs as "not provided" (fresh connection),
    // matching Mattermost reliable websocket semantics.
    let requested_connection_id = connection_id.filter(|id| !id.is_empty());
    let is_resumption_attempt = requested_connection_id.is_some();

    // Create WebSocket actor with session resumption
    let (actor, missed_messages) = WebSocketActor::new(
        socket,
        store,
        user_id,
        requested_connection_id.clone(),
        sequence_number,
        addr,
    )
    .await;

    let actor_connection_id = actor.connection_id.clone();
    let is_resumed = !missed_messages.is_empty() || is_resumption_attempt;
    let should_send_reconnect_snapshot =
        should_send_reconnect_snapshot(requested_connection_id.as_deref(), sequence_number);

    info!(
        connection_id = %actor_connection_id,
        user_id = %user_id,
        resumed = is_resumed,
        missed_count = missed_messages.len(),
        addr = ?addr,
        "WebSocket connection established"
    );

    // Get username
    let username = match websocket_core::fetch_username(&state, user_id).await {
        Ok(name) => name,
        Err(_) => {
            error!(user_id = %user_id, "Failed to get username");
            actor.close(close_codes::INTERNAL_ERROR, "User not found");
            return;
        }
    };

    // Add connection to hub.
    // Presence tracking must use a per-socket unique id instead of the resumable
    // actor connection id, otherwise reconnect races can unregister an active
    // socket that reused the same actor connection id.
    let (hub_conn_id, mut hub_rx) = state.ws_hub.add_connection(user_id, username.clone()).await;
    let presence_connection_id = hub_conn_id.to_string();
    websocket_core::register_presence_connection(&state, user_id, &presence_connection_id).await;

    websocket_core::initialize_connection_state(&state, user_id, true).await;

    let heartbeat_state = state.clone();
    let heartbeat_connection_id = presence_connection_id.clone();

    let heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            websocket_core::heartbeat_presence_connection(
                &heartbeat_state,
                user_id,
                &heartbeat_connection_id,
            )
            .await;
        }
    });

    // Send hello event. Mattermost reliable websocket clients reset their local sequence
    // to 0 whenever connection_id changes, so hello.seq must also be 0 in that case.
    // Only preserve requested sequence when we truly resumed the same connection_id.
    let requested_seq = sequence_number.unwrap_or(0).max(0);
    let resumed_same_connection = requested_connection_id
        .as_deref()
        .map(|id| id == actor_connection_id.as_str())
        .unwrap_or(false);
    let hello_seq = if resumed_same_connection {
        requested_seq
    } else {
        0
    };
    info!(
        connection_id = %actor_connection_id,
        requested_connection_id = ?requested_connection_id,
        requested_seq = requested_seq,
        hello_seq = hello_seq,
        resumed_same_connection = resumed_same_connection,
        "Prepared hello message"
    );
    let hello = mm::WebSocketMessage {
        seq: Some(hello_seq),
        event: "hello".to_string(),
        data: json!({
            "connection_id": actor_connection_id.clone(),
            "server_version": format!("rustchat-{}", env!("CARGO_PKG_VERSION")),
            "protocol_version": "1.0"
        }),
        broadcast: mm::Broadcast {
            omit_users: None,
            user_id: String::new(),
            channel_id: String::new(),
            team_id: String::new(),
        },
    };

    if let Err(e) = actor.send(hello) {
        warn!(
            connection_id = %actor_connection_id,
            error = %e,
            "Failed to send hello message"
        );
        return;
    }

    // Replay missed messages if resuming
    for msg in missed_messages {
        if let Err(e) = actor.send(msg) {
            warn!(
                connection_id = %actor_connection_id,
                error = %e,
                "Failed to send missed message"
            );
            break;
        } else {
            metrics::record_ws_message("sent", "replay");
        }
    }

    // After hello/replay, proactively push a full state snapshot for reconnects.
    send_reconnect_snapshot_if_needed(
        &state,
        &actor,
        user_id,
        &actor_connection_id,
        should_send_reconnect_snapshot,
    )
    .await;

    // Main event loop
    let actor_clone = actor.clone();
    let replay_connection_id = actor_connection_id.clone();

    // Spawn task to forward hub messages to client
    let mut hub_forward_task = tokio::spawn(async move {
        loop {
            let msg_str = match hub_rx.recv().await {
                Ok(msg) => msg,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    metrics::record_ws_dropped("hub_receiver_lagged", skipped);
                    warn!(
                        connection_id = %replay_connection_id,
                        skipped = skipped,
                        "Hub receiver lagged; dropping stale websocket messages"
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    debug!(
                        connection_id = %replay_connection_id,
                        "Hub receiver closed"
                    );
                    break;
                }
            };

            let envelope = match serde_json::from_str::<WsEnvelope>(&msg_str) {
                Ok(value) => value,
                Err(err) => {
                    warn!(
                        connection_id = %replay_connection_id,
                        error = %err,
                        "Failed to deserialize hub websocket envelope"
                    );
                    continue;
                }
            };

            let Some(mut mm_msg) = map_envelope_to_mm(&envelope) else {
                continue;
            };

            let replay_payload = json!({
                "event": mm_msg.event.clone(),
                "data": mm_msg.data.clone(),
                "broadcast": mm_msg.broadcast.clone(),
            });

            if let Some(seq) = replay_store.queue_message(&replay_connection_id, replay_payload) {
                mm_msg.seq = Some(seq);
            }

            if actor_clone.send(mm_msg).is_err() {
                metrics::record_ws_dropped("actor_send_failed", 1);
                break;
            } else {
                metrics::record_ws_message("sent", "hub_event");
            }
        }
    });

    let token_ttl = token_expires_at
        .signed_duration_since(Utc::now())
        .to_std()
        .unwrap_or(Duration::ZERO);
    let auth_expiry_sleep = sleep(token_ttl);
    tokio::pin!(auth_expiry_sleep);

    // Handle events from WebSocket actor
    loop {
        tokio::select! {
            // Handle events from the WebSocket actor
            event = actor.recv() => {
                match event {
                    Some(WsEvent::MessageReceived(text)) => {
                        metrics::record_ws_message("received", "client_text");
                        handle_client_text_message(
                            &state,
                            &actor,
                            user_id,
                            &username,
                            &actor_connection_id,
                            &text,
                        )
                        .await;
                    }
                    Some(WsEvent::BinaryReceived(bytes)) => {
                        metrics::record_ws_message("received", "client_binary");
                        handle_client_binary_message(
                            &state,
                            &actor,
                            user_id,
                            &username,
                            &actor_connection_id,
                            &bytes,
                        )
                        .await;
                    }
                    Some(WsEvent::PongReceived) => {
                        trace!(connection_id = %actor_connection_id, "Pong received");
                    }
                    Some(WsEvent::Closed(reason)) => {
                        info!(
                            connection_id = %actor_connection_id,
                            code = reason.code,
                            reason = %reason.reason,
                            "Connection closed"
                        );
                        break;
                    }
                    Some(WsEvent::Error(e)) => {
                        error!(
                            connection_id = %actor_connection_id,
                            error = %e,
                            "Connection error"
                        );
                        break;
                    }
                    None => {
                        debug!(connection_id = %actor_connection_id, "Event channel closed");
                        break;
                    }
                }
            }

            // If hub forward task ends, we should also close
            _ = &mut hub_forward_task => {
                debug!(connection_id = %actor_connection_id, "Hub forward task ended");
                break;
            }
            _ = &mut auth_expiry_sleep => {
                info!(
                    connection_id = %actor_connection_id,
                    user_id = %user_id,
                    token_expires_at = %token_expires_at,
                    "Closing websocket because authentication token expired"
                );
                actor.close(close_codes::POLICY_VIOLATION, "Authentication token expired");
                break;
            }
        }
    }

    // Cleanup
    hub_forward_task.abort();
    heartbeat_task.abort();

    // Mark connection as disconnected (for potential resumption)
    actor.disconnect();

    // Calls websocket clients may disconnect without sending an explicit
    // `leave` action. Apply a short grace window for reconnect and then
    // best-effort participant cleanup if this exact session is still inactive.
    let disconnect_state = state.clone();
    let disconnect_connection_id = actor_connection_id.clone();
    tokio::spawn(async move {
        sleep(Duration::from_secs(3)).await;
        let still_inactive = disconnect_state
            .connection_store
            .get_connection(&disconnect_connection_id)
            .map(|conn| !conn.is_active.load(Ordering::SeqCst))
            .unwrap_or(true);
        if still_inactive {
            calls_plugin::handle_ws_connection_closed(
                &disconnect_state,
                user_id,
                &disconnect_connection_id,
            )
            .await;
        }
    });

    // Remove from hub
    state.ws_hub.remove_connection(user_id, hub_conn_id).await;
    websocket_core::handle_disconnect(&state, user_id, &presence_connection_id).await;

    info!(
        connection_id = %actor_connection_id,
        user_id = %user_id,
        "WebSocket connection ended"
    );
}

pub(crate) async fn handle_client_text_message(
    state: &AppState,
    actor: &Arc<WebSocketActor>,
    user_id: Uuid,
    username: &str,
    connection_id: &str,
    text: &str,
) {
    trace!(
        user_id = %user_id,
        connection_id = connection_id,
        text = %text,
        "Received client message"
    );

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        handle_client_value_message(state, actor, user_id, username, connection_id, &value).await;
    } else {
        // Handle plain text calls actions (e.g., mobile sends "mute", "unmute" as plain text)
        let trimmed = text.trim();
        let calls_actions = ["mute", "unmute", "raise_hand", "unraise_hand", "leave"];
        if calls_actions.contains(&trimmed) {
            let action = format!("custom_com.mattermost.calls_{}", trimmed);
            let _ =
                calls_plugin::handle_ws_action(state, user_id, connection_id, &action, None).await;
        }
    }

    let _ = websocket_core::handle_client_envelope_message(
        state,
        user_id,
        username,
        text,
        EnvelopeCommandOptions::V4,
    )
    .await;
}

pub(crate) async fn handle_client_binary_message(
    state: &AppState,
    actor: &Arc<WebSocketActor>,
    user_id: Uuid,
    username: &str,
    connection_id: &str,
    bytes: &[u8],
) {
    if let Some(value) = decode_msgpack_value(bytes) {
        trace!(
            user_id = %user_id,
            connection_id = connection_id,
            "Received binary client message"
        );
        handle_client_value_message(state, actor, user_id, username, connection_id, &value).await;
    } else {
        warn!(
            user_id = %user_id,
            connection_id = connection_id,
            "Failed to decode binary websocket message as msgpack"
        );
    }
}

pub(crate) async fn handle_client_value_message(
    state: &AppState,
    actor: &Arc<WebSocketActor>,
    user_id: Uuid,
    username: &str,
    connection_id: &str,
    value: &serde_json::Value,
) {
    let Some(action) = value
        .get("action")
        .or_else(|| value.get("event"))
        .and_then(|v| v.as_str())
    else {
        return;
    };

    if calls_plugin::handle_ws_action(state, user_id, connection_id, action, value.get("data"))
        .await
    {
        return;
    }

    if action == "ping" {
        let seq_reply = value.get("seq").cloned().unwrap_or(serde_json::Value::Null);
        let server_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default();
        let response = json!({
            "status": "OK",
            "seq_reply": seq_reply,
            "data": {
                "text": "pong",
                "version": format!("rustchat-{}", env!("CARGO_PKG_VERSION")),
                "server_time": server_time_ms,
                "node_id": ""
            }
        });

        if let Err(err) = actor.send_raw(response) {
            if err.contains("closing") {
                debug!(
                    user_id = %user_id,
                    connection_id = connection_id,
                    error = %err,
                    "Skipping ping response on closing websocket"
                );
            } else {
                warn!(
                    user_id = %user_id,
                    connection_id = connection_id,
                    error = %err,
                    "Failed to send ping response"
                );
            }
        }
    } else if matches!(action, "reconnect" | "get_initial_load" | "initial_load") {
        let seq_reply = value.get("seq").cloned().unwrap_or(serde_json::Value::Null);
        let response = json!({
            "status": "OK",
            "seq_reply": seq_reply
        });
        let _ = actor.send_raw(response);
        send_reconnect_snapshot_if_needed(state, actor, user_id, connection_id, true).await;
    } else if matches!(action, "user_typing" | "typing" | "typing_start") {
        let channel_id = extract_typing_channel_id(value);
        if let Some(channel_id) = channel_id {
            let broadcast = WsEnvelope::event(
                crate::realtime::EventType::UserTyping,
                crate::realtime::TypingEvent {
                    user_id,
                    display_name: username.to_string(),
                    thread_root_id: extract_typing_thread_root_id(value),
                },
                Some(channel_id),
            )
            .with_broadcast(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: Some(user_id),
            });
            state.ws_hub.broadcast(broadcast).await;
        }
    } else if matches!(action, "user_typing_stop" | "stop_typing" | "typing_stop") {
        let channel_id = extract_typing_channel_id(value);
        if let Some(channel_id) = channel_id {
            let broadcast = WsEnvelope::event(
                crate::realtime::EventType::UserTypingStop,
                crate::realtime::TypingEvent {
                    user_id,
                    display_name: username.to_string(),
                    thread_root_id: extract_typing_thread_root_id(value),
                },
                Some(channel_id),
            )
            .with_broadcast(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: Some(user_id),
            });
            state.ws_hub.broadcast(broadcast).await;
        }
    } else {
        trace!(action = %action, "Unknown action received");
    }
}

fn extract_typing_channel_id(value: &serde_json::Value) -> Option<Uuid> {
    value
        .get("data")
        .and_then(|v| v.get("channel_id"))
        .and_then(|v| v.as_str())
        .and_then(parse_mm_or_uuid)
        .or_else(|| {
            value
                .get("channel_id")
                .and_then(|v| v.as_str())
                .and_then(parse_mm_or_uuid)
        })
}

fn extract_typing_thread_root_id(value: &serde_json::Value) -> Option<Uuid> {
    let data = value.get("data");
    data.and_then(|v| v.get("parent_id"))
        .and_then(|v| v.as_str())
        .and_then(parse_mm_or_uuid)
        .or_else(|| {
            data.and_then(|v| v.get("thread_root_id"))
                .and_then(|v| v.as_str())
                .and_then(parse_mm_or_uuid)
        })
        .or_else(|| {
            value
                .get("parent_id")
                .and_then(|v| v.as_str())
                .and_then(parse_mm_or_uuid)
        })
        .or_else(|| {
            value
                .get("thread_root_id")
                .and_then(|v| v.as_str())
                .and_then(parse_mm_or_uuid)
        })
}

fn decode_msgpack_value(bytes: &[u8]) -> Option<serde_json::Value> {
    let mut idx = 0usize;
    decode_msgpack_at(bytes, &mut idx)
}

fn decode_msgpack_at(bytes: &[u8], idx: &mut usize) -> Option<serde_json::Value> {
    let marker = *bytes.get(*idx)?;
    *idx += 1;

    match marker {
        0x00..=0x7f => Some(serde_json::Value::from(marker as u64)),
        0xe0..=0xff => Some(serde_json::Value::from((marker as i8) as i64)),
        0xc0 => Some(serde_json::Value::Null),
        0xc2 => Some(serde_json::Value::Bool(false)),
        0xc3 => Some(serde_json::Value::Bool(true)),
        0xcc => Some(serde_json::Value::from(read_u8(bytes, idx)? as u64)),
        0xcd => Some(serde_json::Value::from(read_u16(bytes, idx)? as u64)),
        0xce => Some(serde_json::Value::from(read_u32(bytes, idx)? as u64)),
        0xd0 => Some(serde_json::Value::from(read_i8(bytes, idx)? as i64)),
        0xd1 => Some(serde_json::Value::from(read_i16(bytes, idx)? as i64)),
        0xd2 => Some(serde_json::Value::from(read_i32(bytes, idx)? as i64)),
        0xa0..=0xbf => {
            let len = (marker & 0x1f) as usize;
            decode_str(bytes, idx, len)
        }
        0xd9 => {
            let len = read_u8(bytes, idx)? as usize;
            decode_str(bytes, idx, len)
        }
        0xda => {
            let len = read_u16(bytes, idx)? as usize;
            decode_str(bytes, idx, len)
        }
        0xdb => {
            let len = read_u32(bytes, idx)? as usize;
            decode_str(bytes, idx, len)
        }
        0xc4 => {
            let len = read_u8(bytes, idx)? as usize;
            decode_bin_as_json_array(bytes, idx, len)
        }
        0xc5 => {
            let len = read_u16(bytes, idx)? as usize;
            decode_bin_as_json_array(bytes, idx, len)
        }
        0xc6 => {
            let len = read_u32(bytes, idx)? as usize;
            decode_bin_as_json_array(bytes, idx, len)
        }
        0x90..=0x9f => decode_array(bytes, idx, (marker & 0x0f) as usize),
        0xdc => {
            let len = read_u16(bytes, idx)? as usize;
            decode_array(bytes, idx, len)
        }
        0xdd => {
            let len = read_u32(bytes, idx)? as usize;
            decode_array(bytes, idx, len)
        }
        0x80..=0x8f => decode_map(bytes, idx, (marker & 0x0f) as usize),
        0xde => {
            let len = read_u16(bytes, idx)? as usize;
            decode_map(bytes, idx, len)
        }
        0xdf => {
            let len = read_u32(bytes, idx)? as usize;
            decode_map(bytes, idx, len)
        }
        _ => None,
    }
}

fn decode_array(bytes: &[u8], idx: &mut usize, len: usize) -> Option<serde_json::Value> {
    let mut items = Vec::with_capacity(len);
    for _ in 0..len {
        items.push(decode_msgpack_at(bytes, idx)?);
    }
    Some(serde_json::Value::Array(items))
}

fn decode_map(bytes: &[u8], idx: &mut usize, len: usize) -> Option<serde_json::Value> {
    let mut map = serde_json::Map::with_capacity(len);
    for _ in 0..len {
        let key = decode_msgpack_at(bytes, idx)?.as_str()?.to_string();
        let value = decode_msgpack_at(bytes, idx)?;
        map.insert(key, value);
    }
    Some(serde_json::Value::Object(map))
}

fn decode_str(bytes: &[u8], idx: &mut usize, len: usize) -> Option<serde_json::Value> {
    let slice = read_exact(bytes, idx, len)?;
    let text = std::str::from_utf8(slice).ok()?.to_string();
    Some(serde_json::Value::String(text))
}

fn decode_bin_as_json_array(
    bytes: &[u8],
    idx: &mut usize,
    len: usize,
) -> Option<serde_json::Value> {
    let slice = read_exact(bytes, idx, len)?;
    Some(serde_json::Value::Array(
        slice
            .iter()
            .map(|b| serde_json::Value::from(*b as u64))
            .collect(),
    ))
}

fn read_exact<'a>(bytes: &'a [u8], idx: &mut usize, len: usize) -> Option<&'a [u8]> {
    let end = idx.checked_add(len)?;
    let slice = bytes.get(*idx..end)?;
    *idx = end;
    Some(slice)
}

fn read_u8(bytes: &[u8], idx: &mut usize) -> Option<u8> {
    let value = *bytes.get(*idx)?;
    *idx += 1;
    Some(value)
}

fn read_i8(bytes: &[u8], idx: &mut usize) -> Option<i8> {
    read_u8(bytes, idx).map(|v| v as i8)
}

fn read_u16(bytes: &[u8], idx: &mut usize) -> Option<u16> {
    let arr: [u8; 2] = read_exact(bytes, idx, 2)?.try_into().ok()?;
    Some(u16::from_be_bytes(arr))
}

fn read_i16(bytes: &[u8], idx: &mut usize) -> Option<i16> {
    let arr: [u8; 2] = read_exact(bytes, idx, 2)?.try_into().ok()?;
    Some(i16::from_be_bytes(arr))
}

fn read_u32(bytes: &[u8], idx: &mut usize) -> Option<u32> {
    let arr: [u8; 4] = read_exact(bytes, idx, 4)?.try_into().ok()?;
    Some(u32::from_be_bytes(arr))
}

fn read_i32(bytes: &[u8], idx: &mut usize) -> Option<i32> {
    let arr: [u8; 4] = read_exact(bytes, idx, 4)?.try_into().ok()?;
    Some(i32::from_be_bytes(arr))
}

/// Map internal envelope to Mattermost-compatible message
pub fn map_envelope_to_mm(env: &WsEnvelope) -> Option<mm::WebSocketMessage> {
    let seq = None; // Will be assigned by actor

    match env.event.as_str() {
        "posted" | "thread_reply_created" => {
            let mm_post = if let Ok(post_resp) =
                serde_json::from_value::<crate::models::post::PostResponse>(env.data.clone())
            {
                let mapped: mm::Post = post_resp.into();
                mapped
            } else if let Ok(post) = serde_json::from_value::<mm::Post>(env.data.clone()) {
                post
            } else {
                return None;
            };

            let post_json = serde_json::to_string(&mm_post).unwrap_or_default();

            let data = json!({
                "post": post_json,
                "channel_display_name": "",
                "channel_name": "",
                "channel_type": "O",
                "sender_name": mm_post.user_id,
                "team_id": ""
            });

            Some(mm::WebSocketMessage {
                seq,
                event: "posted".to_string(),
                data,
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "typing" | "typing_start" => {
            if let Ok(typing) =
                serde_json::from_value::<crate::realtime::TypingEvent>(env.data.clone())
            {
                let parent_id = typing.thread_root_id.map(encode_mm_id).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq,
                    // Mattermost clients (web/mobile) dispatch typing indicators from `typing`.
                    event: "typing".to_string(),
                    data: json!({
                        "parent_id": parent_id,
                        "user_id": encode_mm_id(typing.user_id),
                        "display_name": typing.display_name,
                        "thread_root_id": parent_id,
                    }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "stop_typing" | "typing_stop" => {
            if let Ok(typing) =
                serde_json::from_value::<crate::realtime::TypingEvent>(env.data.clone())
            {
                let parent_id = typing.thread_root_id.map(encode_mm_id).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq,
                    // Mattermost clients dispatch stop-typing from `stop_typing`.
                    event: "stop_typing".to_string(),
                    data: json!({
                        "parent_id": parent_id,
                        "user_id": encode_mm_id(typing.user_id),
                        "thread_root_id": parent_id,
                    }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "post_edited" | "thread_reply_updated" => {
            if let Ok(post_resp) =
                serde_json::from_value::<crate::models::post::PostResponse>(env.data.clone())
            {
                let mm_post: mm::Post = post_resp.into();
                let post_json = serde_json::to_string(&mm_post).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq,
                    event: "post_edited".to_string(),
                    data: json!({ "post": post_json }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "post_deleted" | "thread_reply_deleted" => {
            if let Ok(post_resp) =
                serde_json::from_value::<crate::models::post::PostResponse>(env.data.clone())
            {
                let mm_post: mm::Post = post_resp.into();
                let post_json = serde_json::to_string(&mm_post).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq,
                    event: "post_deleted".to_string(),
                    data: json!({ "post": post_json }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "reaction_added" => {
            if let Ok(reaction) =
                serde_json::from_value::<crate::models::reaction::Reaction>(env.data.clone())
            {
                let mm_reaction = mm::Reaction {
                    user_id: encode_mm_id(reaction.user_id),
                    post_id: encode_mm_id(reaction.post_id),
                    emoji_name: crate::mattermost_compat::emoji_data::get_short_name_for_emoji(
                        &reaction.emoji_name,
                    ),
                    create_at: reaction.create_at,
                    update_at: reaction.create_at,
                    delete_at: 0,
                    channel_id: env.channel_id.map(encode_mm_id).unwrap_or_default(),
                    remote_id: "".to_string(),
                };
                let reaction_json = serde_json::to_string(&mm_reaction).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq,
                    event: "reaction_added".to_string(),
                    data: json!({ "reaction": reaction_json }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "reaction_removed" => {
            if let Ok(reaction) =
                serde_json::from_value::<crate::models::reaction::Reaction>(env.data.clone())
            {
                let mm_reaction = mm::Reaction {
                    user_id: encode_mm_id(reaction.user_id),
                    post_id: encode_mm_id(reaction.post_id),
                    emoji_name: crate::mattermost_compat::emoji_data::get_short_name_for_emoji(
                        &reaction.emoji_name,
                    ),
                    create_at: reaction.create_at,
                    update_at: reaction.create_at,
                    delete_at: 0,
                    channel_id: env.channel_id.map(encode_mm_id).unwrap_or_default(),
                    remote_id: "".to_string(),
                };
                let reaction_json = serde_json::to_string(&mm_reaction).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq,
                    event: "reaction_removed".to_string(),
                    data: json!({ "reaction": reaction_json }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "user_updated" | "status_change" => {
            if let Some(status_str) = env.data.get("status").and_then(|v| v.as_str()) {
                let user_id = env
                    .data
                    .get("user_id")
                    .and_then(|v| v.as_str())
                    .and_then(parse_mm_or_uuid)
                    .map(encode_mm_id)
                    .unwrap_or_default();

                // Extract additional fields if available
                let manual = env
                    .data
                    .get("manual")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let last_activity_at = env
                    .data
                    .get("last_activity_at")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                Some(mm::WebSocketMessage {
                    seq,
                    event: "status_change".to_string(),
                    data: json!({
                        "user_id": user_id,
                        "status": status_str,
                        "manual": manual,
                        "last_activity_at": last_activity_at,
                        "text": env.data.get("text").cloned().unwrap_or(serde_json::Value::Null),
                        "emoji": env.data.get("emoji").cloned().unwrap_or(serde_json::Value::Null),
                        "expires_at": env.data.get("expires_at").cloned().unwrap_or(serde_json::Value::Null)
                    }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "channel_viewed" => {
            let channel_id = extract_mm_id(env.data.get("channel_id"));
            Some(mm::WebSocketMessage {
                seq,
                event: "channel_viewed".to_string(),
                data: json!({ "channel_id": channel_id }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "post_unread" => {
            let channel_id = extract_mm_id(env.data.get("channel_id"));
            let post_id = extract_mm_id(env.data.get("post_id"));
            Some(mm::WebSocketMessage {
                seq,
                event: "post_unread".to_string(),
                data: json!({
                    "channel_id": channel_id,
                    "post_id": post_id,
                    "msg_count": extract_i64(env.data.get("msg_count")),
                    "msg_count_root": extract_i64(env.data.get("msg_count_root")),
                    "mention_count": extract_i64(env.data.get("mention_count")),
                    "mention_count_root": extract_i64(env.data.get("mention_count_root")),
                    "urgent_mention_count": extract_i64(env.data.get("urgent_mention_count")),
                    "last_viewed_at": extract_i64(env.data.get("last_viewed_at")),
                }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "thread_updated" => Some(mm::WebSocketMessage {
            seq,
            event: "thread_updated".to_string(),
            data: env.data.clone(),
            broadcast: map_broadcast(env.broadcast.as_ref()),
        }),
        "user_added" => {
            let user_id = extract_mm_id(env.data.get("user_id"));
            let channel_id = extract_mm_id(env.data.get("channel_id"));
            let team_id = extract_mm_id(env.data.get("team_id"));
            Some(mm::WebSocketMessage {
                seq,
                event: "user_added".to_string(),
                data: json!({
                    "user_id": user_id,
                    "channel_id": channel_id,
                    "team_id": team_id,
                }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "user_removed" => {
            let user_id = extract_mm_id(env.data.get("user_id"));
            let remover_id = extract_mm_id(env.data.get("remover_id"));
            Some(mm::WebSocketMessage {
                seq,
                event: "user_removed".to_string(),
                data: json!({
                    "user_id": user_id,
                    "remover_id": remover_id,
                }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "received_group" => {
            let group = extract_embedded_json(env.data.get("group"))?;
            Some(mm::WebSocketMessage {
                seq,
                event: "received_group".to_string(),
                data: json!({
                    "group": group,
                }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "group_member_add" => {
            let group_member = extract_embedded_json(env.data.get("group_member"))?;
            Some(mm::WebSocketMessage {
                seq,
                event: "group_member_add".to_string(),
                data: json!({
                    "group_member": group_member,
                }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "group_member_deleted" => {
            let group_member = extract_embedded_json(env.data.get("group_member"))?;
            Some(mm::WebSocketMessage {
                seq,
                event: "group_member_deleted".to_string(),
                data: json!({
                    "group_member": group_member,
                }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        "received_group_associated_to_team"
        | "received_group_not_associated_to_team"
        | "received_group_associated_to_channel"
        | "received_group_not_associated_to_channel" => Some(mm::WebSocketMessage {
            seq,
            event: env.event.clone(),
            data: json!({
                "group_id": extract_mm_id(env.data.get("group_id")),
            }),
            broadcast: map_broadcast(env.broadcast.as_ref()),
        }),
        "first_admin_visit_marketplace_status_received" => Some(mm::WebSocketMessage {
            seq,
            event: "first_admin_visit_marketplace_status_received".to_string(),
            data: env.data.clone(),
            broadcast: map_broadcast(env.broadcast.as_ref()),
        }),
        event_name if event_name.starts_with("custom_") => Some(mm::WebSocketMessage {
            seq,
            event: event_name.to_string(),
            data: env.data.clone(),
            broadcast: map_broadcast(env.broadcast.as_ref()),
        }),
        _ => None,
    }
}

fn extract_mm_id(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|v| v.as_str())
        .and_then(parse_mm_or_uuid)
        .map(encode_mm_id)
        .unwrap_or_default()
}

fn extract_i64(value: Option<&serde_json::Value>) -> i64 {
    value
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_u64().and_then(|n| i64::try_from(n).ok()))
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .unwrap_or_default()
}

fn extract_embedded_json(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        Some(text.to_string())
    } else {
        serde_json::to_string(value).ok()
    }
}

fn map_broadcast(b_opt: Option<&crate::realtime::WsBroadcast>) -> mm::Broadcast {
    if let Some(b) = b_opt {
        mm::Broadcast {
            omit_users: None,
            user_id: b.user_id.map(encode_mm_id).unwrap_or_default(),
            channel_id: b.channel_id.map(encode_mm_id).unwrap_or_default(),
            team_id: b.team_id.map(encode_mm_id).unwrap_or_default(),
        }
    } else {
        mm::Broadcast {
            omit_users: None,
            user_id: String::new(),
            channel_id: String::new(),
            team_id: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::map_envelope_to_mm;
    use crate::mattermost_compat::id::encode_mm_id;
    use crate::mattermost_compat::models as mm;
    use crate::realtime::{WsBroadcast, WsEnvelope};
    use uuid::Uuid;

    #[test]
    fn map_envelope_to_mm_passes_custom_events() {
        let channel_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "custom_com.mattermost.calls_signal".to_string(),
            seq: None,
            channel_id: Some(channel_id),
            data: serde_json::json!({
                "signal": { "type": "answer", "sdp": "v=0" }
            }),
            broadcast: Some(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: None,
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("custom event should map");
        assert_eq!(mapped.event, "custom_com.mattermost.calls_signal");
        assert_eq!(mapped.data["signal"]["type"], "answer");
    }

    #[test]
    fn map_envelope_to_mm_maps_posted_from_mm_post_payload() {
        let channel_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "posted".to_string(),
            seq: None,
            channel_id: Some(channel_id),
            data: serde_json::json!({
                "id": "post123",
                "create_at": 1739500000000i64,
                "update_at": 1739500000000i64,
                "delete_at": 0,
                "edit_at": 0,
                "user_id": "user123",
                "channel_id": "channel123",
                "root_id": "root123",
                "original_id": "",
                "message": "hello from mm payload",
                "type": "",
                "props": {},
                "hashtags": "",
                "file_ids": [],
                "pending_post_id": ""
            }),
            broadcast: Some(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: None,
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("posted event should map");
        assert_eq!(mapped.event, "posted");
        let post_json = mapped.data["post"]
            .as_str()
            .expect("mapped post payload should be a JSON string");
        let post: mm::Post = serde_json::from_str(post_json).expect("post JSON should deserialize");
        assert_eq!(post.id, "post123");
        assert_eq!(post.root_id, "root123");
        assert_eq!(post.message, "hello from mm payload");
    }

    #[test]
    fn map_envelope_to_mm_maps_typing_event_name_to_typing() {
        let channel_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let root_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "typing_start".to_string(),
            seq: None,
            channel_id: Some(channel_id),
            data: serde_json::json!({
                "user_id": user_id,
                "display_name": "alice",
                "thread_root_id": root_id
            }),
            broadcast: Some(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: Some(user_id),
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("typing event should map");
        assert_eq!(mapped.event, "typing");
        assert_eq!(
            mapped.data["user_id"],
            serde_json::json!(encode_mm_id(user_id))
        );
        assert_eq!(
            mapped.data["parent_id"],
            serde_json::json!(encode_mm_id(root_id))
        );
        assert_eq!(
            mapped.broadcast.channel_id,
            encode_mm_id(channel_id),
            "typing channel must be routed via broadcast.channel_id"
        );
    }

    #[test]
    fn map_envelope_to_mm_maps_typing_stop_event_name_to_stop_typing() {
        let channel_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "typing_stop".to_string(),
            seq: None,
            channel_id: Some(channel_id),
            data: serde_json::json!({
                "user_id": user_id,
                "display_name": "alice",
                "thread_root_id": serde_json::Value::Null
            }),
            broadcast: Some(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: Some(user_id),
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("stop typing event should map");
        assert_eq!(mapped.event, "stop_typing");
        assert_eq!(
            mapped.data["user_id"],
            serde_json::json!(encode_mm_id(user_id))
        );
        assert_eq!(mapped.data["parent_id"], serde_json::json!(""));
    }

    #[test]
    fn map_envelope_to_mm_maps_post_unread_payload() {
        let channel_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let post_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "post_unread".to_string(),
            seq: None,
            channel_id: Some(channel_id),
            data: serde_json::json!({
                "channel_id": channel_id,
                "post_id": post_id,
                "msg_count": 12,
                "msg_count_root": 7,
                "mention_count": 3,
                "mention_count_root": 2,
                "urgent_mention_count": 1,
                "last_viewed_at": 1739500000000i64
            }),
            broadcast: Some(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: Some(user_id),
                exclude_user_id: None,
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("post_unread should map");
        assert_eq!(mapped.event, "post_unread");
        assert_eq!(mapped.data["msg_count"], serde_json::json!(12));
        assert_eq!(mapped.data["mention_count_root"], serde_json::json!(2));
        assert_eq!(mapped.data["urgent_mention_count"], serde_json::json!(1));
        assert_eq!(
            mapped.data["channel_id"],
            serde_json::json!(encode_mm_id(channel_id))
        );
        assert_eq!(
            mapped.data["post_id"],
            serde_json::json!(encode_mm_id(post_id))
        );
    }

    #[test]
    fn map_envelope_to_mm_maps_thread_updated_payload() {
        let team_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "thread_updated".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::json!({
                "thread": "{\"id\":\"thread-1\"}"
            }),
            broadcast: Some(WsBroadcast {
                channel_id: None,
                team_id: Some(team_id),
                user_id: Some(user_id),
                exclude_user_id: None,
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("thread_updated should map");
        assert_eq!(mapped.event, "thread_updated");
        assert_eq!(
            mapped.data["thread"],
            serde_json::json!("{\"id\":\"thread-1\"}")
        );
        assert_eq!(
            mapped.broadcast.team_id,
            encode_mm_id(team_id),
            "thread_updated team routing must be preserved"
        );
    }

    #[test]
    fn map_envelope_to_mm_maps_received_group_payload() {
        let group_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "received_group".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::json!({
                "group": {
                    "id": encode_mm_id(group_id),
                    "display_name": "Keycloak Group"
                }
            }),
            broadcast: None,
        };

        let mapped = map_envelope_to_mm(&env).expect("received_group should map");
        assert_eq!(mapped.event, "received_group");
        let group = mapped
            .data
            .get("group")
            .and_then(|v| v.as_str())
            .expect("group payload should be encoded json text");
        let decoded: serde_json::Value =
            serde_json::from_str(group).expect("group payload should be valid json");
        assert_eq!(decoded["id"], serde_json::json!(encode_mm_id(group_id)));
    }

    #[test]
    fn map_envelope_to_mm_maps_group_member_event_payload() {
        let group_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "group_member_add".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::json!({
                "group_member": {
                    "group_id": encode_mm_id(group_id),
                    "user_id": encode_mm_id(user_id),
                    "create_at": 1739500000000i64,
                    "delete_at": 0
                }
            }),
            broadcast: Some(WsBroadcast {
                channel_id: None,
                team_id: None,
                user_id: Some(user_id),
                exclude_user_id: None,
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("group_member_add should map");
        assert_eq!(mapped.event, "group_member_add");
        let member = mapped
            .data
            .get("group_member")
            .and_then(|v| v.as_str())
            .expect("group_member payload should be encoded json text");
        let decoded: serde_json::Value =
            serde_json::from_str(member).expect("group_member payload should be valid json");
        assert_eq!(
            decoded["group_id"],
            serde_json::json!(encode_mm_id(group_id))
        );
    }

    #[test]
    fn map_envelope_to_mm_maps_group_syncable_association_payload() {
        let team_id = Uuid::new_v4();
        let group_id = Uuid::new_v4();
        let env = WsEnvelope {
            msg_type: "event".to_string(),
            event: "received_group_associated_to_team".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::json!({
                "group_id": group_id
            }),
            broadcast: Some(WsBroadcast {
                channel_id: None,
                team_id: Some(team_id),
                user_id: None,
                exclude_user_id: None,
            }),
        };

        let mapped = map_envelope_to_mm(&env).expect("association event should map");
        assert_eq!(mapped.event, "received_group_associated_to_team");
        assert_eq!(
            mapped.data["group_id"],
            serde_json::json!(encode_mm_id(group_id))
        );
        assert_eq!(
            mapped.broadcast.team_id,
            encode_mm_id(team_id),
            "association team routing must be preserved"
        );
    }
}
