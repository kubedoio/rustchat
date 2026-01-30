use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use crate::api::AppState;
use axum::extract::State;

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket_loop(socket))
}

async fn websocket_loop(mut _socket: WebSocket) {
    // Handle authentication handshake
    // Handle ping/pong 
    // Listen for events and push to client
    println!("New MM-compatible WebSocket connection");
}
