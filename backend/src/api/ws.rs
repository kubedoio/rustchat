//! WebSocket API endpoint

use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::HeaderMap,
    middleware,
    response::Response,
    routing::get,
    Router,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use tokio::time::Duration;

use super::AppState;
use crate::api::v4::websocket::map_envelope_to_mm;
use crate::api::websocket_core::{self, EnvelopeCommandOptions};
use crate::realtime::WsEnvelope;
use crate::telemetry::metrics;

/// Build WebSocket routes
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/ws", get(ws_handler))
        .layer(middleware::from_fn_with_state(
            state,
            crate::middleware::rate_limit::websocket_ip_rate_limit,
        ))
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let requested_protocol = websocket_core::requested_protocol(&headers);
    let token = websocket_core::resolve_auth_token(&headers, requested_protocol.as_deref());

    tracing::info!(
        "WS Handshake - Token present: {}, Protocol: {:?}",
        token.is_some(),
        requested_protocol
    );

    let auth = match websocket_core::validate_auth_context(token.as_deref(), &state) {
        Some(auth) => auth,
        None => {
            tracing::warn!("WS Handshake failed: Invalid token");
            return Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap();
        }
    };

    // Check user is active and not deleted
    if let Err(err) = crate::auth::middleware::ensure_user_access_active(&state, auth.user_id).await
    {
        tracing::warn!(user_id = %auth.user_id, error = %err, "WS Handshake failed: user inactive or deleted");
        return Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap();
    }

    if websocket_core::enforce_connection_limit(&state, auth.user_id)
        .await
        .is_err()
    {
        return Response::builder()
            .status(429)
            .body("Too many connections".into())
            .unwrap();
    }

    let username = websocket_core::fetch_username(&state, auth.user_id)
        .await
        .unwrap_or_else(|_| "Unknown".to_string());

    let mut response = ws.on_upgrade(move |socket| handle_socket(socket, auth, username, state));

    // Spec compliance: if client requested a protocol, we MUST return it
    if let Some(p) = requested_protocol {
        if let Ok(header_val) = p.parse() {
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", header_val);
        }
    }

    response
}

/// Handle WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    auth: websocket_core::WebSocketAuth,
    username: String,
    state: AppState,
) {
    let user_id = auth.user_id;
    if auth.expires_at <= Utc::now() {
        tracing::warn!(
            user_id = %user_id,
            token_expires_at = %auth.expires_at,
            "Rejecting websocket connection because token is already expired"
        );
        return;
    }

    let (mut sender, mut receiver) = socket.split();

    // Add connection to hub
    let (connection_id, mut rx) = state.ws_hub.add_connection(user_id, username.clone()).await;
    let connection_id_str = connection_id.to_string();
    websocket_core::register_presence_connection(&state, user_id, &connection_id_str).await;

    websocket_core::initialize_connection_state(&state, user_id, true).await;

    let heartbeat_state = state.clone();
    let heartbeat_connection_id = connection_id_str.clone();
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

    // Send hello message with connection_id for reliable WebSocket support
    let connection_uuid = uuid::Uuid::new_v4();
    let hello = WsEnvelope::hello(
        connection_uuid,
        &format!("rustchat-{}", env!("CARGO_PKG_VERSION")),
    );
    if let Ok(msg) = serde_json::to_string(&hello) {
        let _ = sender.send(Message::Text(msg.into())).await;
    }

    // Channel to signal close frame should be sent
    let (should_close_tx, mut should_close_rx) = tokio::sync::watch::channel(false);

    // Spawn task to forward hub messages to client
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg_result = rx.recv() => {
                    match msg_result {
                        Ok(msg) => {
                            // Convert internal WsEnvelope to Mattermost-compatible format
                            let mm_msg = if let Ok(envelope) = serde_json::from_str::<WsEnvelope>(&msg) {
                                map_envelope_to_mm(&envelope)
                                    .and_then(|m| serde_json::to_string(&m).ok())
                                    .unwrap_or(msg)
                            } else {
                                msg
                            };
                            if sender.send(Message::Text(mm_msg.into())).await.is_err() {
                                break;
                            }
                            metrics::record_ws_message("sent", "hub_event");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            metrics::record_ws_dropped("hub_receiver_lagged", skipped);
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = should_close_rx.changed() => {
                    if *should_close_rx.borrow() {
                        // Send close frame for auth expiry
                        let close_frame = CloseFrame {
                            code: axum::extract::ws::close_code::POLICY,
                            reason: "Authentication token expired".into(),
                        };
                        let _ = sender.send(Message::Close(Some(close_frame))).await;
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming messages from client
    let state_for_receive = state.clone();
    let username_for_receive = username.clone();
    let connection_id_for_receive = connection_id_str.clone();
    let mut receive_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    metrics::record_ws_message("received", "client_message");
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(action) = value.get("action").and_then(|v| v.as_str()) {
                            if crate::api::v4::calls_plugin::handle_ws_action(
                                &state_for_receive,
                                user_id,
                                &connection_id_for_receive,
                                action,
                                value.get("data"),
                            )
                            .await
                            {
                                continue;
                            }
                        }
                    }

                    if !websocket_core::handle_client_envelope_message(
                        &state_for_receive,
                        user_id,
                        &username_for_receive,
                        &text,
                        EnvelopeCommandOptions::V1,
                    )
                    .await
                    {
                        tracing::debug!("Failed to parse ClientEnvelope: {}", text);
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    });

    let token_ttl = auth
        .expires_at
        .signed_duration_since(Utc::now())
        .to_std()
        .unwrap_or(Duration::ZERO);
    let auth_expiry_sleep = tokio::time::sleep(token_ttl);
    tokio::pin!(auth_expiry_sleep);

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => {},
        _ = &mut receive_task => {},
        _ = &mut auth_expiry_sleep => {
            tracing::info!(
                user_id = %user_id,
                token_expires_at = %auth.expires_at,
                "Closing websocket because authentication token expired"
            );
            // Signal send_task to send close frame
            let _ = should_close_tx.send(true);
            // Give send_task time to send the close frame
            let _ = tokio::time::timeout(tokio::time::Duration::from_millis(100), &mut send_task).await;
        },
    }

    if !send_task.is_finished() {
        send_task.abort();
    }
    receive_task.abort();
    heartbeat_task.abort();

    // Cleanup
    state.ws_hub.remove_connection(user_id, connection_id).await;
    websocket_core::handle_disconnect(&state, user_id, &connection_id_str).await;
}
