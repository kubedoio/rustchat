//! WebSocket event types

use crate::mattermost_compat::id::parse_mm_or_uuid;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebSocket event envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    #[serde(rename = "type")]
    pub msg_type: String, // "event", "command", "ack", "error"
    pub event: String, // e.g. "posted"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Uuid>,
    pub data: serde_json::Value,

    // Broadcast targeting info - included in serialization so clients receive routing info
    pub broadcast: Option<WsBroadcast>,
}

/// Broadcast targeting info
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    ThreadUpdated,

    ReactionAdded,
    ReactionRemoved,

    UserTyping,
    UserTypingStop,

    // Resurrected variants
    ChannelCreated,
    ChannelUpdated,
    ChannelDeleted,
    ChannelRestored,
    ChannelViewed,
    ChannelUnread,
    PostUnread,
    MemberAdded,
    MemberRemoved,
    UserUpdated,
    UserPresence,
    EphemeralMessage,
    CallSignal,
    ConfigUpdated,
    UnreadCountsUpdated,

    ActivityCreated,
    ActivityRead,

    ChannelSubscribed,
    ChannelUnsubscribed,
    ReceivedGroup,
    ReceivedGroupAssociatedToTeam,
    ReceivedGroupNotAssociatedToTeam,
    ReceivedGroupAssociatedToChannel,
    ReceivedGroupNotAssociatedToChannel,
    GroupMemberAdd,
    GroupMemberDeleted,

    AgentStreamChunk,
    AgentStreamComplete,
    AgentStreamError,

    Error,
    Hello,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MessageCreated => "posted",
            Self::MessageUpdated => "post_edited",
            Self::MessageDeleted => "post_deleted",
            Self::ThreadReplyCreated => "thread_reply_created",
            Self::ThreadReplyUpdated => "thread_reply_updated",
            Self::ThreadReplyDeleted => "thread_reply_deleted",
            Self::ThreadUpdated => "thread_updated",
            Self::ReactionAdded => "reaction_added",
            Self::ReactionRemoved => "reaction_removed",
            Self::UserTyping => "typing",
            Self::UserTypingStop => "stop_typing",
            Self::ActivityCreated => "activity_created",
            Self::ActivityRead => "activity_read",
            Self::ChannelSubscribed => "channel_subscribed",
            Self::ChannelUnsubscribed => "channel_unsubscribed",
            Self::ReceivedGroup => "received_group",
            Self::ReceivedGroupAssociatedToTeam => "received_group_associated_to_team",
            Self::ReceivedGroupNotAssociatedToTeam => "received_group_not_associated_to_team",
            Self::ReceivedGroupAssociatedToChannel => "received_group_associated_to_channel",
            Self::ReceivedGroupNotAssociatedToChannel => "received_group_not_associated_to_channel",
            Self::GroupMemberAdd => "group_member_add",
            Self::GroupMemberDeleted => "group_member_deleted",
            Self::ChannelCreated => "channel_created",
            Self::ChannelUpdated => "channel_updated",
            Self::ChannelDeleted => "channel_deleted",
            Self::ChannelRestored => "channel_restored",
            Self::ChannelViewed => "channel_viewed",
            Self::ChannelUnread => "channel_unread",
            Self::PostUnread => "post_unread",
            Self::MemberAdded => "user_added",
            Self::MemberRemoved => "user_removed",
            Self::UserUpdated => "user_updated",
            Self::UserPresence => "status_change",
            Self::EphemeralMessage => "ephemeral_message",
            Self::CallSignal => "call_signal",
            Self::ConfigUpdated => "config_updated",
            Self::UnreadCountsUpdated => "unread_counts_updated",
            Self::AgentStreamChunk => "agent_stream_chunk",
            Self::AgentStreamComplete => "agent_stream_complete",
            Self::AgentStreamError => "agent_stream_error",
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
    #[serde(default, deserialize_with = "deserialize_optional_uuid_compat")]
    pub channel_id: Option<Uuid>,
    pub seq: Option<u64>,
    pub client_msg_id: Option<String>,
}

// Helper to deserialize specific command data
#[derive(Debug, Deserialize)]
pub struct TypingCommandData {
    #[serde(default, deserialize_with = "deserialize_optional_uuid_compat")]
    pub thread_root_id: Option<Uuid>,
}

fn deserialize_optional_uuid_compat<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    match raw {
        None => Ok(None),
        Some(id) => parse_mm_or_uuid(&id)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid uuid/mattermost id: {}", id)))
            .map(Some),
    }
}

#[derive(Debug, Deserialize)]
pub struct SubscribeCommandData {
    // maybe empty if channel_id is at top level
}

/// Typing indicator event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn pong(seq: Option<u64>) -> Self {
        Self {
            msg_type: "response".to_string(),
            event: "pong".to_string(),
            seq,
            channel_id: None,
            data: serde_json::json!({ "text": "pong" }), // Mobile app expects data.text === 'pong'
            broadcast: None,
        }
    }

    pub fn hello(connection_id: Uuid, server_version: &str) -> Self {
        Self {
            msg_type: "event".to_string(),
            event: "hello".to_string(),
            seq: None,
            channel_id: None,
            data: serde_json::json!({
                "connection_id": connection_id.to_string(),
                "server_version": server_version,
                "reliable_websockets": true
            }),
            broadcast: None,
        }
    }
}
