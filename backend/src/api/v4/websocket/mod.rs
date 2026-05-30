//! Mattermost-compatible WebSocket endpoint with session resumption
//!
//! Implements:
//! - Protocol-level Ping/Pong (WebSocket control frames)
//! - Connection ID & sequence number based session resumption
//! - 60s ping interval, 100s pong timeout, 30s write deadline
//! - Message buffering for replay on reconnect

pub mod auth;
pub mod connection;
pub mod handler;
pub mod resumption;

pub use connection::map_envelope_to_mm;
pub use handler::handle_websocket;

use serde::Deserialize;

/// WebSocket query parameters
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Connection ID for session resumption
    pub connection_id: Option<String>,
    /// Last sequence number received by client
    pub sequence_number: Option<i64>,
}
