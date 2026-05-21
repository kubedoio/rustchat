#![allow(dead_code)]

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use super::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use crate::repositories::{BookmarkRow, ChannelGroupRow, ChannelRepository};

/// Bookmark with optional file info for API responses
#[derive(serde::Serialize)]
pub struct ChannelBookmarkResponse {
    id: String,
    create_at: i64,
    update_at: i64,
    delete_at: i64,
    channel_id: String,
    owner_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_id: Option<String>,
    display_name: String,
    sort_order: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    link_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emoji: Option<String>,
    #[serde(rename = "type")]
    bookmark_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<serde_json::Value>,
}

impl From<BookmarkRow> for ChannelBookmarkResponse {
    fn from(row: BookmarkRow) -> Self {
        Self {
            id: encode_mm_id(row.id),
            create_at: row.created_at.timestamp_millis(),
            update_at: row.updated_at.timestamp_millis(),
            delete_at: row.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            channel_id: encode_mm_id(row.channel_id),
            owner_id: encode_mm_id(row.owner_id),
            file_id: row.file_id.map(encode_mm_id),
            display_name: row.display_name,
            sort_order: row.sort_order,
            link_url: row.link_url,
            image_url: row.image_url,
            emoji: row.emoji,
            bookmark_type: row.bookmark_type,
            original_id: row.original_id.map(encode_mm_id),
            parent_id: row.parent_id.map(encode_mm_id),
            file: None, // Note: File attachment data not currently loaded; join with files table if needed in future
        }
    }
}

#[derive(Deserialize)]
pub struct BookmarksQuery {
    bookmarks_since: Option<i64>,
}

/// GET /api/v4/channels/{channel_id}/bookmarks
pub(super) async fn get_channel_bookmarks(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Query(query): Query<BookmarksQuery>,
) -> ApiResult<Json<Vec<ChannelBookmarkResponse>>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::InvalidChannelId)?;

    let repo = ChannelRepository::new(&state.db);

    // Verify channel membership
    let is_member = repo.is_channel_member(channel_id, auth.user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    let since = query.bookmarks_since.unwrap_or(0);
    let bookmarks = repo.list_channel_bookmarks(channel_id, since).await?;

    Ok(Json(bookmarks.into_iter().map(Into::into).collect()))
}

#[derive(Deserialize)]
pub struct CreateBookmarkRequest {
    display_name: String,
    #[serde(rename = "type")]
    bookmark_type: String,
    link_url: Option<String>,
    image_url: Option<String>,
    emoji: Option<String>,
    file_id: Option<String>,
}

/// POST /api/v4/channels/{channel_id}/bookmarks
pub(super) async fn create_channel_bookmark(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    _headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<ChannelBookmarkResponse>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::InvalidChannelId)?;

    let repo = ChannelRepository::new(&state.db);

    // Verify channel membership
    let is_member = repo.is_channel_member(channel_id, auth.user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    let req: CreateBookmarkRequest = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("Invalid bookmark body".to_string()))?;

    // Validate bookmark type
    if req.bookmark_type != "link" && req.bookmark_type != "file" {
        return Err(AppError::BadRequest(
            "Type must be 'link' or 'file'".to_string(),
        ));
    }

    // Validate link URL for link type
    if req.bookmark_type == "link" && req.link_url.is_none() {
        return Err(AppError::BadRequest(
            "Link URL required for link bookmarks".to_string(),
        ));
    }

    let file_id = req.file_id.as_ref().and_then(|id| parse_mm_or_uuid(id));

    // Get max sort order for this channel
    let max_order = repo.get_max_bookmark_sort_order(channel_id).await?;
    let sort_order = max_order.unwrap_or(0) + 1;
    let now = Utc::now();

    let bookmark = repo
        .create_channel_bookmark(
            channel_id,
            auth.user_id,
            file_id,
            &req.display_name,
            sort_order,
            req.link_url.as_deref(),
            req.image_url.as_deref(),
            req.emoji.as_deref(),
            &req.bookmark_type,
            now,
        )
        .await?;

    Ok(Json(bookmark.into()))
}

#[derive(Deserialize)]
pub struct PatchBookmarkRequest {
    display_name: Option<String>,
    link_url: Option<String>,
    image_url: Option<String>,
    emoji: Option<String>,
    file_id: Option<String>,
    sort_order: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct UpdateBookmarkResponse {
    updated: ChannelBookmarkResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted: Option<ChannelBookmarkResponse>,
}

/// PATCH /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}
pub(super) async fn patch_channel_bookmark(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, bookmark_id)): Path<(String, String)>,
    _headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<UpdateBookmarkResponse>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::InvalidChannelId)?;
    let bookmark_id = parse_mm_or_uuid(&bookmark_id)
        .ok_or_else(|| AppError::InvalidBookmarkId)?;

    let repo = ChannelRepository::new(&state.db);

    // Verify channel membership
    let is_member = repo.is_channel_member(channel_id, auth.user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    let req: PatchBookmarkRequest = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("Invalid patch body".to_string()))?;

    let file_id = req.file_id.as_ref().and_then(|id| parse_mm_or_uuid(id));

    let bookmark = repo
        .update_channel_bookmark(
            bookmark_id,
            channel_id,
            req.display_name.as_deref(),
            req.link_url.as_deref(),
            req.image_url.as_deref(),
            req.emoji.as_deref(),
            file_id,
            req.sort_order,
        )
        .await?
        .ok_or_else(|| AppError::BookmarkNotFound)?;

    Ok(Json(UpdateBookmarkResponse {
        updated: bookmark.into(),
        deleted: None,
    }))
}

/// POST /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}/sort_order
pub(super) async fn update_channel_bookmark_sort_order(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, bookmark_id)): Path<(String, String)>,
    _headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<ChannelBookmarkResponse>>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::InvalidChannelId)?;
    let bookmark_id = parse_mm_or_uuid(&bookmark_id)
        .ok_or_else(|| AppError::InvalidBookmarkId)?;

    let repo = ChannelRepository::new(&state.db);

    // Verify channel membership
    let is_member = repo.is_channel_member(channel_id, auth.user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    let new_order: i64 = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("Invalid sort order".to_string()))?;

    repo.update_bookmark_sort_order(bookmark_id, channel_id, new_order)
        .await?;

    // Return all bookmarks for this channel
    let bookmarks = repo.list_channel_bookmarks(channel_id, 0).await?;

    Ok(Json(bookmarks.into_iter().map(Into::into).collect()))
}

/// DELETE /api/v4/channels/{channel_id}/bookmarks/{bookmark_id}
pub(super) async fn delete_channel_bookmark(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, bookmark_id)): Path<(String, String)>,
) -> ApiResult<Json<ChannelBookmarkResponse>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::InvalidChannelId)?;
    let bookmark_id = parse_mm_or_uuid(&bookmark_id)
        .ok_or_else(|| AppError::InvalidBookmarkId)?;

    let repo = ChannelRepository::new(&state.db);

    // Verify channel membership
    let is_member = repo.is_channel_member(channel_id, auth.user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    // Soft delete
    let bookmark = repo
        .soft_delete_channel_bookmark(bookmark_id, channel_id)
        .await?
        .ok_or_else(|| AppError::BookmarkNotFound)?;

    Ok(Json(bookmark.into()))
}

/// POST /api/v4/channels/group/search
pub(super) async fn search_group_channels(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Json(_query): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// PUT /api/v4/channels/{channel_id}/scheme
pub(super) async fn update_channel_scheme(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_channel_id): Path<String>,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /api/v4/channels/{channel_id}/members_minus_group_members
pub(super) async fn get_channel_members_minus_group_members(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/channels/{channel_id}/member_counts_by_group
pub(super) async fn get_channel_member_counts_by_group(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/channels/{channel_id}/moderations
pub(super) async fn get_channel_moderations(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// PUT /api/v4/channels/{channel_id}/moderations/patch
pub(super) async fn patch_channel_moderations(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_channel_id): Path<String>,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/channels/{channel_id}/common_teams
pub(super) async fn get_channel_common_teams(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/channels/{channel_id}/groups
pub(super) async fn get_channel_groups(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Query(query): Query<GroupAssociationQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::InvalidChannelId)?;

    enforce_channel_group_read_permission(&state, &auth, channel_id).await?;

    let search_term = query.q.clone().unwrap_or_default().to_ascii_lowercase();
    let filter_allow_reference = should_filter_allow_reference(&auth, &query);
    let (paginate, offset, per_page) = pagination_from_group_query(&query);

    let repo = ChannelRepository::new(&state.db);
    let rows = repo
        .list_channel_groups(channel_id, filter_allow_reference, &search_term)
        .await?;

    let total_group_count = rows.len();
    let paged_rows = if paginate {
        rows.into_iter()
            .skip(offset)
            .take(per_page)
            .collect::<Vec<_>>()
    } else {
        rows
    };

    Ok(Json(serde_json::json!({
        "groups": paged_rows.iter().map(channel_group_json).collect::<Vec<_>>(),
        "total_group_count": total_group_count
    })))
}

/// GET /api/v4/channels/{channel_id}/access_control/attributes
pub(super) async fn get_channel_access_control_attributes(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct GroupAssociationQuery {
    q: Option<String>,
    include_member_count: Option<bool>,
    filter_allow_reference: Option<bool>,
    page: Option<i64>,
    per_page: Option<i64>,
    paginate: Option<bool>,
}

async fn enforce_channel_group_read_permission(
    state: &AppState,
    auth: &MmAuthUser,
    channel_id: Uuid,
) -> ApiResult<()> {
    if auth.has_permission(&permissions::SYSTEM_MANAGE)
        || auth.has_permission(&permissions::ADMIN_FULL)
    {
        return Ok(());
    }

    let repo = ChannelRepository::new(&state.db);

    let (team_id, channel_type) = repo
        .get_channel_type_and_team(channel_id)
        .await?
        .ok_or_else(|| AppError::ChannelNotFound)?;

    let is_channel_member = repo.is_channel_member(channel_id, auth.user_id).await?;

    if channel_type == "private" {
        if !is_channel_member {
            return Err(AppError::Forbidden(
                "Missing permission to view groups for this private channel".to_string(),
            ));
        }
        return Ok(());
    }

    if channel_type == "public" {
        if let Some(team_id) = team_id {
            let is_team_member = repo.is_team_member(team_id, auth.user_id).await?;
            if !is_team_member {
                return Err(AppError::Forbidden(
                    "Missing permission to view groups for this public channel".to_string(),
                ));
            }
            return Ok(());
        }
    }

    if !is_channel_member {
        return Err(AppError::Forbidden(
            "Missing permission to view groups for this channel".to_string(),
        ));
    }

    Ok(())
}

fn should_filter_allow_reference(auth: &MmAuthUser, query: &GroupAssociationQuery) -> bool {
    let has_system_group_read = auth.has_permission(&permissions::SYSTEM_MANAGE)
        || auth.has_permission(&permissions::ADMIN_FULL);

    query.filter_allow_reference.unwrap_or(false) || !has_system_group_read
}

fn pagination_from_group_query(query: &GroupAssociationQuery) -> (bool, usize, usize) {
    let _ = query.include_member_count.unwrap_or(false);
    let paginate = query.paginate.unwrap_or(true);
    let page = query.page.unwrap_or(0).max(0) as usize;
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE) as usize;
    let offset = page.saturating_mul(per_page);
    (paginate, offset, per_page)
}

fn channel_group_json(row: &ChannelGroupRow) -> serde_json::Value {
    serde_json::json!({
        "id": encode_mm_id(row.id),
        "name": row.name,
        "display_name": row.display_name,
        "description": row.description,
        "source": row.source,
        "remote_id": row.remote_id,
        "allow_reference": row.allow_reference,
        "create_at": row.created_at.timestamp_millis(),
        "update_at": row.updated_at.timestamp_millis(),
        "delete_at": row.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
        "has_syncables": row.has_syncables,
        "member_count": row.member_count,
        "scheme_admin": row.scheme_admin,
    })
}
