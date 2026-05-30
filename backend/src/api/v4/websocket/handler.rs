use std::net::SocketAddr;

use axum::{
    extract::{
        ws::{rejection::WebSocketUpgradeRejection, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use tracing::{trace, warn};

use crate::api::websocket_core;
use crate::api::AppState;

use super::auth::authenticate_via_websocket;
use super::connection::run_connection;
use super::WsQuery;

/// Main WebSocket handler
pub async fn handle_websocket(
    ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> Response {
    let requested_protocol = websocket_core::requested_protocol(&headers);
    let ws = match ws {
        Ok(upgrade) => upgrade,
        Err(err) => {
            warn!(
                error = %err,
                connection_header = ?headers.get("connection").and_then(|v| v.to_str().ok()),
                upgrade_header = ?headers.get("upgrade").and_then(|v| v.to_str().ok()),
                has_sec_websocket_key = headers.contains_key("sec-websocket-key"),
                sec_websocket_version = ?headers.get("sec-websocket-version").and_then(|v| v.to_str().ok()),
                user_agent = ?headers.get("user-agent").and_then(|v| v.to_str().ok()),
                "WebSocket upgrade rejected"
            );
            return err.into_response();
        }
    };

    let token = websocket_core::resolve_auth_token(&headers, requested_protocol.as_deref());
    let sequence_number = query.sequence_number;
    let connection_id = query.connection_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let auth = websocket_core::validate_auth_context(token.as_deref(), &state);

    trace!(
        has_token = token.is_some(),
        has_protocol = requested_protocol.is_some(),
        has_user = auth.is_some(),
        connection_id = ?connection_id,
        sequence_number = ?sequence_number,
        "WebSocket connection request"
    );

    let mut response = ws.on_upgrade(move |socket| {
        handle_socket(socket, state, auth, connection_id, sequence_number, None)
    });

    // Match Mattermost behavior by echoing the requested protocol when present.
    if let Some(protocol) = requested_protocol {
        if let Ok(value) = protocol.parse() {
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", value);
        }
    }

    response
}

/// Handle the WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    auth: Option<websocket_core::WebSocketAuth>,
    connection_id: Option<String>,
    sequence_number: Option<i64>,
    addr: Option<SocketAddr>,
) {
    // Handle authentication if not already done
    let auth = match auth {
        Some(auth) => auth,
        None => {
            // Try to authenticate via WebSocket message
            match authenticate_via_websocket(socket, &state).await {
                Some((auth, sock)) => {
                    // Continue with authenticated socket
                    return run_connection(
                        sock,
                        state,
                        auth.user_id,
                        auth.expires_at,
                        connection_id,
                        sequence_number,
                        addr,
                    )
                    .await;
                }
                None => {
                    warn!(addr = ?addr, "WebSocket authentication failed");
                    return;
                }
            }
        }
    };

    run_connection(
        socket,
        state,
        auth.user_id,
        auth.expires_at,
        connection_id,
        sequence_number,
        addr,
    )
    .await;
}
