//! Channel model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "channel_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    #[default]
    Public,
    Private,
    Direct,
    Group,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            ChannelType::Public => "public",
            ChannelType::Private => "private",
            ChannelType::Direct => "direct",
            ChannelType::Group => "group",
        };
        f.write_str(value)
    }
}

/// Channel entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Channel {
    pub id: Uuid,
    pub team_id: Uuid,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
    pub display_name: Option<String>,
    pub purpose: Option<String>,
    pub header: Option<String>,
    pub is_archived: bool,
    pub creator_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Channel member
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ChannelMember {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub notify_props: serde_json::Value,
    pub last_viewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[sqlx(default)]
    pub msg_count: i64,
    #[sqlx(default)]
    pub mention_count: i64,
    #[sqlx(default)]
    pub mention_count_root: i64,
    #[sqlx(default)]
    pub urgent_mention_count: i64,
    #[sqlx(default)]
    pub msg_count_root: i64,
    #[sqlx(default)]
    pub manually_unread: bool,
    #[sqlx(default)]
    pub last_update_at: Option<DateTime<Utc>>,

    // Joined fields (optional)
    #[sqlx(default)]
    pub username: Option<String>,
    #[sqlx(default)]
    pub display_name: Option<String>,
    #[sqlx(default)]
    pub avatar_url: Option<String>,
    #[sqlx(default)]
    pub presence: Option<String>,
}

/// DTO for creating a channel
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChannel {
    pub team_id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub purpose: Option<String>,
    #[serde(default)]
    pub channel_type: ChannelType,
    pub target_user_id: Option<Uuid>,
}

/// DTO for updating a channel
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub purpose: Option<String>,
    pub header: Option<String>,
}

fn sort_user_ids(user_a: Uuid, user_b: Uuid) -> (Uuid, Uuid) {
    if user_a <= user_b {
        (user_a, user_b)
    } else {
        (user_b, user_a)
    }
}

/// Canonical Mattermost DM channel name format: "<user1>__<user2>"
pub fn canonical_direct_channel_name(user_a: Uuid, user_b: Uuid) -> String {
    let (first, second) = sort_user_ids(user_a, user_b);
    format!("{first}__{second}")
}

/// Legacy RustChat DM channel name format kept for backward compatibility.
pub fn legacy_direct_channel_name(user_a: Uuid, user_b: Uuid) -> String {
    let (first, second) = sort_user_ids(user_a, user_b);
    format!("dm_{first}_{second}")
}

/// Parses both canonical ("<id>__<id>") and legacy ("dm_<id>_<id>") DM names.
pub fn parse_direct_channel_name(name: &str) -> Option<(Uuid, Uuid)> {
    if let Some((left, right)) = name.split_once("__") {
        let left_id = Uuid::parse_str(left).ok()?;
        let right_id = Uuid::parse_str(right).ok()?;
        return Some(sort_user_ids(left_id, right_id));
    }

    if let Some(rest) = name.strip_prefix("dm_") {
        let (left, right) = rest.split_once('_')?;
        let left_id = Uuid::parse_str(left).ok()?;
        let right_id = Uuid::parse_str(right).ok()?;
        return Some(sort_user_ids(left_id, right_id));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_direct_channel_name, legacy_direct_channel_name, parse_direct_channel_name,
    };
    use uuid::Uuid;

    #[test]
    fn parses_canonical_direct_channel_name() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let canonical = canonical_direct_channel_name(a, b);
        let parsed = parse_direct_channel_name(&canonical).expect("canonical name should parse");
        assert_eq!(canonical_direct_channel_name(parsed.0, parsed.1), canonical);
    }

    #[test]
    fn parses_legacy_direct_channel_name() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let legacy = legacy_direct_channel_name(a, b);
        let parsed = parse_direct_channel_name(&legacy).expect("legacy name should parse");
        assert_eq!(
            canonical_direct_channel_name(parsed.0, parsed.1),
            canonical_direct_channel_name(a, b)
        );
    }

    #[test]
    fn rejects_invalid_direct_channel_name() {
        assert!(parse_direct_channel_name("dm_not-a-uuid_not-a-uuid").is_none());
        assert!(parse_direct_channel_name("invalid").is_none());
    }
}
