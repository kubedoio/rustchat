use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;

use super::{encode_mm_id, parse_mm_or_uuid, ApiResult, AppState, MmAuthUser};
use crate::repositories::{ChannelRepository, PostRepository, UserRepository};

/// GET /channels/{channel_id}/unread - Get unread counts for a channel
pub async fn get_channel_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    let channel_repo = ChannelRepository::new(&state.db);
    let post_repo = PostRepository::new(state.db.clone());
    let user_repo = UserRepository::new(&state.db);

    channel_repo
        .is_channel_member(channel_id, auth.user_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .then_some(())
        .ok_or_else(|| {
            crate::error::AppError::Forbidden("Not a member of this channel".to_string())
        })?;

    let team_id = channel_repo
        .get_team_id(channel_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .ok_or_else(|| crate::error::AppError::NotFound("Channel not found".to_string()))?;

    let username = user_repo
        .get_username(auth.user_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .unwrap_or_default();

    let last_read_message_id = post_repo
        .get_channel_read(auth.user_id, channel_id)
        .await?
        .unwrap_or(0);

    let (msg_count, mention_count, mention_count_root, urgent_mention_count, msg_count_root) =
        post_repo
            .compute_channel_unread_counts(
                channel_id,
                last_read_message_id,
                &username,
                state.config.unread.post_priority_enabled,
            )
            .await?;

    Ok(Json(serde_json::json!({
        "team_id": encode_mm_id(team_id),
        "channel_id": encode_mm_id(channel_id),
        "msg_count": msg_count,
        "mention_count": mention_count,
        "mention_count_root": mention_count_root,
        "msg_count_root": msg_count_root,
        "urgent_mention_count": urgent_mention_count
    })))
}

/// POST /channels/{channel_id}/members/{user_id}/read - Mark channel as read
pub async fn mark_channel_as_read(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    let target_user_id = if user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&user_id)
            .ok_or_else(|| crate::error::AppError::BadRequest("Invalid user_id".to_string()))?
    };

    // Users can only mark their own channels as read
    if target_user_id != auth.user_id {
        return Err(crate::error::AppError::Forbidden(
            "Cannot mark channel as read for other users".to_string(),
        ));
    }

    let channel_repo = ChannelRepository::new(&state.db);

    // Verify membership
    channel_repo
        .is_channel_member(channel_id, auth.user_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .then_some(())
        .ok_or_else(|| {
            crate::error::AppError::Forbidden("Not a member of this channel".to_string())
        })?;

    channel_repo
        .mark_channel_read(auth.user_id, channel_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    // Broadcast channel viewed event
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::ChannelViewed,
        serde_json::json!({
            "channel_id": encode_mm_id(channel_id),
        }),
        Some(channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: None,
        team_id: None,
        user_id: Some(auth.user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(json!({"status": "OK"})))
}

/// POST /channels/{channel_id}/members/{user_id}/set_unread - Mark channel as unread
/// This sets the last_viewed_at to a past time to create unread state
pub async fn mark_channel_as_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    let target_user_id = if user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&user_id)
            .ok_or_else(|| crate::error::AppError::BadRequest("Invalid user_id".to_string()))?
    };

    // Users can only mark their own channels as unread
    if target_user_id != auth.user_id {
        return Err(crate::error::AppError::Forbidden(
            "Cannot mark channel as unread for other users".to_string(),
        ));
    }

    let channel_repo = ChannelRepository::new(&state.db);
    let post_repo = PostRepository::new(state.db.clone());

    // Verify membership
    channel_repo
        .is_channel_member(channel_id, auth.user_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .then_some(())
        .ok_or_else(|| {
            crate::error::AppError::Forbidden("Not a member of this channel".to_string())
        })?;

    // Get the oldest post in the channel to set as unread point
    let oldest_post_time = post_repo
        .get_oldest_post_time(channel_id)
        .await?;

    // Set last_viewed_at to the oldest post time, or epoch if no posts
    let mark_time = oldest_post_time.unwrap_or(chrono::DateTime::UNIX_EPOCH);

    channel_repo
        .mark_channel_unread(auth.user_id, channel_id, mark_time)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    // Also update channel_reads table
    post_repo
        .reset_channel_read(auth.user_id, channel_id, mark_time)
        .await?;

    // Broadcast unread update
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::ChannelUnread,
        serde_json::json!({
            "channel_id": encode_mm_id(channel_id),
            "user_id": encode_mm_id(auth.user_id),
            "unread_count": 1,
        }),
        Some(channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: None,
        team_id: None,
        user_id: Some(auth.user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(json!({
        "channel_id": encode_mm_id(channel_id),
        "user_id": encode_mm_id(auth.user_id),
        "status": "OK"
    })))
}
