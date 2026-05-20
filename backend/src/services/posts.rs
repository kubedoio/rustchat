use crate::mattermost_compat::models as mm;
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::models::{
    normalize_avatar_url, ChannelMember, CreatePost, FileUploadResponse, Post, PostResponse,
};
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};
use crate::services::activity;
use regex::Regex;

#[derive(Debug, Default)]
pub struct PostsQuery {
    pub page: i64,
    pub per_page: i64,
    pub since: Option<i64>,
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
}

async fn validate_create_post(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    input: &CreatePost,
) -> ApiResult<Option<Uuid>> {
    ensure_permission(state, user_id, "post.create").await?;

    // Check membership
    let _: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Validate message
    if input.message.trim().is_empty() && input.file_ids.is_empty() {
        return Err(AppError::Validation("Message cannot be empty".to_string()));
    }

    // Validate root_post_id if provided
    let root_post_id = input.root_post_id;
    if let Some(r_id) = root_post_id {
        let root_post: Option<Post> = sqlx::query_as(
            r#"
            SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
                   is_pinned, created_at, edited_at, deleted_at,
                   reply_count::int8 as reply_count,
                   last_reply_at, seq
            FROM posts WHERE id = $1 AND channel_id = $2
            "#,
        )
        .bind(r_id)
        .bind(channel_id)
        .fetch_optional(&state.db)
        .await?;

        if root_post.is_none() {
            return Err(AppError::BadRequest("Invalid root post".to_string()));
        }
    }

    Ok(root_post_id)
}

async fn insert_post(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    input: &CreatePost,
    root_post_id: Option<Uuid>,
) -> ApiResult<Post> {
    let post: Post = sqlx::query_as(
        r#"
        INSERT INTO posts (channel_id, user_id, root_post_id, message, props, file_ids)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                  is_pinned, created_at, edited_at, deleted_at,
                  reply_count::int8 as reply_count,
                  last_reply_at, seq
        "#,
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(root_post_id)
    .bind(&input.message)
    .bind(input.props.clone().unwrap_or(serde_json::json!({})))
    .bind(&input.file_ids)
    .fetch_one(&state.db)
    .await?;

    Ok(post)
}

async fn handle_reply_side_effects(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    post: &Post,
    root_post_id: Uuid,
    message: &str,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE posts SET reply_count = reply_count + 1, last_reply_at = NOW() WHERE id = $1",
    )
    .bind(root_post_id)
    .execute(&state.db)
    .await?;

    // Create reply/thread_reply activity for the parent post author
    let parent_info: Option<(Uuid, Option<Uuid>)> =
        sqlx::query_as("SELECT user_id, root_post_id FROM posts WHERE id = $1")
            .bind(root_post_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();

    if let Some((parent_user_id, parent_root_id)) = parent_info {
        if parent_user_id != user_id {
            let team_id_for_reply: Option<Uuid> =
                sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
                    .bind(channel_id)
                    .fetch_optional(&state.db)
                    .await
                    .ok()
                    .flatten();

            if let Some(team_id) = team_id_for_reply {
                if parent_root_id.is_some() {
                    // Parent is itself a reply, this is a thread reply
                    let _ = activity::create_thread_reply_activity(
                        state,
                        parent_user_id,
                        user_id,
                        channel_id,
                        team_id,
                        post.id,
                        root_post_id,
                        message,
                    )
                    .await;
                } else {
                    // Parent is a root post, this is a direct reply
                    let _ = activity::create_reply_activity(
                        state,
                        parent_user_id,
                        user_id,
                        channel_id,
                        team_id,
                        post.id,
                        message,
                    )
                    .await;
                }
            }
        }
    }

    Ok(())
}

async fn build_post_response(
    state: &AppState,
    post: Post,
    user_id: Uuid,
    client_msg_id: Option<String>,
) -> ApiResult<PostResponse> {
    #[derive(sqlx::FromRow)]
    struct PostUser {
        username: String,
        avatar_url: Option<String>,
        email: String,
    }

    let user: PostUser =
        sqlx::query_as("SELECT username, avatar_url, email FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    let mut response = PostResponse {
        id: post.id,
        channel_id: post.channel_id,
        user_id: post.user_id,
        root_post_id: post.root_post_id,
        message: post.message,
        props: post.props,
        file_ids: post.file_ids,
        is_pinned: post.is_pinned,
        created_at: post.created_at,
        edited_at: post.edited_at,
        deleted_at: post.deleted_at,
        username: Some(user.username),
        avatar_url: user.avatar_url,
        email: Some(user.email),
        reply_count: post.reply_count,
        last_reply_at: post.last_reply_at,
        files: vec![],
        reactions: vec![],
        is_saved: false,
        client_msg_id,
        seq: post.seq,
    };
    response.avatar_url = normalize_avatar_url(response.user_id, response.avatar_url.as_deref());

    // Populate files if any
    if !response.file_ids.is_empty() {
        populate_files(state, std::slice::from_mut(&mut response)).await?;
    }

    Ok(response)
}

async fn broadcast_new_post(
    state: &AppState,
    channel_id: Uuid,
    response: &PostResponse,
    root_post_id: Option<Uuid>,
) {
    let event_type = if root_post_id.is_some() {
        EventType::ThreadReplyCreated
    } else {
        EventType::MessageCreated
    };

    let mm_post = mm::Post::from(response.clone());
    let broadcast =
        WsEnvelope::event(event_type, mm_post, Some(channel_id)).with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });

    state.ws_hub.broadcast(broadcast).await;

    // If reply, broadcast update to root post
    if let Some(r_id) = root_post_id {
        let root_update = WsEnvelope::event(
            EventType::MessageUpdated,
            serde_json::json!({
                "id": r_id,
                "reply_count_inc": 1,
                "last_reply_at": response.created_at
            }),
            Some(channel_id),
        )
        .with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(root_update).await;
    }
}

async fn run_post_automation(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    response: &PostResponse,
    root_post_id: Option<Uuid>,
) {
    // Check for playbook triggers
    if root_post_id.is_none() {
        let _ = check_playbook_triggers(state, channel_id, &response.message).await;
    }

    // Check for outgoing webhook triggers
    if root_post_id.is_none() {
        // Get team_id for the channel
        if let Ok(team_id) =
            sqlx::query_scalar::<_, Uuid>("SELECT team_id FROM channels WHERE id = $1")
                .bind(channel_id)
                .fetch_one(&state.db)
                .await
        {
            // Get channel name and username
            let channel_name: String =
                sqlx::query_scalar("SELECT name FROM channels WHERE id = $1")
                    .bind(channel_id)
                    .fetch_one(&state.db)
                    .await
                    .unwrap_or_default();
            let username = response.username.clone().unwrap_or_default();

            let _ = crate::services::webhooks::check_outgoing_triggers(
                state,
                channel_id,
                team_id,
                user_id,
                &username,
                &channel_name,
                &response.message,
            )
            .await;
        }
    }
}

async fn handle_mentions(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    post_id: Uuid,
    response: &mut PostResponse,
) -> ApiResult<Vec<String>> {
    // Parse mentions using regex, excluding code blocks and URLs
    let mentions = parse_mentions(&response.message);

    if !mentions.is_empty() {
        // We could store these in the DB if we wanted persistent notifications
        // For now, we'll just include them in the broadcast if needed,
        // but the frontend already parses them from the message string.
        // Let's at least update the props to include mentions metadata.
        let mut props = response.props.as_object().cloned().unwrap_or_default();
        props.insert("mentions".to_string(), serde_json::json!(&mentions));

        // Update DB with the new props
        sqlx::query("UPDATE posts SET props = $1 WHERE id = $2")
            .bind(serde_json::Value::Object(props.clone()))
            .bind(post_id)
            .execute(&state.db)
            .await
            .ok();

        response.props = serde_json::Value::Object(props);

        // Create mention activities for mentioned users
        // Get team_id for the channel
        let team_id_opt: Option<Uuid> =
            sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
                .bind(channel_id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

        if let Some(team_id) = team_id_opt {
            // Batch-resolve usernames to user IDs
            let usernames = mentions.iter().map(|m| m.as_str()).collect::<Vec<_>>();
            let users: Vec<(Uuid, String)> = sqlx::query_as(
                "SELECT id, username FROM users WHERE username = ANY($1)"
            )
            .bind(&usernames)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let mentioned_user_ids: Vec<Uuid> = users
                .into_iter()
                .map(|(id, _)| id)
                .filter(|id| *id != user_id)
                .collect();

            if !mentioned_user_ids.is_empty() {
                // Batch-check channel memberships
                let member_ids: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>(
                    "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id = ANY($2)"
                )
                .bind(channel_id)
                .bind(&mentioned_user_ids)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();

                for mentioned_user_id in member_ids {
                    let _ = activity::create_mention_activity(
                        state,
                        mentioned_user_id,
                        user_id,
                        channel_id,
                        team_id,
                        post_id,
                        &response.message,
                    )
                    .await;
                }
            }
        }
    }

    Ok(mentions)
}

async fn send_push_notifications(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    _post_id: Uuid,
    response: &PostResponse,
    mentions: &[String],
    username_for_push: String,
) {
    // Get channel info for push notifications
    let channel_info: Option<(String, String, String)> = sqlx::query_as(
        "SELECT c.name, c.display_name, c.type::text as channel_type FROM channels c WHERE c.id = $1"
    )
    .bind(channel_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    if let Some((channel_name, channel_display_name, channel_type)) = channel_info {
        let is_dm = channel_type == "direct";
        let sender_name = username_for_push;
        let message_preview = truncate_preview(&response.message, 100);

        // Get channel members to notify
        let members_to_notify: Vec<Uuid> = if is_dm {
            // For DMs, notify the other participant
            sqlx::query_scalar::<_, Uuid>(
                "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id != $2",
            )
            .bind(channel_id)
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
        } else if !mentions.is_empty() {
            // For mentions, find the mentioned users who are channel members
            let usernames = mentions.iter().map(|m| m.as_str()).collect::<Vec<_>>();
            sqlx::query_scalar::<_, Uuid>(
                "SELECT cm.user_id FROM channel_members cm JOIN users u ON cm.user_id = u.id WHERE cm.channel_id = $1 AND u.username = ANY($2)"
            )
            .bind(channel_id)
            .bind(&usernames)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
        } else {
            // No mentions and not a DM - don't send push notification for regular messages
            vec![]
        };

        // Send push notifications asynchronously with bounded concurrency
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(50));
        for target_user_id in members_to_notify {
            // Don't notify the sender
            if target_user_id == user_id {
                continue;
            }

            let display_channel_name = if !channel_display_name.is_empty() {
                channel_display_name.clone()
            } else {
                channel_name.clone()
            };

            let state_clone = state.clone();
            let sender_name_clone = sender_name.clone();
            let message_preview_clone = message_preview.clone();

            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore closed");

            tokio::spawn(async move {
                let _permit = permit;
                match crate::services::push_notifications::send_message_notification(
                    &state_clone,
                    target_user_id,
                    channel_id,
                    display_channel_name,
                    sender_name_clone,
                    message_preview_clone,
                    is_dm,
                )
                .await
                {
                    Ok(count) if count > 0 => {
                        tracing::debug!(
                            user_id = %target_user_id,
                            "Sent push notification for message"
                        );
                    }
                    Ok(_) => {
                        // No devices to notify
                    }
                    Err(e) => {
                        tracing::debug!(
                            user_id = %target_user_id,
                            error = %e,
                            "Failed to send push notification for message"
                        );
                    }
                }
            });
        }
    }
}

/// Create a new post
pub async fn create_post(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    input: CreatePost,
    client_msg_id: Option<String>,
) -> ApiResult<PostResponse> {
    let root_post_id = validate_create_post(state, user_id, channel_id, &input).await?;
    let post = insert_post(state, user_id, channel_id, &input, root_post_id).await?;

    if let Some(r_id) = root_post_id {
        handle_reply_side_effects(state, user_id, channel_id, &post, r_id, &input.message).await?;
    }

    let mut response = build_post_response(state, post, user_id, client_msg_id).await?;

    // MENTIONS BEFORE BROADCAST (bug fix)
    let mentions = handle_mentions(state, channel_id, user_id, response.id, &mut response).await?;

    broadcast_new_post(state, channel_id, &response, root_post_id).await;
    run_post_automation(state, channel_id, user_id, &response, root_post_id).await;

    let _ = ensure_dm_membership(state, channel_id).await;
    let _ = crate::services::unreads::increment_unreads(state, channel_id, user_id, response.seq).await;

    let username_for_push = response.username.clone().unwrap_or_default();
    send_push_notifications(state, channel_id, user_id, response.id, &response, &mentions, username_for_push).await;

    Ok(response)
}

fn truncate_preview(message: &str, max_chars: usize) -> String {
    let mut chars = message.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

async fn ensure_permission(state: &AppState, user_id: Uuid, permission: &str) -> ApiResult<()> {
    let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

    let allowed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM role_permissions WHERE role = $1 AND permission_id = $2)",
    )
    .bind(&role)
    .bind(permission)
    .fetch_one(&state.db)
    .await?;

    if !allowed {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(())
}

/// Helper to ensure all participants of a DM are members (resurrects DM)
pub async fn ensure_dm_membership(state: &AppState, channel_id: Uuid) -> ApiResult<()> {
    // 1. Get channel info
    let chan: crate::models::Channel =
        match sqlx::query_as("SELECT * FROM channels WHERE id = $1 AND deleted_at IS NULL")
            .bind(channel_id)
            .fetch_optional(&state.db)
            .await?
        {
            Some(c) => c,
            None => return Ok(()),
        };

    if chan.channel_type == crate::models::ChannelType::Direct {
        if let Some((u1, u2)) = crate::models::parse_direct_channel_name(&chan.name) {
            for target_user_id in [u1, u2] {
                // Ensure member exists
                let added: Option<Uuid> = sqlx::query_scalar(
                    r#"
                        INSERT INTO channel_members (channel_id, user_id, role) 
                        VALUES ($1, $2, 'member') 
                        ON CONFLICT (channel_id, user_id) DO NOTHING
                        RETURNING user_id
                        "#,
                )
                .bind(channel_id)
                .bind(target_user_id)
                .fetch_optional(&state.db)
                .await?;

                if added.is_some() {
                    // User was missing and just re-added.
                    // Broadcast ChannelCreated to them so their UI opens it.
                    let event = WsEnvelope::event(
                        EventType::ChannelCreated,
                        chan.clone(),
                        Some(channel_id),
                    )
                    .with_broadcast(WsBroadcast {
                        user_id: Some(target_user_id),
                        channel_id: None,
                        team_id: None,
                        exclude_user_id: None,
                    });
                    state.ws_hub.broadcast(event).await;
                }
            }
        }
    }
    Ok(())
}

/// Helper to populate files for posts
/// Uses authenticated API endpoints instead of presigned S3 URLs
/// This ensures files remain accessible after re-login and require authentication
pub async fn populate_files(state: &AppState, posts: &mut [PostResponse]) -> ApiResult<()> {
    use crate::mattermost_compat::id::encode_mm_id;

    normalize_post_avatar_urls(posts);

    // 1. Collect all file IDs
    let all_file_ids: Vec<Uuid> = posts.iter().flat_map(|p| p.file_ids.clone()).collect();

    if all_file_ids.is_empty() {
        return Ok(());
    }

    // 2. Fetch file infos
    let files: Vec<crate::models::FileInfo> =
        sqlx::query_as("SELECT id, uploader_id, channel_id, post_id, name, key, mime_type, size, backend, width, height, has_thumbnail, thumbnail_key, sha256, created_at FROM files WHERE id = ANY($1)")
            .bind(&all_file_ids)
            .fetch_all(&state.db)
            .await?;

    // 3. Generate authenticated API URLs (not presigned S3 URLs)
    // These URLs require authentication and don't expire
    let mut file_map = HashMap::new();
    for file in files {
        let mm_file_id = encode_mm_id(file.id);

        // Use authenticated API endpoints instead of presigned S3 URLs
        // This ensures:
        // 1. Files require authentication to access
        // 2. URLs don't expire after logout/login
        // 3. Original filenames are preserved in Content-Disposition header
        let url = format!("/api/v4/files/{}", mm_file_id);
        let thumbnail_url = if file.has_thumbnail {
            Some(format!("/api/v4/files/{}/thumbnail", mm_file_id))
        } else {
            None
        };

        file_map.insert(
            file.id,
            FileUploadResponse {
                id: file.id,
                name: file.name,
                mime_type: file.mime_type,
                size: file.size,
                width: file.width.unwrap_or(0),
                height: file.height.unwrap_or(0),
                url,
                thumbnail_url,
            },
        );
    }

    for post in posts {
        post.files.clear();
        for file_id in &post.file_ids {
            if let Some(file_resp) = file_map.get(file_id) {
                post.files.push(file_resp.clone());
            }
        }
    }

    Ok(())
}

pub fn normalize_post_avatar_urls(posts: &mut [PostResponse]) {
    for post in posts {
        post.avatar_url = normalize_avatar_url(post.user_id, post.avatar_url.as_deref());
    }
}

/// Create a system message in a channel
pub async fn create_system_message(
    state: &AppState,
    channel_id: Uuid,
    message: String,
    props: Option<serde_json::Value>,
) -> ApiResult<()> {
    // 1. Find bot user
    let bot_user =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE is_bot = true LIMIT 1")
            .fetch_optional(&state.db)
            .await?
            .unwrap_or_else(Uuid::nil);

    // 2. Prepare props
    let mut final_props = props.unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = final_props.as_object_mut() {
        if !obj.contains_key("type") {
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("system_join_leave".to_string()),
            );
        }
    }

    // 3. Insert post
    let post: Post = sqlx::query_as(
        r#"
        INSERT INTO posts (channel_id, user_id, message, props)
        VALUES ($1, $2, $3, $4)
        RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                  is_pinned, created_at, edited_at, deleted_at,
                  reply_count::int8 as reply_count,
                  last_reply_at, seq
        "#,
    )
    .bind(channel_id)
    .bind(bot_user)
    .bind(&message)
    .bind(&final_props)
    .fetch_one(&state.db)
    .await?;

    // 4. Construct response
    let response = PostResponse {
        id: post.id,
        channel_id: post.channel_id,
        user_id: post.user_id,
        root_post_id: post.root_post_id,
        message: post.message,
        props: post.props,
        file_ids: post.file_ids,
        is_pinned: post.is_pinned,
        created_at: post.created_at,
        edited_at: post.edited_at,
        deleted_at: post.deleted_at,
        username: Some("System".to_string()),
        avatar_url: None,
        email: None,
        reply_count: 0,
        last_reply_at: None,
        files: vec![],
        reactions: vec![],
        is_saved: false,
        client_msg_id: None,
        seq: post.seq,
    };

    // 5. Broadcast
    let broadcast = WsEnvelope::event(EventType::MessageCreated, response, Some(channel_id))
        .with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });

    state.ws_hub.broadcast(broadcast).await;

    // Increment unread counts in Redis for other members
    let _ =
        crate::services::unreads::increment_unreads(state, channel_id, bot_user, post.seq).await;

    Ok(())
}

async fn check_playbook_triggers(
    state: &AppState,
    channel_id: Uuid,
    message: &str,
) -> ApiResult<()> {
    // 1. Get team_id
    let channel_info = sqlx::query_scalar::<_, Uuid>("SELECT team_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(&state.db)
        .await?;

    if let Some(chan) = channel_info {
        // 2. Fetch playbooks with triggers
        let playbooks = sqlx::query_as::<_, crate::models::Playbook>(
            "SELECT * FROM playbooks WHERE team_id = $1 AND is_archived = false AND keyword_triggers IS NOT NULL LIMIT 100"
        )
        .bind(chan)
        .fetch_all(&state.db)
        .await?;

        // 3. Find bot user (optional)
        let bot_user =
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE is_bot = true LIMIT 1")
                .fetch_optional(&state.db)
                .await?;

        let lower_message = message.to_lowercase();

        for playbook in playbooks {
            if let Some(triggers) = &playbook.keyword_triggers {
                for trigger in triggers {
                    if !trigger.is_empty() && lower_message.contains(&trigger.to_lowercase()) {
                        // Match found
                        let system_msg = format!(
                            "**Playbook Trigger**: Keyword '{}' detected.\n[Start Run for {}](/playbooks/{}/start)",
                            trigger, playbook.name, playbook.id
                        );

                        // Insert post
                        sqlx::query(
                            r#"
                            INSERT INTO posts (channel_id, user_id, message, props)
                            VALUES ($1, $2, $3, $4)
                            "#,
                        )
                        .bind(channel_id)
                        .bind(bot_user.unwrap_or_else(Uuid::nil))
                        .bind(&system_msg)
                        .bind(serde_json::json!({
                            "type": "system_playbook_trigger",
                            "override_username": "Playbook Bot",
                            "playbook_id": playbook.id
                        }))
                        .execute(&state.db)
                        .await
                        .ok();

                        return Ok(());
                    }
                }
            }
        }
    }
    Ok(())
}

/// Get posts for a channel with various pagination options
pub async fn get_posts(
    state: &AppState,
    channel_id: Uuid,
    query: PostsQuery,
) -> ApiResult<(Vec<PostResponse>, i64)> {
    let per_page = if query.per_page > 0 {
        query.per_page
    } else {
        60
    }
    .min(200);
    let offset = query.page * per_page;

    let posts: Vec<PostResponse> = if let Some(since) = query.since {
        let since_time =
            chrono::DateTime::from_timestamp_millis(since).unwrap_or_else(chrono::Utc::now);

        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 
              AND (p.created_at >= $2 OR p.edited_at >= $2)
            ORDER BY p.created_at ASC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(since_time)
        .bind(per_page)
        .fetch_all(&state.db)
        .await?
    } else if let Some(before_id) = query.before {
        let before_time: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT created_at FROM posts WHERE id = $1")
                .bind(before_id)
                .fetch_optional(&state.db)
                .await?;

        let before_time =
            before_time.ok_or_else(|| AppError::NotFound("Before post not found".to_string()))?;

        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 
              AND p.deleted_at IS NULL
              AND p.created_at < $2
            ORDER BY p.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(before_time)
        .bind(per_page)
        .fetch_all(&state.db)
        .await?
    } else if let Some(after_id) = query.after {
        let after_time: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT created_at FROM posts WHERE id = $1")
                .bind(after_id)
                .fetch_optional(&state.db)
                .await?;

        let after_time =
            after_time.ok_or_else(|| AppError::NotFound("After post not found".to_string()))?;

        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 
              AND p.deleted_at IS NULL
              AND p.created_at > $2
            ORDER BY p.created_at ASC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(after_time)
        .bind(per_page)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 
              AND p.deleted_at IS NULL
            ORDER BY p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(channel_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE channel_id = $1 AND deleted_at IS NULL",
    )
    .bind(channel_id)
    .fetch_one(&state.db)
    .await?;

    let mut posts = posts;
    normalize_post_avatar_urls(&mut posts);
    if !posts.is_empty() {
        populate_files(state, &mut posts).await?;
    }

    Ok((posts, total))
}

pub async fn get_post_by_id(state: &AppState, post_id: Uuid) -> ApiResult<PostResponse> {
    let post: PostResponse = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.id = $1
        "#,
    )
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let mut post = post;
    post.avatar_url = normalize_avatar_url(post.user_id, post.avatar_url.as_deref());
    populate_files(state, std::slice::from_mut(&mut post)).await?;

    Ok(post)
}

/// Query parameters for thread fetching
#[derive(Debug, Default)]
pub struct ThreadQuery {
    pub cursor: Option<Uuid>,
    pub limit: i64,
}

/// Get thread with parent post and replies
pub async fn get_thread(
    state: &AppState,
    post_id: Uuid,
    cursor: Option<Uuid>,
    limit: i64,
) -> ApiResult<crate::models::ThreadResponse> {
    let limit = limit.clamp(1, 100);

    // Fetch parent post with user info
    let parent: Option<PostResponse> = sqlx::query_as(
        r#"
        SELECT
            p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
            p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
            p.reply_count::int8 as reply_count, p.last_reply_at, p.seq,
            u.username, u.avatar_url, u.email
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.id = $1 AND p.deleted_at IS NULL
        "#,
    )
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?;

    let mut parent = parent.ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;
    parent.avatar_url = normalize_avatar_url(parent.user_id, parent.avatar_url.as_deref());

    // Build query for replies
    let mut query = String::from(
        r#"
        SELECT
            p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
            p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
            p.reply_count::int8 as reply_count, p.last_reply_at, p.seq,
            u.username, u.avatar_url, u.email
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.root_post_id = $1 AND p.deleted_at IS NULL
        "#,
    );

    if cursor.is_some() {
        query.push_str(" AND p.id > $2 ");
    }

    query.push_str("ORDER BY p.created_at ASC LIMIT $3");

    // Fetch replies
    let replies: Vec<PostResponse> = if let Some(cursor_id) = cursor {
        sqlx::query_as(&query)
            .bind(post_id)
            .bind(cursor_id)
            .bind(limit + 1) // Fetch one extra to determine if there's more
            .fetch_all(&state.db)
            .await?
    } else {
        let query_no_cursor = query.replace("AND p.id > $2", "");
        sqlx::query_as(&query_no_cursor.replace("$3", "$2"))
            .bind(post_id)
            .bind(limit + 1)
            .fetch_all(&state.db)
            .await?
    };

    // Determine pagination
    let has_more = replies.len() > limit as usize;
    let mut replies: Vec<PostResponse> = replies.into_iter().take(limit as usize).collect();
    normalize_post_avatar_urls(&mut replies);

    let next_cursor = if has_more {
        replies.last().map(|r| r.id.to_string())
    } else {
        None
    };

    // Build response
    let mut order = vec![parent.id.to_string()];
    let mut posts_map = std::collections::HashMap::new();
    posts_map.insert(parent.id.to_string(), parent);

    for reply in replies {
        order.push(reply.id.to_string());
        posts_map.insert(reply.id.to_string(), reply);
    }

    Ok(crate::models::ThreadResponse {
        order,
        posts: posts_map,
        next_cursor,
    })
}

/// Parse @mentions from a message, excluding code blocks and URLs.
fn parse_mentions(message: &str) -> Vec<String> {
    let mention_re = Regex::new(r"@([a-zA-Z0-9_\-\.]+)").expect("valid regex");
    let mut mentions = Vec::new();
    let mut in_code_block = false;

    for line in message.lines() {
        // Track fenced code blocks (```)
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Find mentions in this line, skipping inline code segments
        for mat in mention_re.find_iter(line) {
            let start = mat.start();
            let prefix = &line[..start];
            // Skip inline code: odd number of backticks before mention
            if prefix.matches('`').count() % 2 == 1 {
                continue;
            }
            // Skip URLs (http://... or https://...)
            if prefix.ends_with("http://") || prefix.ends_with("https://") {
                continue;
            }
            mentions.push(mat.as_str()[1..].to_string());
        }
    }

    mentions
}

#[cfg(test)]
mod tests {
    use super::truncate_preview;

    #[test]
    fn truncate_preview_keeps_valid_utf8_boundaries() {
        let input = "🙂".repeat(101);
        let truncated = truncate_preview(&input, 100);

        assert_eq!(truncated, format!("{}...", "🙂".repeat(100)));
    }
}
