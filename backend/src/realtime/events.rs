//! WebSocket event types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebSocket event envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    #[serde(rename = "type")]
    pub msg_type: String,     // "event", "command", "ack", "error"
    pub event: String,        // e.g. "message_created"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Uuid>,
    pub data: serde_json::Value,
    
    // Internal use for broadcast targeting (not serialized to client if possible, or filtered out before send)
    // Actually, we can use a wrapper or just strip this field. 
    // For simplicity, let's keep it but skip serializing it.
    #[serde(skip)]
    pub broadcast: Option<WsBroadcast>,
}

/// Broadcast targeting info
#[derive(Debug, Clone)]
pub struct WsBroadcast {
    pub channel_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub exclude_user_id: Option<Uuid>, // New: to exclude sender (optional)
}

/// Event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    MessageCreated,
    MessageUpdated,
    MessageDeleted,
    ThreadReplyCreated,
    // ThreadReplyUpdated/Deleted can reuse MessageUpdated/Deleted if logic allows, 
    // or be explicit. Let's strictly follow spec if given, or use standard patterns.
    // Prompt mentions: thread_reply_created, thread_reply_updated, thread_reply_deleted
    ThreadReplyUpdated,
    ThreadReplyDeleted,
    
    ReactionAdded,
    ReactionRemoved,
    
    UserTyping,
    UserTypingStop,

    // Resurrected variants
    ChannelCreated,
    ChannelUpdated,
    ChannelDeleted,
    MemberAdded,
    MemberRemoved,
    UserUpdated,
    UserPresence,
    CallSignal,
    ConfigUpdated,
    
    ChannelSubscribed,
    ChannelUnsubscribed,
    
    Error,
    Hello,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MessageCreated => "message_created",
            Self::MessageUpdated => "message_updated",
            Self::MessageDeleted => "message_deleted",
            Self::ThreadReplyCreated => "thread_reply_created",
            Self::ThreadReplyUpdated => "thread_reply_updated",
            Self::ThreadReplyDeleted => "thread_reply_deleted",
            Self::ReactionAdded => "reaction_added",
            Self::ReactionRemoved => "reaction_removed",
            Self::UserTyping => "user_typing",
            Self::UserTypingStop => "user_typing_stop",
            Self::ChannelSubscribed => "channel_subscribed",
            Self::ChannelUnsubscribed => "channel_unsubscribed",
            Self::ChannelCreated => "channel_created",
            Self::ChannelUpdated => "channel_updated",
            Self::ChannelDeleted => "channel_deleted",
            Self::MemberAdded => "member_added",
            Self::MemberRemoved => "member_removed",
            Self::UserUpdated => "user_updated",
            Self::UserPresence => "user_presence",
            Self::CallSignal => "call_signal",
            Self::ConfigUpdated => "config_updated",
            Self::Error => "error",
            Self::Hello => "hello",
        }
    }
}

/// Client message (from client to server)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")] // Discrimate by "type": "command"
pub struct ClientEnvelope {
    pub event: String,
    pub data: serde_json::Value,
    pub channel_id: Option<Uuid>,
    pub seq: Option<u64>,
    pub client_msg_id: Option<String>,
}

// Helper to deserialize specific command data
#[derive(Debug, Deserialize)]
pub struct TypingCommandData {
    pub thread_root_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SubscribeCommandData {
    // maybe empty if channel_id is at top level
}

/// Typing indicator event payload
#[derive(Debug, Clone, Serialize)]
pub struct TypingEvent {
    pub user_id: Uuid,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_root_id: Option<Uuid>,
}

/// Presence event
#[derive(Debug, Clone, Serialize)]
pub struct PresenceEvent {
    pub user_id: Uuid,
    pub status: String, // online, away, offline
}

/// Call signaling event (SDP, ICE candidate)
#[derive(Debug, Clone, Serialize)]
pub struct CallSignalEvent {
    pub sender_id: Uuid,
    pub signal: serde_json::Value,
}

impl WsEnvelope {
    pub fn event<T: Serialize>(event: EventType, data: T, channel_id: Option<Uuid>) -> Self {
        Self {
            msg_type: "event".to_string(),
            event: event.as_str().to_string(),
            seq: None,
            channel_id,
            data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
            broadcast: None,
        }
    }

    pub fn with_broadcast(mut self, broadcast: WsBroadcast) -> Self {
        self.broadcast = Some(broadcast);
        self
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            msg_type: "event".to_string(),
            event: "error".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::json!({ "message": message }),
            broadcast: None,
        }
    }

    pub fn hello(user_id: Uuid) -> Self {
        Self {
            msg_type: "event".to_string(),
            event: "hello".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::json!({ "user_id": user_id }),
            broadcast: None,
        }
    }

    pub fn pong() -> Self {
        Self {
            msg_type: "ack".to_string(), // Use "ack" or "response"
            event: "pong".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::Value::Null,
            broadcast: None,
        }
    }
}
