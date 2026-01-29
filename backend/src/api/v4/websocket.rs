use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::HeaderMap,
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use std::time::Duration;
use tokio::time::interval;
use chrono;

use crate::api::AppState;
use crate::auth::validate_token;
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::realtime::{TypingEvent, WsEnvelope};

pub fn router() -> Router<AppState> {
    Router::new().route("/websocket", get(ws_handler))
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: Option<String>,
    #[allow(dead_code)]
    connection_id: Option<String>,
    #[allow(dead_code)]
    sequence_number: Option<i64>,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> Response {
    let mut token = query.token.clone();
    let seq_start = query.sequence_number.unwrap_or(0);

    // Check Authorization header if token not in query
    if token.is_none() {
        if let Some(auth_header) = headers.get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    token = Some(auth_str[7..].to_string());
                } else {
                    token = Some(auth_str.to_string());
                }
            }
        }
    }

    let user_id = if let Some(ref t) = token {
        validate_token(t, &state.jwt_secret).ok().map(|c| c.claims.sub)
    } else {
        None
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, seq_start))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    mut user_id: Option<Uuid>,
    seq_start: i64,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut seq: i64 = seq_start;
    let connection_id = encode_mm_id(Uuid::new_v4());

    // 1. Wait for authentication if not already authenticated via handshake
    if user_id.is_none() {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    if value["action"] == "authentication_challenge" {
                        if let Some(token) = value["data"]["token"].as_str() {
                            if let Ok(claims) = validate_token(token, &state.jwt_secret) {
                                user_id = Some(claims.claims.sub);

                                // Send OK response
                                let resp = json!({
                                    "status": "OK",
                                    "seq_reply": value["seq"]
                                });
                                let _ = sender.send(Message::Text(resp.to_string().into())).await;
                                break;
                            }
                        }
                    }
                }
            } else if let Ok(Message::Close(_)) | Err(_) = msg {
                return;
            }
        }
    }

    let user_id = match user_id {
        Some(uid) => uid,
        None => return, // Failed to auth
    };

    // 2. Send Hello event immediately after successful auth
    let hello = mm::WebSocketMessage {
        seq: Some(seq),
        event: "hello".to_string(),
        data: json!({
            "server_version": "9.5.0",
            "connection_id": connection_id
        }),
        broadcast: mm::Broadcast {
            omit_users: None,
            user_id: "".to_string(),
            channel_id: "".to_string(),
            team_id: "".to_string(),
        },
    };
    seq += 1;
    let _ = sender
        .send(Message::Text(serde_json::to_string(&hello).unwrap_or_default().into()))
        .await;

    // 3. Authenticated. Setup Hub connection.
    let username = match sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(name) => name,
        Err(_) => "Unknown".to_string(),
    };

    // Add connection with multi-login support
    let (conn_id, rx) = match state.ws_hub.add_connection(user_id, username.clone()).await {
        Some((id, receiver)) => (id, receiver),
        None => {
            tracing::warn!("Max connections reached for user {}, rejecting WebSocket", user_id);
            let _ = sender.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                code: axum::extract::ws::close_code::POLICY,
                reason: "Max connections reached".into(),
            }))).await;
            return;
        }
    };

    // Subscribe to teams and channels
    let teams = sqlx::query_scalar::<_, Uuid>("SELECT team_id FROM team_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
    for team_id in teams {
        state.ws_hub.subscribe_team(user_id, team_id).await;
    }

    let channels = sqlx::query_scalar::<_, Uuid>("SELECT channel_id FROM channel_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
    for channel_id in channels {
        state.ws_hub.subscribe_channel(user_id, channel_id).await;
    }

    // 4. Main loops
    let mut hub_rx = rx;
    let (mut sender_sink, mut receiver_stream) = (sender, receiver);

    let state_clone = state.clone();
    
    // Task for forwarding events from hub to client + Heartbeat
    let sender_task = tokio::spawn(async move {
        let mut heartbeat = interval(Duration::from_secs(15));  // Send heartbeat every 15 seconds
        let mut seq = seq;
        let mut last_ping = std::time::Instant::now();
        loop {
            tokio::select! {
                // Heartbeat
                _ = heartbeat.tick() => {
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(last_ping).as_secs();
                    tracing::debug!("[WS] Sending heartbeat ping to user {} (last ping {}s ago)", user_id, elapsed);
                    
                    // Send a ping event or frame. MM uses a "ping" event or WS Ping.
                    // We'll send a JSON ping event for visibility.
                    let ping = json!({
                        "type": "event",
                        "event": "ping",
                        "data": {
                            "server_time": chrono::Utc::now().timestamp_millis(),
                            "interval": 15000
                        },
                        "seq": seq
                    });
                    seq += 1;
                    last_ping = now;
                    if sender_sink.send(Message::Text(ping.to_string().into())).await.is_err() {
                        tracing::warn!("[WS] Failed to send heartbeat to user {}, connection may be dead", user_id);
                        break;
                    }
                }
                // Hub events
                msg_res = hub_rx.recv() => {
                    if let Ok(msg_str) = msg_res {
                        tracing::debug!("[WS] Received hub event: {}", msg_str);
                        if let Ok(envelope) = serde_json::from_str::<WsEnvelope>(&msg_str) {
                            tracing::debug!("[WS] Parsed envelope: event={}", envelope.event);
                            if let Some(mm_msg) = map_envelope_to_mm(&envelope, seq) {
                                tracing::info!("[WS] Sending to client {}: event={} seq={}", user_id, mm_msg.event, seq);
                                if let Ok(json) = serde_json::to_string(&mm_msg) {
                                    seq += 1;
                                    if sender_sink.send(Message::Text(json.into())).await.is_err() {
                                        tracing::warn!("[WS] Failed to send to client {}", user_id);
                                        break;
                                    }
                                }
                            } else {
                                tracing::warn!("[WS] Failed to map envelope to MM format: event={}", envelope.event);
                            }
                        } else {
                            tracing::error!("[WS] Failed to parse envelope: {}", msg_str);
                        }
                    } else {
                        tracing::info!("[WS] Hub receiver closed for user {}", user_id);
                        break;
                    }
                }
            }
        }
    });

    // Task for handling incoming messages (typing, etc.)
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    handle_upstream_message(&state_clone, user_id, &text).await;
                }
                Ok(Message::Ping(data)) => {
                    // Respond with Pong to keep connection alive
                    tracing::debug!("[WS] Received Ping from {}, sending Pong", user_id);
                    // Note: We can't easily send Pong from here since sender is in another task
                    // But the tokio-tungstenite library should auto-respond to Ping
                }
                Ok(Message::Pong(_)) => {
                    tracing::debug!("[WS] Received Pong from {}", user_id);
                }
                Ok(Message::Close(frame)) => {
                    tracing::info!("[WS] Client {} closed connection: {:?}", user_id, frame);
                    break;
                }
                Err(e) => {
                    tracing::error!("[WS] Error from client {}: {}", user_id, e);
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = sender_task => {},
        _ = receive_task => {},
    }

    state.ws_hub.remove_connection(user_id, conn_id).await;
}

fn map_envelope_to_mm(env: &WsEnvelope, seq: i64) -> Option<mm::WebSocketMessage> {
    match env.event.as_str() {
        "message_created" | "thread_reply_created" => {
            if let Ok(post_resp) =
                serde_json::from_value::<crate::models::post::PostResponse>(env.data.clone())
            {
                let mm_post: mm::Post = post_resp.into();
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
                    seq: Some(seq),
                    event: "posted".to_string(),
                    data,
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "user_typing" => {
            if let Ok(typing) = serde_json::from_value::<TypingEvent>(env.data.clone()) {
                let parent_id = typing
                    .thread_root_id
                    .map(encode_mm_id)
                    .unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq: Some(seq),
                    event: "typing".to_string(),
                    data: json!({
                        "parent_id": parent_id,
                        "user_id": encode_mm_id(typing.user_id),
                    }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "message_updated" | "thread_reply_updated" => {
            if let Ok(post_resp) =
                serde_json::from_value::<crate::models::post::PostResponse>(env.data.clone())
            {
                let mm_post: mm::Post = post_resp.into();
                let post_json = serde_json::to_string(&mm_post).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq: Some(seq),
                    event: "post_edited".to_string(),
                    data: json!({ "post": post_json }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "message_deleted" | "thread_reply_deleted" => {
            if let Ok(post_resp) =
                serde_json::from_value::<crate::models::post::PostResponse>(env.data.clone())
            {
                let mm_post: mm::Post = post_resp.into();
                let post_json = serde_json::to_string(&mm_post).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq: Some(seq),
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
                serde_json::from_value::<crate::models::post::Reaction>(env.data.clone())
            {
                let mm_reaction = mm::Reaction {
                    user_id: encode_mm_id(reaction.user_id),
                    post_id: encode_mm_id(reaction.post_id),
                    emoji_name: reaction.emoji_name,
                    create_at: reaction.created_at.timestamp_millis(),
                };
                let reaction_json = serde_json::to_string(&mm_reaction).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq: Some(seq),
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
                serde_json::from_value::<crate::models::post::Reaction>(env.data.clone())
            {
                let mm_reaction = mm::Reaction {
                    user_id: encode_mm_id(reaction.user_id),
                    post_id: encode_mm_id(reaction.post_id),
                    emoji_name: reaction.emoji_name,
                    create_at: reaction.created_at.timestamp_millis(),
                };
                let reaction_json = serde_json::to_string(&mm_reaction).unwrap_or_default();
                Some(mm::WebSocketMessage {
                    seq: Some(seq),
                    event: "reaction_removed".to_string(),
                    data: json!({ "reaction": reaction_json }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
            } else {
                None
            }
        }
        "user_updated" => {
             if let Some(status_str) = env.data.get("status").and_then(|v| v.as_str()) {
                 let user_id = env
                     .data
                     .get("user_id")
                     .and_then(|v| v.as_str())
                     .and_then(parse_mm_or_uuid)
                     .map(encode_mm_id)
                     .unwrap_or_default();
                 let manual = env
                     .data
                     .get("manual")
                     .and_then(|v| v.as_bool())
                     .unwrap_or(true);
                 let last_activity_at = env
                     .data
                     .get("last_activity_at")
                     .and_then(|v| v.as_i64())
                     .unwrap_or(0);
                  Some(mm::WebSocketMessage {
                     seq: Some(seq),
                     event: "status_change".to_string(),
                     data: json!({
                         "user_id": user_id,
                         "status": status_str,
                         "manual": manual,
                         "last_activity_at": last_activity_at
                     }),
                     broadcast: map_broadcast(env.broadcast.as_ref()),
                 })
             } else {
                 None
             }
        }
        "channel_updated" => {
             env.data.get("channel_id").and_then(|cid| cid.as_str()).map(|cid| {
                let channel_id = parse_mm_or_uuid(cid).map(encode_mm_id).unwrap_or_default();
                mm::WebSocketMessage {
                    seq: Some(seq),
                    event: "channel_viewed".to_string(),
                    data: json!({ "channel_id": channel_id }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                }
             })
        }
        _ => None,
    }
}

async fn handle_upstream_message(
    state: &AppState,
    user_id: Uuid,
    msg: &str
) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(msg) {
        if let Some(action) = value.get("action").and_then(|v| v.as_str()) {
             if action == "user_typing" {
                 if let Some(data) = value.get("data") {
                     if let Some(channel_id_str) = data.get("channel_id").and_then(|v| v.as_str()) {
                         if let Some(channel_id) = parse_mm_or_uuid(channel_id_str) {
                              let broadcast = WsEnvelope::event(
                                    crate::realtime::EventType::UserTyping,
                                    crate::realtime::TypingEvent {
                                        user_id,
                                        display_name: "".to_string(),
                                        thread_root_id: data
                                            .get("parent_id")
                                            .and_then(|v| v.as_str())
                                            .and_then(parse_mm_or_uuid),
                                    },
                                    Some(channel_id),
                                ).with_broadcast(crate::realtime::WsBroadcast {
                                    channel_id: Some(channel_id),
                                    team_id: None,
                                    user_id: None,
                                    exclude_user_id: Some(user_id),
                                });
                                state.ws_hub.broadcast(broadcast).await;
                         }
                     }
                 }
             }
        }
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
            user_id: "".to_string(),
            channel_id: "".to_string(),
            team_id: "".to_string(),
        }
    }
}
