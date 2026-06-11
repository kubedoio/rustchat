//! WebSocket Actor for Mattermost-compatible connections
//!
//! Implements:
//! - Protocol-level Ping/Pong (WebSocket control frames)
//! - 30s ping interval, 2 missed-heartbeat timeout, 30s write deadline
//! - Session resumption support
//! - Graceful shutdown with proper close codes

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, timeout};
use tracing::{debug, error, trace, warn};
use uuid::Uuid;

use crate::mattermost_compat::models as mm;
use crate::realtime::connection_store::{ConnectionState, ConnectionStore};
use crate::telemetry::metrics;

// Heartbeat constants
/// Write timeout for WebSocket operations (30 seconds)
const WRITE_WAIT: Duration = Duration::from_secs(30);
/// Interval between Ping frames (30 seconds)
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// Number of missed heartbeat windows tolerated before closing.
const MAX_MISSED_HEARTBEATS: u32 = 2;
/// Actor command buffer capacity per connection.
const COMMAND_BUFFER_CAPACITY: usize = 256;
/// Actor event buffer capacity per connection.
const EVENT_BUFFER_CAPACITY: usize = 256;

fn is_benign_disconnect_error(error: &str) -> bool {
    let e = error.to_ascii_lowercase();
    e.contains("without closing handshake")
        || e.contains("connection reset by peer")
        || e.contains("broken pipe")
        || e.contains("connection closed")
}

fn emit_event(event_tx: &mpsc::Sender<WsEvent>, connection_id: &str, event: WsEvent) -> bool {
    match event_tx.try_send(event) {
        Ok(()) => true,
        Err(TrySendError::Full(_dropped)) => {
            metrics::record_ws_dropped("actor_event_queue_full", 1);
            warn!(
                connection_id = %connection_id,
                "Dropping websocket event because event queue is full"
            );
            // Best-effort signal to the connection handler that the client
            // should perform a full resync.
            let _ = event_tx.try_send(WsEvent::ResyncRequired {
                reason: "queue_full".to_string(),
            });
            false
        }
        Err(TrySendError::Closed(_)) => false,
    }
}

/// WebSocket close codes
pub mod close_codes {
    /// Normal closure
    pub const NORMAL: u16 = 1000;
    /// Going away (server restart, etc.)
    pub const GOING_AWAY: u16 = 1001;
    /// Protocol error
    pub const PROTOCOL_ERROR: u16 = 1002;
    /// Unsupported data
    pub const UNSUPPORTED_DATA: u16 = 1003;
    /// Policy violation (auth failure)
    pub const POLICY_VIOLATION: u16 = 1008;
    /// Message too big
    pub const MESSAGE_TOO_BIG: u16 = 1009;
    /// Internal server error
    pub const INTERNAL_ERROR: u16 = 1011;
    /// Service restart
    pub const SERVICE_RESTART: u16 = 1012;
    /// Try again later
    pub const TRY_AGAIN_LATER: u16 = 1013;
}

/// Commands sent to the WebSocket actor
#[derive(Debug)]
pub enum WsCommand {
    /// Send a message to the client
    SendMessage(mm::WebSocketMessage),
    /// Send raw JSON to the client
    SendRaw(serde_json::Value),
    /// Close the connection with a specific code
    Close(u16, String),
    /// Update channel subscriptions
    SubscribeChannels(Vec<Uuid>),
    /// Update team subscriptions
    SubscribeTeams(Vec<Uuid>),
}

/// Events received from the WebSocket actor
#[derive(Debug)]
pub enum WsEvent {
    /// Message received from client
    MessageReceived(String),
    /// Binary message received from client
    BinaryReceived(Vec<u8>),
    /// Client sent a pong (activity detected)
    PongReceived,
    /// Connection closed
    Closed(CloseReason),
    /// Error occurred
    Error(String),
    /// Resync required due to queue overflow or lag
    ResyncRequired { reason: String },
}

/// Reason for connection close
#[derive(Debug, Clone)]
pub struct CloseReason {
    pub code: u16,
    pub reason: String,
}

/// WebSocket Actor that manages a single connection
pub struct WebSocketActor {
    /// Connection ID
    pub connection_id: String,
    /// User ID
    pub user_id: Uuid,
    /// Connection state for resumption
    #[allow(dead_code)]
    state: Arc<ConnectionState>,
    /// Connection store for session management
    store: Arc<ConnectionStore>,
    /// Channel for sending commands to the actor
    cmd_tx: mpsc::Sender<WsCommand>,
    /// Channel for receiving events from the actor
    event_rx: Mutex<mpsc::Receiver<WsEvent>>,
    /// Last activity timestamp (for pong timeout)
    #[allow(dead_code)]
    last_activity: Arc<std::sync::atomic::AtomicU64>,
    /// Whether the connection is closing
    is_closing: Arc<AtomicBool>,
    /// Remote address (for logging)
    #[allow(dead_code)]
    remote_addr: Option<SocketAddr>,
}

impl WebSocketActor {
    /// Create a new WebSocket actor and start processing
    ///
    /// # Arguments
    /// * `socket` - The WebSocket stream
    /// * `store` - Connection store for session management
    /// * `user_id` - Authenticated user ID
    /// * `connection_id` - Existing connection ID for resumption, or None for new
    /// * `sequence_number` - Last sequence number received by client
    /// * `remote_addr` - Client remote address for logging
    pub async fn new(
        socket: WebSocket,
        store: Arc<ConnectionStore>,
        user_id: Uuid,
        connection_id: Option<String>,
        sequence_number: Option<i64>,
        remote_addr: Option<SocketAddr>,
    ) -> (Arc<Self>, Vec<mm::WebSocketMessage>) {
        // Resume or create connection state
        let (state, is_resumed, missed_messages) =
            store.resume_or_create(connection_id, user_id, sequence_number);

        let conn_id = state.connection_id.clone();

        // Create channels for actor communication
        let (cmd_tx, cmd_rx) = mpsc::channel(COMMAND_BUFFER_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel(EVENT_BUFFER_CAPACITY);

        let last_activity = Arc::new(std::sync::atomic::AtomicU64::new(
            Instant::now().elapsed().as_secs(),
        ));
        let is_closing = Arc::new(AtomicBool::new(false));

        // Convert missed SequencedMessages to WebSocketMessages
        let missed_ws_messages: Vec<mm::WebSocketMessage> = missed_messages
            .into_iter()
            .map(|msg| replay_message_to_ws_message(msg.seq, &msg.message))
            .collect();

        // Spawn the actor task
        let actor = ActorTask {
            socket,
            state: state.clone(),
            store: store.clone(),
            cmd_rx,
            event_tx,
            last_activity: last_activity.clone(),
            is_closing: is_closing.clone(),
            user_id,
            connection_id: conn_id.clone(),
        };

        tokio::spawn(actor.run());

        let actor = Arc::new(Self {
            connection_id: conn_id,
            user_id,
            state,
            store,
            cmd_tx,
            event_rx: Mutex::new(event_rx),
            last_activity,
            is_closing,
            remote_addr,
        });

        debug!(
            connection_id = %actor.connection_id,
            user_id = %user_id,
            resumed = is_resumed,
            missed_count = missed_ws_messages.len(),
            "WebSocket actor created"
        );

        (actor, missed_ws_messages)
    }

    /// Send a message to the client
    pub fn send(&self, msg: mm::WebSocketMessage) -> Result<(), String> {
        if self.is_closing.load(Ordering::SeqCst) {
            return Err("Connection is closing".to_string());
        }

        self.cmd_tx
            .try_send(WsCommand::SendMessage(msg))
            .map_err(|e| match e {
                TrySendError::Full(_) => "Connection command queue is full".to_string(),
                TrySendError::Closed(_) => "Connection is closed".to_string(),
            })
    }

    /// Send raw JSON to the client
    pub fn send_raw(&self, data: serde_json::Value) -> Result<(), String> {
        if self.is_closing.load(Ordering::SeqCst) {
            return Err("Connection is closing".to_string());
        }

        self.cmd_tx
            .try_send(WsCommand::SendRaw(data))
            .map_err(|e| match e {
                TrySendError::Full(_) => "Connection command queue is full".to_string(),
                TrySendError::Closed(_) => "Connection is closed".to_string(),
            })
    }

    /// Close the connection
    pub fn close(&self, code: u16, reason: &str) {
        self.is_closing.store(true, Ordering::SeqCst);
        let _ = self
            .cmd_tx
            .try_send(WsCommand::Close(code, reason.to_string()));
    }

    /// Receive the next event from the actor
    pub async fn recv(&self) -> Option<WsEvent> {
        let mut rx = self.event_rx.lock().await;
        rx.recv().await
    }

    /// Update last activity (called when any message is received)
    #[allow(dead_code)]
    fn update_activity(&self) {
        self.last_activity
            .store(Instant::now().elapsed().as_secs(), Ordering::SeqCst);
        self.state.touch();
    }

    /// Clone the command sender for external shutdown signalling.
    pub fn cmd_tx(&self) -> mpsc::Sender<WsCommand> {
        self.cmd_tx.clone()
    }

    /// Check if the connection is alive (not timed out)
    pub fn is_alive(&self) -> bool {
        !self.is_closing.load(Ordering::SeqCst)
    }

    /// Mark connection as disconnected (for session resumption)
    pub fn disconnect(&self) {
        self.is_closing.store(true, Ordering::SeqCst);
        self.store.disconnect_connection(&self.connection_id);
    }
}

fn replay_message_to_ws_message(seq: i64, message: &serde_json::Value) -> mm::WebSocketMessage {
    let broadcast = message
        .get("broadcast")
        .cloned()
        .and_then(|value| serde_json::from_value::<mm::Broadcast>(value).ok())
        .unwrap_or_else(default_broadcast);

    mm::WebSocketMessage {
        seq: Some(seq),
        event: message
            .get("event")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown")
            .to_string(),
        data: message.get("data").cloned().unwrap_or(json!({})),
        broadcast,
    }
}

fn default_broadcast() -> mm::Broadcast {
    mm::Broadcast {
        omit_users: None,
        user_id: String::new(),
        channel_id: String::new(),
        team_id: String::new(),
    }
}

/// Internal actor task that manages the WebSocket I/O
struct ActorTask {
    socket: WebSocket,
    state: Arc<ConnectionState>,
    store: Arc<ConnectionStore>,
    cmd_rx: mpsc::Receiver<WsCommand>,
    event_tx: mpsc::Sender<WsEvent>,
    #[allow(dead_code)]
    last_activity: Arc<std::sync::atomic::AtomicU64>,
    is_closing: Arc<AtomicBool>,
    user_id: Uuid,
    connection_id: String,
}

impl ActorTask {
    async fn run(mut self) {
        let event_tx = self.event_tx.clone();
        let event_connection_id = self.connection_id.clone();
        let (mut ws_sink, mut ws_stream) = self.socket.split();

        // Create ping interval
        let mut ping_interval = interval(PING_INTERVAL);
        // Skip the immediate first tick; we want the first ping after PING_INTERVAL.
        ping_interval.tick().await;

        // Track last pong time
        let last_pong = Arc::new(std::sync::Mutex::new(Instant::now()));

        // Main event loop
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(mut guard) = last_pong.lock() {
                                *guard = Instant::now();
                            }
                            self.state.touch();

                            let text_str = text.to_string();
                            trace!(
                                connection_id = %self.connection_id,
                                text = %text_str,
                                "Received text message"
                            );

                            if !emit_event(&event_tx, &event_connection_id, WsEvent::MessageReceived(text_str)) {
                                break;
                            }
                        }
                        Some(Ok(Message::Binary(bin))) => {
                            if let Ok(mut guard) = last_pong.lock() {
                                *guard = Instant::now();
                            }
                            self.state.touch();
                            if !emit_event(&event_tx, &event_connection_id, WsEvent::BinaryReceived(bin.to_vec())) {
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            trace!(connection_id = %self.connection_id, "Received Pong frame");
                            if let Ok(mut guard) = last_pong.lock() {
                                *guard = Instant::now();
                            }
                            self.state.touch();

                            if !emit_event(&event_tx, &event_connection_id, WsEvent::PongReceived) {
                                break;
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            // Respond with Pong immediately
                            trace!(connection_id = %self.connection_id, "Received Ping frame, sending Pong");
                            if let Err(e) = ws_sink.send(Message::Pong(data)).await {
                                warn!(
                                    connection_id = %self.connection_id,
                                    error = %e,
                                    "Failed to send Pong"
                                );
                                break;
                            }
                        }
                        Some(Ok(Message::Close(frame))) => {
                            debug!(
                                connection_id = %self.connection_id,
                                frame = ?frame,
                                "Received Close frame"
                            );

                            let reason = frame.map(|f| CloseReason {
                                code: f.code,
                                reason: f.reason.to_string(),
                            }).unwrap_or_else(|| CloseReason {
                                code: close_codes::NORMAL,
                                reason: "Client closed".to_string(),
                            });

                            let _ = emit_event(&event_tx, &event_connection_id, WsEvent::Closed(reason));
                            break;
                        }
                        Some(Err(e)) => {
                            let error_text = e.to_string();
                            if is_benign_disconnect_error(&error_text) {
                                warn!(
                                    connection_id = %self.connection_id,
                                    error = %error_text,
                                    "WebSocket disconnected without clean close handshake"
                                );
                                let _ = emit_event(&event_tx, &event_connection_id, WsEvent::Closed(CloseReason {
                                    code: close_codes::GOING_AWAY,
                                    reason: "Peer disconnected".to_string(),
                                }));
                            } else {
                                error!(
                                    connection_id = %self.connection_id,
                                    error = %error_text,
                                    "WebSocket error"
                                );
                                let _ = emit_event(&event_tx, &event_connection_id, WsEvent::Error(error_text));
                            }
                            break;
                        }
                        None => {
                            debug!(
                                connection_id = %self.connection_id,
                                "WebSocket stream ended"
                            );
                            break;
                        }
                    }
                }

                // Handle commands from the actor
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(WsCommand::SendMessage(msg)) => {
                            match serde_json::to_string(&msg) {
                                Ok(json) => {
                                    // Apply write timeout
                                    match timeout(
                                        WRITE_WAIT,
                                        ws_sink.send(Message::Text(json.into()))
                                    ).await {
                                        Ok(Ok(())) => {
                                            trace!(
                                                connection_id = %self.connection_id,
                                                seq = ?msg.seq,
                                                event = %msg.event,
                                                "Message sent"
                                            );
                                        }
                                        Ok(Err(e)) => {
                                            warn!(
                                                connection_id = %self.connection_id,
                                                error = %e,
                                                "Failed to send message"
                                            );
                                            break;
                                        }
                                        Err(_) => {
                                            warn!(
                                                connection_id = %self.connection_id,
                                                "Write timeout"
                                            );
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        connection_id = %self.connection_id,
                                        error = %e,
                                        "Failed to serialize message"
                                    );
                                }
                            }
                        }
                        Some(WsCommand::SendRaw(data)) => {
                            match serde_json::to_string(&data) {
                                Ok(json) => {
                                    match timeout(
                                        WRITE_WAIT,
                                        ws_sink.send(Message::Text(json.into()))
                                    ).await {
                                        Ok(Ok(())) => {}
                                        Ok(Err(e)) => {
                                            warn!(
                                                connection_id = %self.connection_id,
                                                error = %e,
                                                "Failed to send raw message"
                                            );
                                            break;
                                        }
                                        Err(_) => {
                                            warn!(
                                                connection_id = %self.connection_id,
                                                "Write timeout on raw message"
                                            );
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        connection_id = %self.connection_id,
                                        error = %e,
                                        "Failed to serialize raw message"
                                    );
                                }
                            }
                        }
                        Some(WsCommand::Close(code, reason)) => {
                            debug!(
                                connection_id = %self.connection_id,
                                code = code,
                                reason = %reason,
                                "Closing connection"
                            );

                            let reason_clone = reason.clone();
                            let close_frame = CloseFrame {
                                code,
                                reason: reason.into(),
                            };

                            let _ = ws_sink.send(Message::Close(Some(close_frame))).await;
                            let _ = emit_event(&event_tx, &event_connection_id, WsEvent::Closed(CloseReason {
                                code,
                                reason: reason_clone,
                            }));
                            break;
                        }
                        Some(WsCommand::SubscribeChannels(channels)) => {
                            // Update state
                            // Note: We'd need to add proper channel tracking to ConnectionState
                            debug!(
                                connection_id = %self.connection_id,
                                channel_count = channels.len(),
                                "Subscribing to channels"
                            );
                        }
                        Some(WsCommand::SubscribeTeams(teams)) => {
                            debug!(
                                connection_id = %self.connection_id,
                                team_count = teams.len(),
                                "Subscribing to teams"
                            );
                        }
                        None => {
                            // Command channel closed
                            break;
                        }
                    }
                }

                // Send periodic pings
                _ = ping_interval.tick() => {
                    if self.is_closing.load(Ordering::SeqCst) {
                        break;
                    }

                    let last_pong_time = last_pong.lock().ok().map(|g| *g).unwrap_or_else(Instant::now);
                    let heartbeat_deadline =
                        PING_INTERVAL.saturating_mul(MAX_MISSED_HEARTBEATS);
                    if Instant::now().duration_since(last_pong_time) > heartbeat_deadline {
                        warn!(
                            connection_id = %self.connection_id,
                            missed_heartbeats = MAX_MISSED_HEARTBEATS,
                            "Pong timeout - closing connection"
                        );
                        break;
                    }

                    trace!(connection_id = %self.connection_id, "Sending periodic Ping");

                    match timeout(
                        WRITE_WAIT,
                        ws_sink.send(Message::Ping(vec![].into()))
                    ).await {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) => {
                            warn!(
                                connection_id = %self.connection_id,
                                error = %e,
                                "Failed to send Ping"
                            );
                            break;
                        }
                        Err(_) => {
                            warn!(
                                connection_id = %self.connection_id,
                                "Ping write timeout"
                            );
                            break;
                        }
                    }
                }
            }
        }

        // Cleanup
        self.is_closing.store(true, Ordering::SeqCst);

        // Send close frame for graceful shutdown (unless client already closed)
        // This prevents "Connection reset without closing handshake" errors
        let close_frame = CloseFrame {
            code: close_codes::GOING_AWAY,
            reason: "Connection ended".into(),
        };
        // Best-effort close frame - don't wait or error if it fails
        let _ = timeout(
            Duration::from_secs(1),
            ws_sink.send(Message::Close(Some(close_frame))),
        )
        .await;

        // Mark connection as disconnected (but retain state for resumption)
        self.store.disconnect_connection(&self.connection_id);

        debug!(
            connection_id = %self.connection_id,
            user_id = %self.user_id,
            "WebSocket actor stopped"
        );
    }
}

/// Configure TCP keepalive for mobile networks
///
/// This function configures socket-level keepalive to prevent
/// mobile carriers from dropping idle connections.
#[cfg(unix)]
pub fn configure_tcp_keepalive(socket: &std::net::TcpStream) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;

    let fd = socket.as_raw_fd();

    // Enable TCP keepalive
    let enabled: libc::c_int = 1;
    set_sockopt_checked(
        fd,
        libc::SOL_SOCKET,
        libc::SO_KEEPALIVE,
        &enabled as *const _ as *const libc::c_void,
        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
    )?;

    // Set keepalive interval to 15 seconds (keep under 30s carrier timeout)
    let interval: libc::c_int = 15;
    set_sockopt_checked(
        fd,
        libc::IPPROTO_TCP,
        libc::TCP_KEEPINTVL,
        &interval as *const _ as *const libc::c_void,
        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
    )?;

    // Set probe count to 3
    let probes: libc::c_int = 3;
    set_sockopt_checked(
        fd,
        libc::IPPROTO_TCP,
        libc::TCP_KEEPCNT,
        &probes as *const _ as *const libc::c_void,
        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
    )?;

    Ok(())
}

#[cfg(unix)]
fn set_sockopt_checked(
    fd: std::os::unix::io::RawFd,
    level: libc::c_int,
    optname: libc::c_int,
    optval: *const libc::c_void,
    optlen: libc::socklen_t,
) -> std::io::Result<()> {
    // SAFETY: `optval` points to a valid option value with `optlen` bytes and
    // `fd` is provided by a live `TcpStream`.
    let rc = unsafe { libc::setsockopt(fd, level, optname, optval, optlen) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
pub fn configure_tcp_keepalive(_socket: &std::net::TcpStream) -> std::io::Result<()> {
    // Not implemented for non-Unix platforms
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_close_codes() {
        assert_eq!(close_codes::NORMAL, 1000);
        assert_eq!(close_codes::GOING_AWAY, 1001);
        assert_eq!(close_codes::POLICY_VIOLATION, 1008);
        assert_eq!(close_codes::INTERNAL_ERROR, 1011);
    }

    #[test]
    fn replay_message_preserves_broadcast() {
        let payload = json!({
            "event": "typing",
            "data": { "user_id": "abc" },
            "broadcast": {
                "omit_users": null,
                "user_id": "u1",
                "channel_id": "c1",
                "team_id": "t1"
            }
        });

        let msg = replay_message_to_ws_message(42, &payload);
        assert_eq!(msg.seq, Some(42));
        assert_eq!(msg.event, "typing");
        assert_eq!(msg.broadcast.user_id, "u1");
        assert_eq!(msg.broadcast.channel_id, "c1");
        assert_eq!(msg.broadcast.team_id, "t1");
    }

    #[test]
    fn replay_message_defaults_broadcast_when_missing() {
        let payload = json!({
            "event": "posted",
            "data": { "post": "{}" }
        });

        let msg = replay_message_to_ws_message(7, &payload);
        assert_eq!(msg.seq, Some(7));
        assert_eq!(msg.broadcast.user_id, "");
        assert_eq!(msg.broadcast.channel_id, "");
        assert_eq!(msg.broadcast.team_id, "");
    }
}
