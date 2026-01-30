use super::{id::encode_mm_id, models as mm};
use crate::models::{
    channel::{Channel, ChannelMember, ChannelType},
    post::{Post, PostResponse},
    team::{Team, TeamMember},
    user::User,
    file::FileInfo,
};
use serde_json::json;

impl From<User> for mm::User {
    fn from(user: User) -> Self {
        mm::User {
            id: encode_mm_id(user.id),
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
            id: encode_mm_id(team.id),
            create_at: team.created_at.timestamp_millis(),
            update_at: team.updated_at.timestamp_millis(),
            delete_at: 0,
            display_name: team.display_name.unwrap_or_else(|| team.name.clone()),
            name: team.name,
            description: team.description.unwrap_or_default(),
            email: "".to_string(),
            team_type: if team.is_public {
                "O".to_string()
            } else {
                "I".to_string()
            },
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
            id: encode_mm_id(channel.id),
            create_at: channel.created_at.timestamp_millis(),
            update_at: channel.updated_at.timestamp_millis(),
            delete_at: if channel.is_archived {
                channel.updated_at.timestamp_millis()
            } else {
                0
            },
            team_id: encode_mm_id(channel.team_id),
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
            creator_id: channel.creator_id.map(encode_mm_id).unwrap_or_default(),
        }
    }
}

impl From<Post> for mm::Post {
    fn from(post: Post) -> Self {
        mm::Post {
            id: encode_mm_id(post.id),
            create_at: post.created_at.timestamp_millis(),
            update_at: post.edited_at.unwrap_or(post.created_at).timestamp_millis(),
            delete_at: post.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            edit_at: post.edited_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            user_id: encode_mm_id(post.user_id),
            channel_id: encode_mm_id(post.channel_id),
            root_id: post.root_post_id.map(encode_mm_id).unwrap_or_default(),
            original_id: "".to_string(),
            message: post.message,
            post_type: "".to_string(),
            props: post.props,
            hashtags: "".to_string(),
            file_ids: post.file_ids.iter().map(|id| encode_mm_id(*id)).collect(),
            pending_post_id: "".to_string(),
            metadata: None,
        }
    }
}

impl From<PostResponse> for mm::Post {
    fn from(post: PostResponse) -> Self {
        mm::Post {
            id: encode_mm_id(post.id),
            create_at: post.created_at.timestamp_millis(),
            update_at: post.edited_at.unwrap_or(post.created_at).timestamp_millis(),
            delete_at: post.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            edit_at: post.edited_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            user_id: encode_mm_id(post.user_id),
            channel_id: encode_mm_id(post.channel_id),
            root_id: post.root_post_id.map(encode_mm_id).unwrap_or_default(),
            original_id: "".to_string(),
            message: post.message,
            post_type: "".to_string(),
            props: post.props,
            hashtags: "".to_string(),
            file_ids: post.file_ids.iter().map(|id| encode_mm_id(*id)).collect(),
            pending_post_id: post.client_msg_id.unwrap_or_default(),
            metadata: None,
        }
    }
}

impl From<TeamMember> for mm::TeamMember {
    fn from(m: TeamMember) -> Self {
        mm::TeamMember {
            team_id: encode_mm_id(m.team_id),
            user_id: encode_mm_id(m.user_id),
            roles: map_role(&m.role),
            delete_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: m.role == "admin" || m.role == "system_admin",
        }
    }
}

impl From<ChannelMember> for mm::ChannelMember {
    fn from(m: ChannelMember) -> Self {
        mm::ChannelMember {
            channel_id: encode_mm_id(m.channel_id),
            user_id: encode_mm_id(m.user_id),
            roles: map_role(&m.role),
            last_viewed_at: m.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            msg_count: 0,
            mention_count: 0,
            notify_props: m.notify_props.clone(),
            last_update_at: m.created_at.timestamp_millis(),
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: m.role == "admin" || m.role == "channel_admin",
        }
    }
}

impl From<FileInfo> for mm::FileInfo {
    fn from(f: FileInfo) -> Self {
        mm::FileInfo {
            id: encode_mm_id(f.id),
            user_id: encode_mm_id(f.uploader_id),
            create_at: f.created_at.timestamp_millis(),
            update_at: f.created_at.timestamp_millis(),
            delete_at: 0,
            name: f.name.clone(),
            extension: f.name.rsplit('.').next().unwrap_or_default().to_string(),
            size: f.size,
            mime_type: f.mime_type,
            width: f.width.unwrap_or(0),
            height: f.height.unwrap_or(0),
            has_preview_image: f.has_thumbnail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

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
            password_updated_at: now,
        };

        let mm_u: mm::User = u.into();
        assert_eq!(mm_u.id, encode_mm_id(user_id));
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
        assert_eq!(mm_c.id, encode_mm_id(channel_id));
        assert_eq!(mm_c.team_id, encode_mm_id(team_id));
        assert_eq!(mm_c.channel_type, "O");
        assert_eq!(mm_c.name, "general");
    }
}
