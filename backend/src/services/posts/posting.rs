//! Post creation: validation, transactional persistence, websocket fan-out,
//! automation and push notifications, plus playbook trigger handling.

use crate::mattermost_compat::models as mm;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::models::{
    normalize_avatar_url, Activity, ActivityType, ChannelMember, CreatePost, Post, PostResponse,
};
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};
use crate::repositories::{
    ChannelRepository, FileRepository, PlaybookRepository, PostRepository, UserRepository,
};
use crate::services::activity;
use crate::services::posts::parse_mentions;
use crate::services::posts::query::populate_files;

/// Returns true when the database error is the unique violation on the
/// partial unique index for client_msg_id.
fn is_client_msg_id_conflict(err: &sqlx::Error) -> bool {
    if let Some(db_err) = err.as_database_error() {
        db_err.code().as_deref() == Some("23505")
            && db_err.constraint() == Some("idx_posts_client_msg_id_unique")
    } else {
        false
    }
}

async fn validate_create_post(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    input: &CreatePost,
) -> ApiResult<Option<Uuid>> {
    ensure_permission(state, user_id, "post.create").await?;

    // Check membership
    let _: ChannelMember = PostRepository::new(state.db.clone())
        .require_channel_membership(channel_id, user_id)
        .await?;

    // Validate message
    if input.message.trim().is_empty() && input.file_ids.is_empty() {
        return Err(AppError::Validation("Message cannot be empty".to_string()));
    }

    // Validate root_post_id if provided
    let root_post_id = input.root_post_id;
    if let Some(r_id) = root_post_id {
        let root_post = PostRepository::new(state.db.clone())
            .get_post_by_id_and_channel(r_id, channel_id)
            .await?;

        if root_post.is_none() {
            return Err(AppError::BadRequest("Invalid root post".to_string()));
        }
    }

    // Max message length
    let max_len = state.config.messaging.max_message_length;
    if input.message.chars().count() > max_len {
        return Err(AppError::Validation(format!(
            "Message exceeds maximum length of {} characters",
            max_len
        )));
    }

    // Max file count
    let max_files = state.config.messaging.max_file_count;
    if input.file_ids.len() > max_files {
        return Err(AppError::Validation(format!(
            "Cannot attach more than {} files",
            max_files
        )));
    }

    Ok(root_post_id)
}

async fn build_post_response(
    state: &AppState,
    post: Post,
    user_id: Uuid,
    client_msg_id: Option<String>,
) -> ApiResult<PostResponse> {
    let (username, avatar_url, email, is_bot) = UserRepository::new(&state.db)
        .get_username_avatar_email(user_id)
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
        username: Some(username),
        avatar_url,
        email: Some(email),
        is_bot,
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
        if let Ok(Some(team_id)) = ChannelRepository::new(&state.db)
            .get_team_id(channel_id)
            .await
        {
            let channel_name = ChannelRepository::new(&state.db)
                .get_name(channel_id)
                .await
                .ok()
                .flatten()
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

async fn send_push_notifications(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    post_id: Uuid,
    response: &PostResponse,
    mentions: &[String],
    username_for_push: String,
) {
    // Get channel info for push notifications
    let channel_info = ChannelRepository::new(&state.db)
        .get_channel_push_info(channel_id)
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
            ChannelRepository::new(&state.db)
                .get_other_dm_participant(channel_id, user_id)
                .await
                .unwrap_or_default()
        } else if !mentions.is_empty() {
            // For mentions, find the mentioned users who are channel members
            let usernames = mentions.iter().map(|m| m.as_str()).collect::<Vec<_>>();
            ChannelRepository::new(&state.db)
                .get_member_ids_by_usernames(channel_id, &usernames)
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
                    post_id,
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

/// Create a new post with all DB side effects committed atomically.
pub async fn create_post(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    input: CreatePost,
    client_msg_id: Option<String>,
) -> ApiResult<PostResponse> {
    let root_post_id = validate_create_post(state, user_id, channel_id, &input).await?;

    // Validate and normalize the persisted client_msg_id (covers v4 pending_post_id path).
    let client_msg_id = client_msg_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(id) = client_msg_id {
        if id.len() > 64 {
            return Err(AppError::Validation(
                "client_msg_id must be at most 64 characters".to_string(),
            ));
        }
    }

    // Pre-compute data needed before the transaction.
    let team_id_opt = ChannelRepository::new(&state.db)
        .get_team_id(channel_id)
        .await
        .ok()
        .flatten();
    let mentions = parse_mentions(&input.message);
    let mention_user_ids: Vec<Uuid> = if !mentions.is_empty() && team_id_opt.is_some() {
        UserRepository::new(&state.db)
            .get_ids_by_usernames(&mentions.iter().map(|m| m.as_str()).collect::<Vec<_>>())
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| *id != user_id)
            .collect()
    } else {
        vec![]
    };

    // Pre-resolve DM users (if any) so we don't need channel type lookup inside tx.
    let dm_users = if let Some(chan) = ChannelRepository::new(&state.db)
        .get_by_id_optional(channel_id)
        .await
        .ok()
        .flatten()
    {
        if chan.channel_type == crate::models::ChannelType::Direct {
            crate::models::parse_direct_channel_name(&chan.name)
        } else {
            None
        }
    } else {
        None
    };

    // Validate file attachments before starting tx (read-only)
    if !input.file_ids.is_empty() {
        let files = FileRepository::new(&state.db)
            .get_by_ids(&input.file_ids)
            .await?;

        // Verify all file IDs exist
        if files.len() != input.file_ids.len() {
            return Err(AppError::Validation(
                "One or more attached files do not exist".to_string(),
            ));
        }

        for file in &files {
            // Verify uploader is the post author
            if file.uploader_id != user_id {
                return Err(AppError::Forbidden(
                    "Cannot attach a file uploaded by another user".to_string(),
                ));
            }
            // Verify file belongs to the target channel (or is unassociated)
            if let Some(file_channel_id) = file.channel_id {
                if file_channel_id != channel_id {
                    return Err(AppError::Forbidden(
                        "File does not belong to this channel".to_string(),
                    ));
                }
            }
            // Verify file is not already attached to another post
            if file.post_id.is_some() {
                return Err(AppError::Validation(
                    "File is already attached to another post".to_string(),
                ));
            }
        }
    }

    // Optional Buzz integration: resolve the author label up front (read-only)
    // so the outbox enqueue inside the transaction stays cheap. When the
    // integration is disabled this is a single boolean check — no queries.
    let buzz_author_label = if state.config.integrations.buzz.enabled {
        UserRepository::new(&state.db)
            .get_by_id(user_id)
            .await
            .ok()
            .flatten()
            .map(|u| {
                u.display_name
                    .filter(|d| !d.trim().is_empty())
                    .unwrap_or(u.username)
            })
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        String::new()
    };

    // ========================================================================
    // SERVICE-LEVEL TRANSACTION: all DB side effects inside, external after.
    // ========================================================================
    let mut tx = state.db.begin().await?;

    // 1. Insert post + reply count. If a concurrent request already inserted
    // the same client_msg_id, the unique index raises 23505; return that post.
    let post = match PostRepository::new(state.db.clone())
        .create_post_in_tx(
            &mut tx,
            channel_id,
            user_id,
            root_post_id,
            &input.message,
            input.props.clone().unwrap_or(serde_json::json!({})),
            &input.file_ids,
            client_msg_id,
        )
        .await
    {
        Ok(post) => post,
        Err(err) => {
            if let AppError::Database(ref sqlx_err) = err {
                if is_client_msg_id_conflict(sqlx_err) {
                    drop(tx);
                    if let Some(id) = client_msg_id {
                        if let Some(existing_id) = PostRepository::new(state.db.clone())
                            .find_post_by_client_msg_id(user_id, id)
                            .await?
                        {
                            let existing_post = PostRepository::new(state.db.clone())
                                .get_post_by_id(existing_id)
                                .await?
                                .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

                            // Verify this is a true idempotent retry, not a reuse of
                            // client_msg_id for a different post.
                            let same_channel = existing_post.channel_id == channel_id;
                            let same_root = existing_post.root_post_id == root_post_id;
                            let same_message = existing_post.message == input.message;
                            let mut existing_file_ids = existing_post.file_ids.clone();
                            existing_file_ids.sort();
                            let mut request_file_ids = input.file_ids.clone();
                            request_file_ids.sort();
                            let same_files = existing_file_ids == request_file_ids;
                            if !same_channel || !same_root || !same_message || !same_files {
                                return Err(AppError::Conflict(
                                    "client_msg_id reused with different post content".to_string(),
                                ));
                            }

                            return build_post_response(
                                state,
                                existing_post,
                                user_id,
                                client_msg_id.map(|s| s.to_string()),
                            )
                            .await;
                        }
                    }
                }
            }
            return Err(err);
        }
    };

    // Link files to this post inside the transaction (with ownership re-validation)
    if !input.file_ids.is_empty() {
        let attached_ids = FileRepository::new(&state.db)
            .attach_files_to_post_in_tx(&mut tx, &input.file_ids, post.id, channel_id, user_id)
            .await?;

        if attached_ids.len() != input.file_ids.len() {
            return Err(AppError::Validation(
                "One or more files could not be attached".to_string(),
            ));
        }
    }

    // 2. Update props with mentions inside tx
    let mut props = post.props.as_object().cloned().unwrap_or_default();
    if !mentions.is_empty() {
        props.insert("mentions".to_string(), serde_json::json!(&mentions));
        PostRepository::new(state.db.clone())
            .update_props_in_tx(&mut tx, post.id, serde_json::Value::Object(props.clone()))
            .await?;
    }

    // 3. Reply activities inside tx
    let mut reply_activities: Vec<Activity> = Vec::new();
    if let Some(r_id) = root_post_id {
        if let Some((parent_user_id, parent_root_id)) = PostRepository::new(state.db.clone())
            .get_parent_info_in_tx(&mut tx, r_id)
            .await
            .ok()
            .flatten()
        {
            if parent_user_id != user_id {
                if let Some(team_id) = team_id_opt {
                    let activity_type = if parent_root_id.is_some() {
                        ActivityType::ThreadReply
                    } else {
                        ActivityType::Reply
                    };
                    let activity = activity::create_activity_in_tx(
                        &mut tx,
                        parent_user_id,
                        activity_type,
                        user_id,
                        channel_id,
                        team_id,
                        post.id,
                        Some(r_id),
                        Some(input.message.clone()),
                        None,
                    )
                    .await?;
                    reply_activities.push(activity);
                }
            }
        }
    }

    // 4. Mention activities inside tx
    let mut mention_activities: Vec<Activity> = Vec::new();
    if let Some(team_id) = team_id_opt {
        let member_ids = ChannelRepository::new(&state.db)
            .get_member_ids_by_user_ids_in_tx(&mut tx, channel_id, &mention_user_ids)
            .await
            .unwrap_or_default();
        for mentioned_user_id in member_ids {
            let activity = activity::create_activity_in_tx(
                &mut tx,
                mentioned_user_id,
                ActivityType::Mention,
                user_id,
                channel_id,
                team_id,
                post.id,
                root_post_id,
                Some(input.message.clone()),
                None,
            )
            .await?;
            mention_activities.push(activity);
        }
    }

    // 5. DM membership repair inside tx
    let mut dm_added_users: Vec<Uuid> = Vec::new();
    if let Some((u1, u2)) = dm_users {
        for target_user_id in [u1, u2] {
            if let Ok(added) = ChannelRepository::new(&state.db)
                .ensure_membership_in_tx(&mut tx, channel_id, target_user_id)
                .await
            {
                if added.is_some() {
                    dm_added_users.push(target_user_id);
                }
            }
        }
    }

    // 6. Author's read position inside tx
    crate::services::unreads::update_author_channel_read_in_tx(
        &mut tx, channel_id, user_id, post.seq,
    )
    .await?;

    // 7. Integration outbox (transactional): if this channel is bridged to a
    // Buzz channel, enqueue the outbound event in the same transaction so
    // the message and the delivery intent commit (or roll back) atomically.
    // A single boolean check when the integration is disabled.
    //
    // The integration is OPTIONAL and isolated: a failure to enqueue must
    // never fail or roll back the RustChat post itself. Because any error in
    // Postgres aborts the enclosing transaction, we guard the enqueue with a
    // SAVEPOINT and, on failure, roll back only the enqueue's work — leaving
    // the outer post transaction valid. The message is still created in
    // RustChat; only the outbound bridge intent for that message is dropped,
    // with an operator-visible warning.
    if state.config.integrations.buzz.enabled {
        let _ = sqlx::query("SAVEPOINT rustchat_buzz_enqueue")
            .execute(&mut *tx)
            .await;
        match crate::integrations::buzz::dispatcher::enqueue_message_created_in_tx(
            &mut tx,
            channel_id,
            post.id,
            post.root_post_id,
            &buzz_author_label,
            &input.message,
        )
        .await
        {
            Ok(()) => {
                let _ = sqlx::query("RELEASE SAVEPOINT rustchat_buzz_enqueue")
                    .execute(&mut *tx)
                    .await;
            }
            Err(e) => {
                tracing::warn!(
                    post_id = %post.id,
                    error = %e,
                    "Buzz integration enqueue failed; RustChat message created without a bridge intent"
                );
                let _ = sqlx::query("ROLLBACK TO SAVEPOINT rustchat_buzz_enqueue")
                    .execute(&mut *tx)
                    .await;
                let _ = sqlx::query("RELEASE SAVEPOINT rustchat_buzz_enqueue")
                    .execute(&mut *tx)
                    .await;
            }
        }
    }

    // Commit
    tx.commit().await?;
    // ========================================================================
    // POST-COMMIT: best-effort external effects only.
    // ========================================================================

    let mut response =
        build_post_response(state, post, user_id, client_msg_id.map(|s| s.to_string())).await?;
    response.props = serde_json::Value::Object(props);

    // Broadcast post over WS
    broadcast_new_post(state, channel_id, &response, root_post_id).await;

    // Broadcast reply activities over WS (full ActivityResponse shape)
    for activity in &reply_activities {
        activity::broadcast_activity(state, activity).await;
    }

    // Broadcast mention activities over WS (full ActivityResponse shape)
    for activity in &mention_activities {
        activity::broadcast_activity(state, activity).await;
    }

    // Automation (best-effort)
    run_post_automation(state, channel_id, user_id, &response, root_post_id).await;

    // Agent triggers (best-effort)
    if let Some(runtime) = state.agent_runtime.clone() {
        let post_clone = response.clone();
        let channel_id_clone = channel_id;
        tokio::spawn(async move {
            if let Err(e) = runtime
                .handle_post_created(&post_clone, channel_id_clone)
                .await
            {
                tracing::error!(error = %e, "Agent runtime failed");
            }
        });
    }

    // DM membership WS broadcast
    if !dm_added_users.is_empty() {
        if let Ok(Some(chan)) = ChannelRepository::new(&state.db)
            .get_by_id_optional(channel_id)
            .await
        {
            for target_user_id in dm_added_users {
                let event =
                    WsEnvelope::event(EventType::ChannelCreated, chan.clone(), Some(channel_id))
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

    // Redis unread + WS unread broadcast
    let members = ChannelRepository::new(&state.db)
        .get_all_member_ids(channel_id)
        .await
        .unwrap_or_default();
    if let Some(team_id) = team_id_opt {
        let _ = crate::services::unreads::increment_unreads_external(
            state,
            channel_id,
            user_id,
            response.seq,
            team_id,
            members,
            &input.message,
        )
        .await;
    }

    // Push notifications
    let username_for_push = response.username.clone().unwrap_or_default();
    send_push_notifications(
        state,
        channel_id,
        user_id,
        response.id,
        &response,
        &mentions,
        username_for_push,
    )
    .await;

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
    let role = UserRepository::new(&state.db)
        .get_role_by_id(user_id)
        .await?;

    let allowed = UserRepository::new(&state.db)
        .has_permission(&role, permission)
        .await?;

    if !allowed {
        return Err(AppError::InsufficientPermissions);
    }

    Ok(())
}

async fn check_playbook_triggers(
    state: &AppState,
    channel_id: Uuid,
    message: &str,
) -> ApiResult<()> {
    // 1. Get team_id
    let channel_info = ChannelRepository::new(&state.db)
        .get_team_id(channel_id)
        .await?;

    if let Some(chan) = channel_info {
        // 2. Fetch playbooks with triggers
        let playbooks = PlaybookRepository::new(&state.db)
            .get_playbooks_with_triggers(chan)
            .await?;

        // 3. Find bot user (optional)
        let bot_user = UserRepository::new(&state.db).get_bot_user_id().await?;

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
                        let _ = PostRepository::new(state.db.clone())
                            .insert_post_no_returning(
                                channel_id,
                                bot_user.unwrap_or_else(Uuid::nil),
                                &system_msg,
                                serde_json::json!({
                                    "type": "system_playbook_trigger",
                                    "override_username": "Playbook Bot",
                                    "playbook_id": playbook.id
                                }),
                            )
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
