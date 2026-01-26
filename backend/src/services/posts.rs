use std::collections::HashMap;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::models::{ChannelMember, CreatePost, FileUploadResponse, Post, PostResponse};
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};

/// Create a new post
pub async fn create_post(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    input: CreatePost,
    client_msg_id: Option<String>,
) -> ApiResult<PostResponse> {
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
        let root_post: Option<Post> =
            sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND channel_id = $2")
                .bind(r_id)
                .bind(channel_id)
                .fetch_optional(&state.db)
                .await?;

        if root_post.is_none() {
            return Err(AppError::BadRequest("Invalid root post".to_string()));
        }
    }

    // Insert post
    let post: Post = sqlx::query_as(
        r#"
        INSERT INTO posts (channel_id, user_id, root_post_id, message, props, file_ids)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(root_post_id)
    .bind(&input.message)
    .bind(input.props.unwrap_or(serde_json::json!({})))
    .bind(&input.file_ids)
    .fetch_one(&state.db)
    .await?;

    // If this is a reply, update the root post
    if let Some(r_id) = root_post_id {
        sqlx::query(
            "UPDATE posts SET reply_count = reply_count + 1, last_reply_at = NOW() WHERE id = $1",
        )
        .bind(r_id)
        .execute(&state.db)
        .await?;
    }

    // Fetch user details
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

    // Populate files if any
    if !response.file_ids.is_empty() {
        populate_files(state, std::slice::from_mut(&mut response)).await?;
    }

    // Broadcast new message
    let event_type = if root_post_id.is_some() {
        EventType::ThreadReplyCreated
    } else {
        EventType::MessageCreated
    };

    let broadcast = WsEnvelope::event(event_type, response.clone(), Some(channel_id))
        .with_broadcast(WsBroadcast {
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
                "last_reply_at": post.created_at
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

    // Check for playbook triggers
    if root_post_id.is_none() {
        let _ = check_playbook_triggers(state, channel_id, &response.message).await;
    }

    // Ensure DM membership for recipient if they left
    let _ = ensure_dm_membership(state, channel_id).await;

    // Increment unread counts in Redis for other members
    let _ = crate::services::unreads::increment_unreads(state, channel_id, user_id, post.seq).await;

    // Parse mentions (simple parsing for now)
    let mentions: Vec<String> = response
        .message
        .split_whitespace()
        .filter_map(|word| {
            if word.starts_with('@') && word.len() > 1 {
                Some(
                    word[1..]
                        .trim_matches(|c: char| !c.is_alphanumeric())
                        .to_string(),
                )
            } else {
                None
            }
        })
        .collect();

    if !mentions.is_empty() {
        // We could store these in the DB if we wanted persistent notifications
        // For now, we'll just include them in the broadcast if needed,
        // but the frontend already parses them from the message string.
        // Let's at least update the props to include mentions metadata.
        let mut props = response.props.as_object().cloned().unwrap_or_default();
        props.insert("mentions".to_string(), serde_json::json!(mentions));

        // Update DB with the new props
        sqlx::query("UPDATE posts SET props = $1 WHERE id = $2")
            .bind(serde_json::Value::Object(props.clone()))
            .bind(post.id)
            .execute(&state.db)
            .await
            .ok();

        response.props = serde_json::Value::Object(props);
    }

    Ok(response)
}

/// Helper to ensure all participants of a DM are members (resurrects DM)
pub async fn ensure_dm_membership(state: &AppState, channel_id: Uuid) -> ApiResult<()> {
    // 1. Get channel info
    let chan: crate::models::Channel = match sqlx::query_as("SELECT * FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(&state.db)
        .await?
    {
        Some(c) => c,
        None => return Ok(()),
    };

    if chan.channel_type == crate::models::ChannelType::Direct && chan.name.starts_with("dm_") {
        let parts: Vec<&str> = chan.name.split('_').collect();
        if parts.len() == 3 {
            let id1 = Uuid::parse_str(parts[1]).ok();
            let id2 = Uuid::parse_str(parts[2]).ok();

            if let (Some(u1), Some(u2)) = (id1, id2) {
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
    }
    Ok(())
}

/// Helper to populate files for posts
pub async fn populate_files(state: &AppState, posts: &mut [PostResponse]) -> ApiResult<()> {
    // 1. Collect all file IDs
    let all_file_ids: Vec<Uuid> = posts.iter().flat_map(|p| p.file_ids.clone()).collect();

    if all_file_ids.is_empty() {
        return Ok(());
    }

    // 2. Fetch file infos
    // crate::models::FileInfo is needed, ensure it is pub
    let files: Vec<crate::models::FileInfo> =
        sqlx::query_as("SELECT * FROM files WHERE id = ANY($1)")
            .bind(&all_file_ids)
            .fetch_all(&state.db)
            .await?;

    // 3. Generate presigned URLs and map to posts
    let mut file_map = HashMap::new();
    for file in files {
        let url = state
            .s3_client
            .presigned_download_url(&file.key, 3600)
            .await?;
        let thumbnail_url = if file.has_thumbnail {
            if let Some(t_key) = &file.thumbnail_key {
                state
                    .s3_client
                    .presigned_download_url(t_key, 3600)
                    .await
                    .ok()
            } else {
                None
            }
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

/// Create a system message in a channel
pub async fn create_system_message(
    state: &AppState,
    channel_id: Uuid,
    message: String,
    props: Option<serde_json::Value>,
) -> ApiResult<()> {
    // 1. Find bot user
    let bot_user = sqlx::query!("SELECT id FROM users WHERE is_bot = true LIMIT 1")
        .fetch_optional(&state.db)
        .await?
        .map(|r| r.id)
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
        RETURNING *
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
    let channel_info = sqlx::query!("SELECT team_id FROM channels WHERE id = $1", channel_id)
        .fetch_optional(&state.db)
        .await?;

    if let Some(chan) = channel_info {
        // 2. Fetch playbooks with triggers
        let playbooks = sqlx::query_as::<_, crate::models::Playbook>(
            "SELECT * FROM playbooks WHERE team_id = $1 AND is_archived = false AND keyword_triggers IS NOT NULL"
        )
        .bind(chan.team_id)
        .fetch_all(&state.db)
        .await?;

        // 3. Find bot user (optional)
        let bot_user = sqlx::query!("SELECT id FROM users WHERE is_bot = true LIMIT 1")
            .fetch_optional(&state.db)
            .await?
            .map(|r| r.id);

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
