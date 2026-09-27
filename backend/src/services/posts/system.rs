//! System message creation (e.g. join/leave notices posted by the system bot).

use uuid::Uuid;

use crate::api::AppState;
use crate::error::ApiResult;
use crate::models::PostResponse;
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};
use crate::repositories::{ChannelRepository, PostRepository, UserRepository};

/// Create a system message in a channel
pub async fn create_system_message(
    state: &AppState,
    channel_id: Uuid,
    message: String,
    props: Option<serde_json::Value>,
) -> ApiResult<()> {
    // 1. Find bot user (create one if none exists)
    let bot_user = match UserRepository::new(&state.db).get_bot_user_id().await? {
        Some(id) => id,
        None => {
            crate::repositories::IntegrationRepository::new(&state.db)
                .create_bot_user("system", "system@rustchat.local")
                .await?
        }
    };

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

    // 3. Start tx, insert post, update author's reads
    let mut tx = state.db.begin().await?;
    let post = PostRepository::new(state.db.clone())
        .create_system_message_post_in_tx(&mut tx, channel_id, bot_user, &message, final_props)
        .await?;
    crate::services::unreads::update_author_channel_read_in_tx(
        &mut tx, channel_id, bot_user, post.seq,
    )
    .await?;
    tx.commit().await?;

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
        is_bot: false,
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

    // 6. Increment unread counts for other members (post-commit)
    let team_id = ChannelRepository::new(&state.db)
        .get_team_id(channel_id)
        .await
        .ok()
        .flatten();
    if let Some(team_id) = team_id {
        let members: Vec<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM channel_members WHERE channel_id = $1")
                .bind(channel_id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
        let _ = crate::services::unreads::increment_unreads_external(
            state, channel_id, bot_user, post.seq, team_id, members, &message,
        )
        .await;
    }

    Ok(())
}
