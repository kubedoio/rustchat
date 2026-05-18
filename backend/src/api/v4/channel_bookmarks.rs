//! Channel Bookmarks API
//!
//! Handles bookmarks in channels (links and files).

use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::Utc;
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use crate::models::channel_bookmark::{
    BookmarkResponse, ChannelBookmark, CreateBookmarkRequest, ReorderBookmarkRequest,
    UpdateBookmarkRequest,
};
use crate::realtime::{WsBroadcast, WsEnvelope};
use crate::repositories::{BookmarkRepository, ChannelRepository};

/// Build channel bookmarks routes
pub fn router() -> Router<AppState> {
    Router::new()
        // GET /api/v4/channels/{channel_id}/bookmarks
        .route(
            "/channels/{channel_id}/bookmarks",
            get(get_channel_bookmarks).post(create_bookmark),
        )
        // PATCH /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}
        // DELETE /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}
        .route(
            "/channels/{channel_id}/bookmarks/{bookmark_id}",
            patch(update_bookmark).delete(delete_bookmark),
        )
        // POST /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}/sort_order
        .route(
            "/channels/{channel_id}/bookmarks/{bookmark_id}/sort_order",
            post(reorder_bookmark),
        )
}

/// Get all bookmarks for a channel
/// GET /api/v4/channels/{channel_id}/bookmarks
async fn get_channel_bookmarks(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<Vec<BookmarkResponse>>> {
    let channel_uuid = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;

    let repo = ChannelRepository::new(&state.db);
    let is_member = repo.is_channel_member(channel_uuid, auth.user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    let bookmark_repo = BookmarkRepository::new(&state.db);
    let bookmarks = bookmark_repo.list_for_channel(channel_uuid).await?;

    let responses: Vec<BookmarkResponse> = bookmarks.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

/// Create a new bookmark
/// POST /api/v4/channels/{channel_id}/bookmarks
async fn create_bookmark(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(body): Json<CreateBookmarkRequest>,
) -> ApiResult<Json<BookmarkResponse>> {
    let channel_uuid = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;

    let repo = ChannelRepository::new(&state.db);
    let is_member = repo.is_channel_member(channel_uuid, auth.user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    // Validate bookmark type
    if body.r#type != "link" && body.r#type != "file" {
        return Err(AppError::Validation(
            "Bookmark type must be 'link' or 'file'".to_string(),
        ));
    }

    // Validate based on type
    if body.r#type == "link" && body.link_url.is_none() {
        return Err(AppError::Validation(
            "Link bookmarks require link_url".to_string(),
        ));
    }

    if body.r#type == "file" && body.file_id.is_none() {
        return Err(AppError::Validation(
            "File bookmarks require file_id".to_string(),
        ));
    }

    let file_uuid = body.file_id.as_ref().and_then(|id| parse_mm_or_uuid(id));

    let now = Utc::now().timestamp_millis();
    let sort_order = body.sort_order.unwrap_or(0);

    let bookmark_repo = BookmarkRepository::new(&state.db);
    let bookmark = bookmark_repo.create(
        channel_uuid,
        auth.user_id,
        &body.r#type,
        body.display_name.as_deref(),
        body.link_url.as_deref(),
        file_uuid,
        body.emoji.as_deref(),
        sort_order,
        body.image_url.as_deref(),
        now,
    ).await?;

    // Broadcast event
    broadcast_bookmark_event(&state, "channel_bookmark_created", &bookmark).await;

    Ok(Json(bookmark.into()))
}

/// Update a bookmark
/// PATCH /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}
async fn update_bookmark(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, bookmark_id)): Path<(String, String)>,
    Json(body): Json<UpdateBookmarkRequest>,
) -> ApiResult<Json<BookmarkResponse>> {
    let channel_uuid = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;
    let bookmark_uuid = parse_mm_or_uuid(&bookmark_id)
        .ok_or_else(|| AppError::BadRequest("Invalid bookmark ID".to_string()))?;

    let bookmark_repo = BookmarkRepository::new(&state.db);
    let owner_id = bookmark_repo.get_owner_id(bookmark_uuid, channel_uuid).await?
        .ok_or_else(|| AppError::NotFound("Bookmark not found".to_string()))?;

    if owner_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only update your own bookmarks".to_string(),
        ));
    }

    let now = Utc::now().timestamp_millis();

    let bookmark = bookmark_repo.update(
        bookmark_uuid,
        channel_uuid,
        body.display_name.as_deref(),
        body.link_url.as_deref(),
        body.emoji.as_deref(),
        body.sort_order,
        body.image_url.as_deref(),
        now,
    ).await?;

    // Broadcast event
    broadcast_bookmark_event(&state, "channel_bookmark_updated", &bookmark).await;

    Ok(Json(bookmark.into()))
}

/// Delete a bookmark
/// DELETE /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}
async fn delete_bookmark(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, bookmark_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let channel_uuid = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;
    let bookmark_uuid = parse_mm_or_uuid(&bookmark_id)
        .ok_or_else(|| AppError::BadRequest("Invalid bookmark ID".to_string()))?;

    let bookmark_repo = BookmarkRepository::new(&state.db);
    let bookmark = bookmark_repo.get(bookmark_uuid, channel_uuid).await?
        .ok_or_else(|| AppError::NotFound("Bookmark not found".to_string()))?;

    if bookmark.owner_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only delete your own bookmarks".to_string(),
        ));
    }

    let now = Utc::now().timestamp_millis();

    bookmark_repo.soft_delete(bookmark_uuid, channel_uuid, now).await?;

    // Broadcast event
    broadcast_bookmark_event(&state, "channel_bookmark_deleted", &bookmark).await;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// Reorder a bookmark
/// POST /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}/sort_order
async fn reorder_bookmark(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, bookmark_id)): Path<(String, String)>,
    Json(body): Json<ReorderBookmarkRequest>,
) -> ApiResult<Json<BookmarkResponse>> {
    let channel_uuid = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;
    let bookmark_uuid = parse_mm_or_uuid(&bookmark_id)
        .ok_or_else(|| AppError::BadRequest("Invalid bookmark ID".to_string()))?;

    let bookmark_repo = BookmarkRepository::new(&state.db);
    let bookmark = bookmark_repo.get(bookmark_uuid, channel_uuid).await?
        .ok_or_else(|| AppError::NotFound("Bookmark not found".to_string()))?;

    if bookmark.owner_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only reorder your own bookmarks".to_string(),
        ));
    }

    let now = Utc::now().timestamp_millis();

    let bookmark = bookmark_repo.reorder(bookmark_uuid, channel_uuid, body.sort_order, now).await?;

    // Broadcast event
    broadcast_bookmark_event(&state, "channel_bookmark_sorted", &bookmark).await;

    Ok(Json(bookmark.into()))
}

/// Broadcast bookmark event via WebSocket
async fn broadcast_bookmark_event(state: &AppState, event_type: &str, bookmark: &ChannelBookmark) {
    let event = WsEnvelope {
        msg_type: "event".to_string(),
        event: event_type.to_string(),
        seq: None,
        channel_id: Some(bookmark.channel_id),
        data: serde_json::json!({
            "bookmark": {
                "id": encode_mm_id(bookmark.id),
                "channel_id": encode_mm_id(bookmark.channel_id),
                "owner_id": encode_mm_id(bookmark.owner_id),
                "type": bookmark.r#type,
                "display_name": bookmark.display_name,
                "link_url": bookmark.link_url,
                "file_id": bookmark.file_id.map(encode_mm_id),
                "emoji": bookmark.emoji,
                "sort_order": bookmark.sort_order,
                "image_url": bookmark.image_url,
                "create_at": bookmark.create_at,
                "update_at": bookmark.update_at,
            }
        }),
        broadcast: Some(WsBroadcast {
            user_id: Some(bookmark.owner_id),
            channel_id: Some(bookmark.channel_id),
            team_id: None,
            exclude_user_id: None,
        }),
    };

    state.ws_hub.broadcast(event).await;
}
