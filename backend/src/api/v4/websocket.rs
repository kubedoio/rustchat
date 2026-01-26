use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::validate_token;
use crate::mattermost_compat::{models as mm, MM_VERSION};
use crate::realtime::{TypingEvent, WsEnvelope};

pub fn router() -> Router<AppState> {
    Router::new().route("/websocket", get(ws_handler))
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    #[allow(dead_code)]
    connection_id: Option<String>,
    #[allow(dead_code)]
    sequence_number: Option<i64>,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(_query): Query<WsQuery>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut user_id: Option<Uuid> = None;
    let mut seq = 0;

    // Wait for authentication
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

                            // Send Hello event
                            // This is required by some clients to finish connection setup
                            let hello = json!({
                                "event": "hello",
                                "data": {
                                    "server_version": MM_VERSION,
                                    "connection_id": "", // We don't track connection IDs strictly yet
                                },
                                "broadcast": {
                                    "user_id": claims.claims.sub.to_string(),
                                    "omit_users": null,
                                    "team_id": "",
                                    "channel_id": ""
                                },
                                "seq": seq
                            });
                            seq += 1;
                            let _ = sender.send(Message::Text(hello.to_string().into())).await;

                            break;
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    let user_id = match user_id {
        Some(uid) => uid,
        None => return, // Failed to auth
    };

    // Authenticated. Connect to Hub.
    let username = match sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(name) => name,
        Err(_) => "Unknown".to_string(),
    };

    let rx = state.ws_hub.add_connection(user_id, username.clone()).await;

    // Subscribe to teams
    let teams =
        sqlx::query_scalar::<_, Uuid>("SELECT team_id FROM team_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    for team_id in teams {
        state.ws_hub.subscribe_team(user_id, team_id).await;
    }

    // Subscribe to channels
    let channels =
        sqlx::query_scalar::<_, Uuid>("SELECT channel_id FROM channel_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    for channel_id in channels {
        state.ws_hub.subscribe_channel(user_id, channel_id).await;
    }

    // Main loop: forward events from rx to sender (mapped to MM format)

    let mut hub_rx = rx;
    let sender_task = tokio::spawn(async move {
        while let Ok(msg_str) = hub_rx.recv().await {
            // msg_str is WsEnvelope JSON string from RustChat hub
            if let Ok(envelope) = serde_json::from_str::<WsEnvelope>(&msg_str) {
                if let Some(mm_msg) = map_envelope_to_mm(&envelope, seq) {
                    seq += 1;
                    if let Ok(json) = serde_json::to_string(&mm_msg) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Handle incoming pings/typing
    let state_clone = state.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    handle_upstream_message(&state_clone, user_id, &text).await;
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = sender_task => {},
        _ = receive_task => {},
    }

    state.ws_hub.remove_connection(user_id).await;
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
                Some(mm::WebSocketMessage {
                    seq: Some(seq),
                    event: "typing".to_string(),
                    data: json!({
                        "parent_id": typing.thread_root_id.unwrap_or_default().to_string(),
                        "user_id": typing.user_id.to_string(),
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
                    user_id: reaction.user_id.to_string(),
                    post_id: reaction.post_id.to_string(),
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
                    user_id: reaction.user_id.to_string(),
                    post_id: reaction.post_id.to_string(),
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
            // Check if this is a status update
             if let Some(status_str) = env.data.get("status").and_then(|v| v.as_str()) {
                 let user_id = env.data.get("user_id").and_then(|v| v.as_str()).unwrap_or_default();
                 Some(mm::WebSocketMessage {
                    seq: Some(seq),
                    event: "status_change".to_string(),
                    data: json!({ "user_id": user_id, "status": status_str }),
                    broadcast: map_broadcast(env.broadcast.as_ref()),
                })
             } else {
                 None
             }
        }
        "channel_updated" => {
             // Check if it is a view event (hacky heuristic based on data shape)
             // or just map generic channel update
             env.data.get("channel_id").map(|cid| mm::WebSocketMessage {
                seq: Some(seq),
                event: "channel_viewed".to_string(),
                data: json!({ "channel_id": cid }),
                broadcast: map_broadcast(env.broadcast.as_ref()),
            })
        }
        _ => None,
    }
}

// Handle client upstream messages (commands)
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
                         if let Ok(channel_id) = Uuid::parse_str(channel_id_str) {
                              // Broadcast typing
                              let broadcast = WsEnvelope::event(
                                    crate::realtime::EventType::UserTyping,
                                    crate::realtime::TypingEvent {
                                        user_id,
                                        display_name: "".to_string(),
                                        thread_root_id: data.get("parent_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()),
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
            user_id: b.user_id.map(|u| u.to_string()).unwrap_or_default(),
            channel_id: b.channel_id.map(|c| c.to_string()).unwrap_or_default(),
            team_id: b.team_id.map(|t| t.to_string()).unwrap_or_default(),
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
