use chrono::Utc;
use tracing::warn;
use uuid::Uuid;

use crate::api::AppState;
use crate::mattermost_compat::id::encode_mm_id;
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};

use super::state::CallState;

#[derive(sqlx::FromRow)]
struct CallThreadPostRow {
    id: Uuid,
    created_at: chrono::DateTime<Utc>,
    seq: i64,
}
async fn create_call_thread_post(
    state: &AppState,
    call_id: Uuid,
    channel_id: Uuid,
    owner_id: Uuid,
    started_at: i64,
) -> Result<Uuid, sqlx::Error> {
    let props = serde_json::json!({
        "type": "custom_calls",
        "call_id": encode_mm_id(call_id),
        "start_at": started_at,
        "end_at": 0,
        "participants": [encode_mm_id(owner_id)],
    });

    let post: CallThreadPostRow = sqlx::query_as(
        r#"
        INSERT INTO posts (channel_id, user_id, message, props, file_ids)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, created_at, seq
        "#,
    )
    .bind(channel_id)
    .bind(owner_id)
    .bind("")
    .bind(&props)
    .bind(Vec::<Uuid>::new())
    .fetch_one(&state.db)
    .await?;

    let mm_post = crate::mattermost_compat::models::Post {
        id: encode_mm_id(post.id),
        create_at: post.created_at.timestamp_millis(),
        update_at: post.created_at.timestamp_millis(),
        delete_at: 0,
        edit_at: 0,
        user_id: encode_mm_id(owner_id),
        channel_id: encode_mm_id(channel_id),
        root_id: String::new(),
        original_id: String::new(),
        message: String::new(),
        post_type: "custom_calls".to_string(),
        props,
        hashtags: String::new(),
        file_ids: Vec::new(),
        pending_post_id: String::new(),
        metadata: None,
        is_bot: false,
    };

    let broadcast = WsEnvelope::event(EventType::MessageCreated, mm_post, Some(channel_id))
        .with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });
    state.ws_hub.broadcast(broadcast).await;

    let _ =
        crate::services::unreads::increment_unreads(state, channel_id, owner_id, post.seq).await;

    Ok(post.id)
}
pub(crate) async fn mark_call_thread_post_ended(
    state: &AppState,
    thread_id: Uuid,
    ended_at: i64,
) -> Result<Option<crate::models::post::PostResponse>, sqlx::Error> {
    let mut post = sqlx::query_as(
        r#"
        WITH updated_post AS (
            UPDATE posts
            SET
                props = jsonb_set(
                    COALESCE(props, '{}'::jsonb),
                    '{end_at}',
                    to_jsonb($1::bigint),
                    true
                ),
                edited_at = NOW()
            WHERE id = $2
            RETURNING *
        )
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count, p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email, COALESCE(u.is_bot, false) as is_bot
        FROM updated_post p
        LEFT JOIN users u ON p.user_id = u.id
        "#,
    )
    .bind(ended_at)
    .bind(thread_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(post) = &mut post {
        crate::services::posts::normalize_post_avatar_urls(std::slice::from_mut(post));
    }

    Ok(post)
}
pub(crate) async fn ensure_call_thread_id(state: &AppState, call: &CallState) -> Option<Uuid> {
    if let Some(thread_id) = call.thread_id {
        return Some(thread_id);
    }

    match create_call_thread_post(
        state,
        call.call_id,
        call.channel_id,
        call.owner_id,
        call.started_at,
    )
    .await
    {
        Ok(thread_id) => {
            state
                .call_state_manager
                .set_thread_id(call.call_id, Some(thread_id))
                .await;
            Some(thread_id)
        }
        Err(err) => {
            warn!(
                call_id = %call.call_id,
                channel_id = %call.channel_id,
                error = %err,
                "calls failed to create call thread post"
            );
            None
        }
    }
}
