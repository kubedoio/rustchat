//! WebRTC Signaling Messages
//!
//! Handles offer/answer exchange and ICE candidate communication.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

/// Signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    /// Offer from client
    #[serde(rename = "offer")]
    Offer { sdp: String },

    /// Answer from server
    #[serde(rename = "answer")]
    Answer { sdp: String },

    /// ICE candidate
    #[serde(rename = "ice-candidate")]
    IceCandidate {
        candidate: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sdp_mid: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sdp_mline_index: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        username_fragment: Option<String>,
    },

    /// ICE connection state change
    #[serde(rename = "ice-state")]
    IceConnectionState { state: String },

    /// Peer connection state change
    #[serde(rename = "connection-state")]
    ConnectionState { state: String },

    /// Error
    #[serde(rename = "error")]
    Error { message: String },
}

/// Signaling server manages signaling channels for participants
pub struct SignalingServer {
    /// Signaling channels: session_id -> sender
    channels: Arc<RwLock<HashMap<Uuid, mpsc::Sender<SignalingMessage>>>>,
}

impl SignalingServer {
    /// Create a new signaling server
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a signaling channel for a participant
    pub async fn register_channel(&self, session_id: Uuid, tx: mpsc::Sender<SignalingMessage>) {
        self.channels.write().await.insert(session_id, tx);
    }

    /// Unregister a signaling channel
    pub async fn unregister_channel(&self, session_id: Uuid) {
        self.channels.write().await.remove(&session_id);
    }

    /// Send a signaling message to a participant
    pub async fn send_message(
        &self,
        session_id: Uuid,
        message: SignalingMessage,
    ) -> Result<(), String> {
        let channels = self.channels.read().await;

        if let Some(tx) = channels.get(&session_id) {
            tx.try_send(message).map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => {
                    "Signaling channel is full; message dropped".to_string()
                }
                mpsc::error::TrySendError::Closed(_) => "Signaling channel is closed".to_string(),
            })
        } else {
            Err("Participant not found".to_string())
        }
    }

    /// Broadcast a message to all participants except sender
    pub async fn broadcast_message(&self, sender_session_id: Uuid, message: SignalingMessage) {
        let channels = self.channels.read().await;

        for (session_id, tx) in channels.iter() {
            if *session_id == sender_session_id {
                continue;
            }

            if tx.try_send(message.clone()).is_err() {
                continue;
            }
        }
    }

    /// Parse SDP from string
    pub fn parse_sdp(sdp_str: &str) -> Result<RTCSessionDescription, String> {
        // Parse the SDP string into a SessionDescription
        // This is a simplified version - production code should use proper SDP parsing

        let sdp = RTCSessionDescription::offer(sdp_str.to_string())
            .map_err(|e| format!("Failed to parse SDP: {:?}", e))?;

        Ok(sdp)
    }

    /// Serialize SDP to string
    pub fn serialize_sdp(_sdp: &RTCSessionDescription) -> String {
        // Extract the SDP string from the session description
        // The webrtc crate stores this internally
        // For now, return a placeholder - in production you'd access the internal sdp field
        String::new()
    }
}

impl Default for SignalingServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert WebSocket message to SignalingMessage
pub fn parse_websocket_message(data: &str) -> Result<SignalingMessage, String> {
    serde_json::from_str(data).map_err(|e| format!("Failed to parse signaling message: {}", e))
}

/// Convert SignalingMessage to WebSocket message
pub fn serialize_websocket_message(msg: &SignalingMessage) -> Result<String, String> {
    serde_json::to_string(msg).map_err(|e| format!("Failed to serialize signaling message: {}", e))
}
