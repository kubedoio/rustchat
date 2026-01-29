//! WebSocket API endpoint

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
// use std::sync::Arc;

use super::AppState;
use crate::auth::validate_token;
use crate::realtime::{
    ClientEnvelope, EventType, PresenceEvent, TypingCommandData, TypingEvent, WsBroadcast,
    WsEnvelope,
};

/// Build WebSocket routes
pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    headers: HeaderMap,
) -> Response {
    let mut token = query.token.clone().unwrap_or_default();
    
    // Extract protocol to echo back (required by spec if sent by client)
    let requested_protocol = headers
        .get("Sec-WebSocket-Protocol")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string());

    // Fallback: if token missing in query, try protocol header
    if token.is_empty() || token == "undefined" {
        if let Some(ref p) = requested_protocol {
            if p.len() > 20 {
                token = p.clone();
            }
        }
    }

    // Remove "Bearer " prefix if present
    if token.starts_with("Bearer ") {
        token = token.trim_start_matches("Bearer ").to_string();
    }

    tracing::info!("WS Handshake - Token present: {}, Protocol: {:?}", !token.is_empty(), requested_protocol);

    // Validate token
    let claims = match validate_token(&token, &state.jwt_secret) {
        Ok(data) => data.claims,
        Err(_) => {
            tracing::warn!("WS Handshake failed: Invalid token");
            return Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap();
        }
    };

    let user_id = claims.sub;
    let username = match sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(name) => name,
        Err(_) => "Unknown".to_string(),
    };

    let mut response = ws.on_upgrade(move |socket| handle_socket(socket, user_id, username, state));

    // Spec compliance: if client requested a protocol, we MUST return it
    if let Some(p) = requested_protocol {
        if let Ok(header_val) = p.parse() {
            response.headers_mut().insert("Sec-WebSocket-Protocol", header_val);
        }
    }

    response
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, user_id: uuid::Uuid, username: String, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Add connection to hub with multi-login support
    let (conn_id, mut rx) = match state.ws_hub.add_connection(user_id, username.clone()).await {
        Some((id, receiver)) => (id, receiver),
        None => {
            tracing::warn!("Max connections reached for user {}, rejecting WebSocket", user_id);
            return;
        }
    };

    // Fetch user's teams and subscribe
    let teams =
        sqlx::query_scalar::<_, uuid::Uuid>("SELECT team_id FROM team_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    for team_id in teams {
        state.ws_hub.subscribe_team(user_id, team_id).await;
    }

    // Persist presence and broadcast
    let _ = sqlx::query("UPDATE users SET presence = 'online' WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await;

    let presence_evt = WsEnvelope::event(
        EventType::UserPresence,
        PresenceEvent {
            user_id,
            status: "online".to_string(),
        },
        None,
    );
    state.ws_hub.broadcast(presence_evt).await;

    // Send hello message
    let hello = WsEnvelope::hello(user_id);
    if let Ok(msg) = serde_json::to_string(&hello) {
        let _ = sender.send(Message::Text(msg.into())).await;
    }

    // Spawn task to forward hub messages to client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client
    let hub_for_receive = state.ws_hub.clone();
    let state_for_receive = state.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    // Try parsing as ClientEnvelope
                    if let Ok(envelope) = serde_json::from_str::<ClientEnvelope>(&text) {
                        let channel_id = envelope.channel_id;

                        match envelope.event.as_str() {
                            "send_message" => {
                                if let Some(cid) = channel_id {
                                    if let Ok(input) =
                                        serde_json::from_value::<crate::models::CreatePost>(
                                            envelope.data.clone(),
                                        )
                                    {
                                        // Process message sending
                                        match crate::services::posts::create_post(
                                            &state_for_receive,
                                            user_id,
                                            cid,
                                            input,
                                            envelope.client_msg_id,
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                // Message created and broadcasted by service.
                                                // We can optionally send an Ack here if needed, but not strictly required as per plan.
                                            }
                                            Err(e) => {
                                                // Send error back
                                                let err_msg = e.to_string();
                                                let err = WsEnvelope::error(&err_msg);
                                                // Direct response to user (broadcast with target user_id)
                                                hub_for_receive
                                                    .broadcast(err.with_broadcast(WsBroadcast {
                                                        user_id: Some(user_id),
                                                        channel_id: None,
                                                        team_id: None,
                                                        exclude_user_id: None,
                                                    }))
                                                    .await;
                                            }
                                        }
                                    }
                                }
                            }
                            "subscribe_channel" => {
                                if let Some(cid) = channel_id {
                                    hub_for_receive.subscribe_channel(user_id, cid).await;
                                    // Ack? Or just emit event? Spec says server->client event "channel_subscribed"
                                    let evt = WsEnvelope::event(
                                        EventType::ChannelSubscribed,
                                        serde_json::json!({ "channel_id": cid }),
                                        None, // Direct response
                                    );
                                    // To send direct response, we'd need access to 'sender' but that's in another task...
                                    // Actually WsHub.broadcast with user_id target can do it.
                                    hub_for_receive
                                        .broadcast(evt.with_broadcast(WsBroadcast {
                                            user_id: Some(user_id),
                                            channel_id: None,
                                            team_id: None,
                                            exclude_user_id: None,
                                        }))
                                        .await;
                                }
                            }
                            "unsubscribe_channel" => {
                                if let Some(cid) = channel_id {
                                    hub_for_receive.unsubscribe_channel(user_id, cid).await;
                                    let evt = WsEnvelope::event(
                                        EventType::ChannelUnsubscribed,
                                        serde_json::json!({ "channel_id": cid }),
                                        None,
                                    );
                                    hub_for_receive
                                        .broadcast(evt.with_broadcast(WsBroadcast {
                                            user_id: Some(user_id),
                                            channel_id: None,
                                            team_id: None,
                                            exclude_user_id: None,
                                        }))
                                        .await;
                                }
                            }
                            "typing" | "typing_start" => {
                                // prompt requests "typing_start" but let's handle "typing" for backward compat if needed
                                if let Some(cid) = channel_id {
                                    let thread_root_id = if let Ok(data) =
                                        serde_json::from_value::<TypingCommandData>(
                                            envelope.data.clone(),
                                        ) {
                                        data.thread_root_id
                                    } else {
                                        None
                                    };

                                    let event = WsEnvelope::event(
                                        EventType::UserTyping,
                                        TypingEvent {
                                            user_id,
                                            display_name: username.clone(),
                                            thread_root_id,
                                        },
                                        Some(cid),
                                    );

                                    hub_for_receive
                                        .broadcast(event.with_broadcast(WsBroadcast {
                                            channel_id: Some(cid),
                                            user_id: None,
                                            team_id: None,
                                            exclude_user_id: Some(user_id), // Don't echo valid typing to self
                                        }))
                                        .await;
                                }
                            }
                            "typing_stop" => {
                                if let Some(cid) = channel_id {
                                    let thread_root_id = if let Ok(data) =
                                        serde_json::from_value::<TypingCommandData>(
                                            envelope.data.clone(),
                                        ) {
                                        data.thread_root_id
                                    } else {
                                        None
                                    };

                                    let event = WsEnvelope::event(
                                        EventType::UserTypingStop,
                                        TypingEvent {
                                            user_id,
                                            display_name: username.clone(),
                                            thread_root_id,
                                        },
                                        Some(cid),
                                    );

                                    hub_for_receive
                                        .broadcast(event.with_broadcast(WsBroadcast {
                                            channel_id: Some(cid),
                                            user_id: None,
                                            team_id: None,
                                            exclude_user_id: Some(user_id),
                                        }))
                                        .await;
                                }
                            }
                            "presence" => {
                                if let Some(status) =
                                    envelope.data.get("status").and_then(|v| v.as_str())
                                {
                                    hub_for_receive
                                        .set_presence(user_id, status.to_string())
                                        .await;
                                    let event = WsEnvelope::event(
                                        EventType::UserPresence,
                                        PresenceEvent {
                                            user_id,
                                            status: status.to_string(),
                                        },
                                        None,
                                    );
                                    // Presence is global? or per team? Usually broadcast to known connections.
                                    // For now, broadcast global or we'd need team context.
                                    hub_for_receive.broadcast(event).await;
                                }
                            }
                            "ping" => {
                                // Should reply with Pong
                                // Since we can't easily access 'sender', assume axum handles low level ping frames,
                                // but for application level ping/pong:
                                let pong = WsEnvelope::pong();
                                hub_for_receive
                                    .broadcast(pong.with_broadcast(WsBroadcast {
                                        user_id: Some(user_id),
                                        channel_id: None,
                                        team_id: None,
                                        exclude_user_id: None,
                                    }))
                                    .await;
                            }
                            _ => {
                                // Unknown command
                                let err = WsEnvelope::error("Unknown command");
                                hub_for_receive
                                    .broadcast(err.with_broadcast(WsBroadcast {
                                        user_id: Some(user_id),
                                        channel_id: None,
                                        team_id: None,
                                        exclude_user_id: None,
                                    }))
                                    .await;
                            }
                        }
                    } else {
                        println!("DEBUG: Failed to parse ClientEnvelope: {}", text);
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
    }

    // Cleanup
    state.ws_hub.remove_connection(user_id, conn_id).await;

    // Persist presence and broadcast
    let _ = sqlx::query("UPDATE users SET presence = 'offline' WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await;

    let presence_evt = WsEnvelope::event(
        EventType::UserPresence,
        PresenceEvent {
            user_id,
            status: "offline".to_string(),
        },
        None,
    );
    state.ws_hub.broadcast(presence_evt).await;
}
