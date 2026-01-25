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
use crate::mattermost_compat::models as mm;
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
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(_)) = msg {
                // Handle ping/pong if needed
            } else if matches!(msg, Ok(Message::Close(_))) || msg.is_err() {
                break;
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
        _ => None,
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
