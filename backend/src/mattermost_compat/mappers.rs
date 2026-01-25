use super::models as mm;
use crate::models::{
    channel::{Channel, ChannelType},
    post::{Post, PostResponse},
    team::Team,
    user::User,
};
use serde_json::json;

impl From<User> for mm::User {
    fn from(user: User) -> Self {
        mm::User {
            id: user.id.to_string(),
            create_at: user.created_at.timestamp_millis(),
            update_at: user.updated_at.timestamp_millis(),
            delete_at: 0,
            username: user.username,
            first_name: "".to_string(),
            last_name: "".to_string(),
            nickname: user.display_name.unwrap_or_default(),
            email: user.email,
            email_verified: true,
            auth_service: "".to_string(),
            roles: map_role(&user.role),
            locale: "en".to_string(),
            notify_props: json!({ "email": "true", "push": "mention" }),
            props: json!({}),
            last_password_update: 0,
            last_picture_update: 0,
            failed_attempts: 0,
            mfa_active: false,
            timezone: json!({ "automaticTimezone": "UTC", "manualTimezone": "UTC", "useAutomaticTimezone": "true" }),
        }
    }
}

fn map_role(role: &str) -> String {
    match role {
        "system_admin" => "system_admin system_user".to_string(),
        _ => "system_user".to_string(),
    }
}

impl From<Team> for mm::Team {
    fn from(team: Team) -> Self {
        mm::Team {
            id: team.id.to_string(),
            create_at: team.created_at.timestamp_millis(),
            update_at: team.updated_at.timestamp_millis(),
            delete_at: 0,
            display_name: team.display_name.unwrap_or_else(|| team.name.clone()),
            name: team.name,
            description: team.description.unwrap_or_default(),
            email: "".to_string(),
            team_type: if team.is_public { "O".to_string() } else { "I".to_string() },
            company_name: "".to_string(),
            allowed_domains: "".to_string(),
            invite_id: "".to_string(),
            allow_open_invite: team.allow_open_invite,
        }
    }
}

impl From<Channel> for mm::Channel {
    fn from(channel: Channel) -> Self {
        mm::Channel {
            id: channel.id.to_string(),
            create_at: channel.created_at.timestamp_millis(),
            update_at: channel.updated_at.timestamp_millis(),
            delete_at: if channel.is_archived { channel.updated_at.timestamp_millis() } else { 0 },
            team_id: channel.team_id.to_string(),
            channel_type: match channel.channel_type {
                ChannelType::Public => "O",
                ChannelType::Private => "P",
                ChannelType::Direct => "D",
                ChannelType::Group => "G",
            }
            .to_string(),
            display_name: channel.display_name.unwrap_or_else(|| channel.name.clone()),
            name: channel.name,
            header: channel.header.unwrap_or_default(),
            purpose: channel.purpose.unwrap_or_default(),
            last_post_at: 0,
            total_msg_count: 0,
            extra_update_at: 0,
            creator_id: channel.creator_id.unwrap_or_default().to_string(),
        }
    }
}

impl From<Post> for mm::Post {
    fn from(post: Post) -> Self {
        mm::Post {
            id: post.id.to_string(),
            create_at: post.created_at.timestamp_millis(),
            update_at: post.edited_at.unwrap_or(post.created_at).timestamp_millis(),
            delete_at: post.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            edit_at: post.edited_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            user_id: post.user_id.to_string(),
            channel_id: post.channel_id.to_string(),
            root_id: post.root_post_id.unwrap_or_default().to_string(),
            original_id: "".to_string(),
            message: post.message,
            post_type: "".to_string(),
            props: post.props,
            hashtags: "".to_string(),
            file_ids: post.file_ids.iter().map(|id| id.to_string()).collect(),
            pending_post_id: "".to_string(),
            metadata: None,
        }
    }
}

impl From<PostResponse> for mm::Post {
    fn from(post: PostResponse) -> Self {
        mm::Post {
            id: post.id.to_string(),
            create_at: post.created_at.timestamp_millis(),
            update_at: post.edited_at.unwrap_or(post.created_at).timestamp_millis(),
            delete_at: post.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            edit_at: post.edited_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            user_id: post.user_id.to_string(),
            channel_id: post.channel_id.to_string(),
            root_id: post.root_post_id.unwrap_or_default().to_string(),
            original_id: "".to_string(),
            message: post.message,
            post_type: "".to_string(),
            props: post.props,
            hashtags: "".to_string(),
            file_ids: post.file_ids.iter().map(|id| id.to_string()).collect(),
            pending_post_id: post.client_msg_id.unwrap_or_default(),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn test_user_mapping() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let u = User {
            id: user_id,
            org_id: None,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            display_name: Some("Test User".to_string()),
            avatar_url: None,
            is_bot: false,
            is_active: true,
            role: "member".to_string(),
            presence: "offline".to_string(),
            status_text: None,
            status_emoji: None,
            status_expires_at: None,
            custom_status: None,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        };

        let mm_u: mm::User = u.into();
        assert_eq!(mm_u.id, user_id.to_string());
        assert_eq!(mm_u.username, "testuser");
        assert_eq!(mm_u.email, "test@example.com");
        assert_eq!(mm_u.roles, "system_user");
    }

    #[test]
    fn test_channel_mapping() {
        let channel_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();
        let now = Utc::now();
        let c = Channel {
            id: channel_id,
            team_id: team_id,
            channel_type: ChannelType::Public,
            name: "general".to_string(),
            display_name: Some("General".to_string()),
            purpose: Some("Purpose".to_string()),
            header: Some("Header".to_string()),
            is_archived: false,
            creator_id: None,
            created_at: now,
            updated_at: now,
        };

        let mm_c: mm::Channel = c.into();
        assert_eq!(mm_c.id, channel_id.to_string());
        assert_eq!(mm_c.team_id, team_id.to_string());
        assert_eq!(mm_c.channel_type, "O");
        assert_eq!(mm_c.name, "general");
    }
}
