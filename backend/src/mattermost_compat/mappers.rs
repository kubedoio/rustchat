use super::{id::encode_mm_id, models as mm};
use crate::models::{
    channel::{Channel, ChannelMember, ChannelType},
    file::FileInfo,
    post::{Post, PostResponse},
    team::{Team, TeamMember},
    user::User,
};
use serde_json::json;
use uuid::Uuid;

fn post_type_from_props(props: &serde_json::Value) -> String {
    props
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

impl From<User> for mm::User {
    fn from(user: User) -> Self {
        let is_deleted = user.deleted_at.is_some();
        let tz = user
            .timezone
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("UTC");

        mm::User {
            id: encode_mm_id(user.id),
            create_at: user.created_at.timestamp_millis(),
            update_at: user.updated_at.timestamp_millis(),
            delete_at: user.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            username: if is_deleted {
                "Deleted user".to_string()
            } else {
                user.username
            },
            first_name: if is_deleted {
                String::new()
            } else {
                user.first_name.unwrap_or_default()
            },
            last_name: if is_deleted {
                String::new()
            } else {
                user.last_name.unwrap_or_default()
            },
            nickname: if is_deleted {
                "Deleted user".to_string()
            } else {
                user.nickname.or(user.display_name).unwrap_or_default()
            },
            email: if is_deleted {
                "deleted-user@local".to_string()
            } else {
                user.email
            },
            email_verified: true,
            auth_service: "".to_string(),
            roles: map_system_role(&user.role),
            locale: "en".to_string(),
            notify_props: user.notify_props.clone(),
            props: json!({}),
            last_password_update: 0,
            last_picture_update: 0,
            failed_attempts: 0,
            mfa_active: false,
            timezone: json!({
                "automaticTimezone": tz,
                "manualTimezone": tz,
                "useAutomaticTimezone": "true"
            }),
        }
    }
}

pub(crate) fn map_system_role(role: &str) -> String {
    match role {
        "admin" | "system_admin" => "system_admin system_user".to_string(),
        _ => "system_user".to_string(),
    }
}

pub(crate) fn map_team_role(role: &str) -> String {
    match role {
        "admin" | "team_admin" => "team_admin team_user".to_string(),
        _ => "team_user".to_string(),
    }
}

pub(crate) fn map_channel_role(role: &str) -> String {
    match role {
        "admin" | "channel_admin" => "channel_admin channel_user".to_string(),
        _ => "channel_user".to_string(),
    }
}

impl From<Team> for mm::Team {
    fn from(team: Team) -> Self {
        mm::Team {
            id: encode_mm_id(team.id),
            create_at: team.created_at.timestamp_millis(),
            update_at: team.updated_at.timestamp_millis(),
            delete_at: team.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            display_name: team.display_name.unwrap_or_else(|| team.name.clone()),
            name: team.name,
            description: team.description.unwrap_or_default(),
            email: "".to_string(),
            team_type: match team.privacy.as_deref() {
                Some("open") => "O".to_string(),
                Some("invite") => "I".to_string(),
                _ => {
                    if team.is_public {
                        "O".to_string()
                    } else {
                        "I".to_string()
                    }
                }
            },
            company_name: "".to_string(),
            allowed_domains: "".to_string(),
            invite_id: team.invite_id,
            allow_open_invite: team.allow_open_invite,
        }
    }
}

impl From<Channel> for mm::Channel {
    fn from(channel: Channel) -> Self {
        let display_name = match channel.display_name {
            Some(name) if !name.trim().is_empty() => name,
            _ => match channel.channel_type {
                ChannelType::Direct => "Direct Message".to_string(),
                ChannelType::Group => "Group Message".to_string(),
                _ => channel.name.clone(),
            },
        };
        let name = if channel.channel_type == ChannelType::Direct {
            crate::models::parse_direct_channel_name(&channel.name)
                .map(|(left, right)| crate::models::canonical_direct_channel_name(left, right))
                .unwrap_or_else(|| channel.name.clone())
        } else {
            channel.name.clone()
        };

        mm::Channel {
            id: encode_mm_id(channel.id),
            create_at: channel.created_at.timestamp_millis(),
            update_at: channel.updated_at.timestamp_millis(),
            delete_at: channel
                .deleted_at
                .map(|t| t.timestamp_millis())
                .unwrap_or(0),
            team_id: encode_mm_id(channel.team_id),
            channel_type: match channel.channel_type {
                ChannelType::Public => "O",
                ChannelType::Private => "P",
                ChannelType::Direct => "D",
                ChannelType::Group => "G",
            }
            .to_string(),
            display_name,
            name,
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
        let post_type = post_type_from_props(&post.props);
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
            post_type,
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
        let post_type = post_type_from_props(&post.props);
        // Build metadata with files if files are present
        let metadata = if !post.files.is_empty() {
            let mm_files: Vec<mm::FileInfo> = post
                .files
                .into_iter()
                .map(|f| mm::FileInfo {
                    id: encode_mm_id(f.id),
                    user_id: String::new(), // Not available in FileUploadResponse
                    post_id: encode_mm_id(post.id),
                    channel_id: encode_mm_id(post.channel_id),
                    create_at: post.created_at.timestamp_millis(), // Fallback
                    update_at: post.created_at.timestamp_millis(), // Fallback
                    delete_at: 0,
                    name: f.name.clone(),
                    extension: f
                        .name
                        .rsplit_once('.')
                        .map(|(_, ext)| ext)
                        .unwrap_or_default()
                        .to_string(),
                    size: f.size,
                    mime_type: f.mime_type,
                    width: f.width,
                    height: f.height,
                    has_preview_image: f.thumbnail_url.is_some(),
                    mini_preview: None,
                })
                .collect();
            Some(json!({
                "files": mm_files,
                "reactions": post.reactions.iter().map(|r| json!({
                    "user_id": encode_mm_id(r.users.first().copied().unwrap_or_else(Uuid::nil)),
                    "post_id": encode_mm_id(post.id),
                    "emoji_name": r.emoji,
                    "create_at": post.created_at.timestamp_millis()
                })).collect::<Vec<_>>()
            }))
        } else if !post.reactions.is_empty() {
            Some(json!({
                "reactions": post.reactions.iter().map(|r| json!({
                    "user_id": encode_mm_id(r.users.first().copied().unwrap_or_else(Uuid::nil)),
                    "post_id": encode_mm_id(post.id),
                    "emoji_name": r.emoji,
                    "create_at": post.created_at.timestamp_millis()
                })).collect::<Vec<_>>()
            }))
        } else {
            None
        };

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
            post_type,
            props: post.props,
            hashtags: "".to_string(),
            file_ids: post.file_ids.iter().map(|id| encode_mm_id(*id)).collect(),
            pending_post_id: post.client_msg_id.unwrap_or_default(),
            metadata,
        }
    }
}

impl From<TeamMember> for mm::TeamMember {
    fn from(m: TeamMember) -> Self {
        mm::TeamMember {
            team_id: encode_mm_id(m.team_id),
            user_id: encode_mm_id(m.user_id),
            roles: map_team_role(&m.role),
            delete_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: m.role == "admin" || m.role == "system_admin",
            presence: Some(m.presence).filter(|p| !p.is_empty()),
        }
    }
}

impl From<ChannelMember> for mm::ChannelMember {
    fn from(m: ChannelMember) -> Self {
        mm::ChannelMember {
            channel_id: encode_mm_id(m.channel_id),
            user_id: encode_mm_id(m.user_id),
            roles: map_channel_role(&m.role),
            last_viewed_at: m.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            msg_count: m.msg_count,
            mention_count: m.mention_count,
            mention_count_root: m.mention_count_root,
            urgent_mention_count: m.urgent_mention_count,
            msg_count_root: m.msg_count_root,
            notify_props: m.notify_props.clone(),
            last_update_at: m.last_update_at.unwrap_or(m.created_at).timestamp_millis(),
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: m.role == "admin" || m.role == "channel_admin",
        }
    }
}

impl From<FileInfo> for mm::FileInfo {
    fn from(f: FileInfo) -> Self {
        let extension = f
            .name
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .unwrap_or_default()
            .to_string();

        mm::FileInfo {
            id: encode_mm_id(f.id),
            user_id: encode_mm_id(f.uploader_id),
            post_id: f.post_id.map(encode_mm_id).unwrap_or_default(),
            channel_id: f.channel_id.map(encode_mm_id).unwrap_or_default(),
            create_at: f.created_at.timestamp_millis(),
            update_at: f.created_at.timestamp_millis(),
            delete_at: 0,
            name: f.name.clone(),
            extension,
            size: f.size,
            mime_type: f.mime_type,
            width: f.width.unwrap_or(0),
            height: f.height.unwrap_or(0),
            has_preview_image: f.has_thumbnail,
            mini_preview: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::file::FileUploadResponse;
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
            password_hash: Some("hash".to_string()),
            display_name: Some("Test User".to_string()),
            avatar_url: None,
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            nickname: None,
            position: None,
            is_bot: false,
            is_active: true,
            role: "member".to_string(),
            presence: "offline".to_string(),
            status_text: None,
            status_emoji: None,
            status_expires_at: None,
            custom_status: None,
            notify_props: serde_json::json!({}),
            timezone: Some("UTC".to_string()),
            last_login_at: None,
            email_verified: true,
            email_verified_at: Some(now),
            deleted_at: None,
            deleted_by: None,
            delete_reason: None,
            created_at: now,
            updated_at: now,
            entity_type: crate::models::entity::EntityType::Human,
            api_key_hash: None,
            entity_metadata: serde_json::json!({}),
            rate_limit_tier: None,
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
            team_id,
            channel_type: ChannelType::Public,
            name: "general".to_string(),
            display_name: Some("General".to_string()),
            purpose: Some("Purpose".to_string()),
            header: Some("Header".to_string()),
            is_archived: false,
            creator_id: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };

        let mm_c: mm::Channel = c.into();
        assert_eq!(mm_c.id, encode_mm_id(channel_id));
        assert_eq!(mm_c.team_id, encode_mm_id(team_id));
        assert_eq!(mm_c.channel_type, "O");
        assert_eq!(mm_c.name, "general");
    }

    #[test]
    fn test_post_response_file_dimensions_mapped_to_metadata() {
        let now = Utc::now();
        let post_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();

        let post = PostResponse {
            id: post_id,
            channel_id,
            user_id,
            root_post_id: None,
            message: "image post".to_string(),
            props: serde_json::json!({}),
            file_ids: vec![file_id],
            is_pinned: false,
            created_at: now,
            edited_at: None,
            deleted_at: None,
            reply_count: 0,
            last_reply_at: None,
            username: Some("alice".to_string()),
            avatar_url: None,
            email: None,
            is_bot: false,
            files: vec![FileUploadResponse {
                id: file_id,
                name: "photo.jpg".to_string(),
                mime_type: "image/jpeg".to_string(),
                size: 1024,
                width: 1280,
                height: 720,
                url: "/api/v4/files/file".to_string(),
                thumbnail_url: Some("/api/v4/files/file/thumbnail".to_string()),
            }],
            reactions: vec![],
            is_saved: false,
            client_msg_id: None,
            seq: 1,
        };

        let mm_post: mm::Post = post.into();
        let metadata = mm_post.metadata.expect("metadata should exist");
        let files = metadata
            .get("files")
            .and_then(|v| v.as_array())
            .expect("files metadata should exist");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].get("width"), Some(&serde_json::json!(1280)));
        assert_eq!(files[0].get("height"), Some(&serde_json::json!(720)));
    }
}
