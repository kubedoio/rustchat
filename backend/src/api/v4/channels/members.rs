use axum::{
    body::Bytes,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::utils::ensure_channel_admin_or_system_manage;
use super::utils::{fetch_channel_member_compat_rows, row_to_mm_channel_member};
use super::{mm, parse_mm_or_uuid, ApiResult, AppError, AppState, MmAuthUser};
use crate::repositories::ChannelRepository;
use serde_json::json;

#[derive(Deserialize)]
pub struct ChannelMemberPath {
    pub channel_id: String,
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct ChannelMemberIdsRequest {
    pub user_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct ChannelMemberRolesRequest {
    pub roles: String,
}

pub async fn get_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    let repo = ChannelRepository::new(&state.db);
    repo.require_member(channel_id, auth.user_id)
        .await
        .map_err(|_| crate::error::AppError::NotAMember)?;

    let rows = fetch_channel_member_compat_rows(&state, channel_id, None).await?;
    let mm_members = rows
        .into_iter()
        .map(|row| row_to_mm_channel_member(row, state.config.unread.post_priority_enabled))
        .collect();

    Ok(Json(mm_members))
}

pub async fn get_channel_member_me(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::ChannelMember>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;
    let rows = fetch_channel_member_compat_rows(&state, channel_id, Some(&[auth.user_id])).await?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| crate::error::AppError::NotAMember)?;

    Ok(Json(row_to_mm_channel_member(
        row,
        state.config.unread.post_priority_enabled,
    )))
}

pub async fn add_channel_member(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::ChannelMember>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    let input: AddMemberRequest = super::utils::parse_body(&headers, &body, "Invalid member body")?;

    let user_id =
        parse_mm_or_uuid(&input.user_id).ok_or_else(|| crate::error::AppError::InvalidUserId)?;

    let repo = ChannelRepository::new(&state.db);

    // Get channel to check type and verify caller can add members
    let channel = repo
        .get_by_id_optional(channel_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::ChannelNotFound)?;

    // For private channels, only channel admins or system admins may add members.
    // Public channels allow any member to add others.
    if channel.channel_type == crate::models::channel::ChannelType::Private {
        ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;
    } else {
        // For public/direct/group channels, verify caller is a member
        repo.require_member(channel_id, auth.user_id)
            .await
            .map_err(|_| crate::error::AppError::NotAMember)?;
    }

    // Add the user
    repo.add_member(channel_id, user_id, "member")
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let team_id = repo
        .get_team_id(channel_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::MemberAdded,
        serde_json::json!({
            "user_id": user_id,
            "channel_id": channel_id,
            "team_id": team_id,
        }),
        Some(channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(channel_id),
        team_id,
        user_id: Some(user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    let rows = fetch_channel_member_compat_rows(&state, channel_id, Some(&[user_id])).await?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| crate::error::AppError::MemberNotFound)?;

    Ok(Json(row_to_mm_channel_member(
        row,
        state.config.unread.post_priority_enabled,
    )))
}

/// DELETE /channels/{channel_id}/members/{user_id} - Remove a member from a channel
pub async fn remove_channel_member(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ChannelMemberPath>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    let user_id = if path.user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&path.user_id).ok_or_else(|| crate::error::AppError::InvalidUserId)?
    };

    remove_channel_member_by_id(state, auth, channel_id, user_id).await
}

/// DELETE /channels/{channel_id}/members/me - Leave a channel.
pub async fn remove_channel_member_me(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    let user_id = auth.user_id;
    remove_channel_member_by_id(state, auth, channel_id, user_id).await
}

async fn remove_channel_member_by_id(
    state: AppState,
    auth: MmAuthUser,
    channel_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> ApiResult<impl IntoResponse> {
    // Users may remove themselves; to remove others, channel or system admin is required
    if auth.user_id != user_id {
        ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;
    }

    let repo = ChannelRepository::new(&state.db);

    let team_id = repo
        .get_team_id(channel_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Remove the user
    repo.remove_member(channel_id, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Revoke WebSocket subscription so removed user stops receiving events
    state.ws_hub.unsubscribe_channel(user_id, channel_id).await;

    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::MemberRemoved,
        serde_json::json!({
            "user_id": user_id,
            "channel_id": channel_id,
            "team_id": team_id,
            "remover_id": auth.user_id,
        }),
        Some(channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(channel_id),
        team_id,
        user_id: Some(user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(json!({"status": "OK"})))
}

pub async fn get_channel_member_by_id(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ChannelMemberPath>,
) -> ApiResult<Json<mm::ChannelMember>> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::InvalidChannelId)?;
    let user_id = if path.user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&path.user_id).ok_or_else(|| crate::error::AppError::InvalidUserId)?
    };

    let repo = ChannelRepository::new(&state.db);
    repo.require_member(channel_id, auth.user_id)
        .await
        .map_err(|_| crate::error::AppError::NotAMember)?;

    let rows = fetch_channel_member_compat_rows(&state, channel_id, Some(&[user_id])).await?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| crate::error::AppError::MemberNotFound)?;

    Ok(Json(row_to_mm_channel_member(
        row,
        state.config.unread.post_priority_enabled,
    )))
}

pub async fn get_channel_members_by_ids(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    let repo = ChannelRepository::new(&state.db);
    repo.require_member(channel_id, auth.user_id)
        .await
        .map_err(|_| crate::error::AppError::NotAMember)?;

    let input: ChannelMemberIdsRequest =
        super::utils::parse_body(&headers, &body, "Invalid ids body")?;
    if input.user_ids.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let mut user_ids = Vec::new();
    for id in input.user_ids {
        let parsed = parse_mm_or_uuid(&id).ok_or_else(|| crate::error::AppError::InvalidUserId)?;
        user_ids.push(parsed);
    }

    let rows = fetch_channel_member_compat_rows(&state, channel_id, Some(&user_ids)).await?;
    let mm_members = rows
        .into_iter()
        .map(|row| row_to_mm_channel_member(row, state.config.unread.post_priority_enabled))
        .collect();

    Ok(Json(mm_members))
}

pub async fn update_channel_member_roles(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ChannelMemberPath>,
    Json(input): Json<ChannelMemberRolesRequest>,
) -> ApiResult<Json<mm::ChannelMember>> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::InvalidChannelId)?;
    let user_id = if path.user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&path.user_id).ok_or_else(|| crate::error::AppError::InvalidUserId)?
    };

    // Only channel admins or system admins may change member roles
    ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;

    let role = if input.roles.contains("channel_admin") {
        "channel_admin"
    } else {
        "member"
    };

    ChannelRepository::new(&state.db)
        .update_member_role(channel_id, user_id, role)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let rows = fetch_channel_member_compat_rows(&state, channel_id, Some(&[user_id])).await?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| crate::error::AppError::MemberNotFound)?;

    Ok(Json(row_to_mm_channel_member(
        row,
        state.config.unread.post_priority_enabled,
    )))
}

pub async fn update_channel_member_notify_props(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ChannelMemberPath>,
    Json(input): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::InvalidChannelId)?;
    let user_id = if path.user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&path.user_id).ok_or_else(|| crate::error::AppError::InvalidUserId)?
    };

    if user_id != auth.user_id {
        return Err(crate::error::AppError::Forbidden(
            "Cannot update another user's notify props".to_string(),
        ));
    }

    ChannelRepository::new(&state.db)
        .update_member_notify_props(channel_id, user_id, &input)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({"status": "OK"})))
}

/// PUT /channels/{channel_id}/members/{user_id}/schemeRoles - Update member scheme roles
#[derive(Deserialize)]
pub struct UpdateSchemeRolesRequest {
    #[serde(default)]
    pub scheme_admin: bool,
    #[serde(rename = "scheme_user", default)]
    pub _scheme_user: bool,
}

pub async fn update_channel_member_scheme_roles(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((channel_id, user_id)): Path<(String, String)>,
    Json(input): Json<UpdateSchemeRolesRequest>,
) -> ApiResult<impl IntoResponse> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;
    let target_user_id = if user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&user_id).ok_or_else(|| crate::error::AppError::InvalidUserId)?
    };

    // Only channel admins or system admins may change member scheme roles
    ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;

    // Update the target user's role based on scheme_admin
    let new_role = if input.scheme_admin {
        "admin"
    } else {
        "member"
    };

    ChannelRepository::new(&state.db)
        .update_member_role(channel_id, target_user_id, new_role)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({"status": "OK"})))
}
