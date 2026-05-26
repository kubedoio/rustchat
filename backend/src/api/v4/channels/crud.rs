use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::utils::ensure_channel_admin_or_system_manage;
use super::utils::resolve_direct_channel_display_name;
use super::{
    mm, parse_mm_or_uuid, permissions, ApiResult, AppError, AppState, Channel, MmAuthUser,
};
use crate::api::v4::channels::utils::map_channel_with_team_data_row;
use crate::models::ChannelType;
use crate::realtime::events::{EventType, WsBroadcast, WsEnvelope};
use crate::repositories::{ChannelRepository, UserRepository};
use crate::services::posts::create_system_message;
use serde_json::json;

#[derive(Debug, Deserialize, Default)]
pub struct GetAllChannelsQuery {
    #[serde(default)]
    pub not_associated_to_group: Option<String>,
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub per_page: Option<u64>,
    #[serde(default)]
    pub exclude_default_channels: bool,
    #[serde(default)]
    pub include_deleted: bool,
    #[serde(default)]
    pub include_total_count: bool,
}

pub async fn get_all_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Query(query): Query<GetAllChannelsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !auth.has_permission(&permissions::SYSTEM_MANAGE)
        && !auth.has_permission(&permissions::ADMIN_FULL)
    {
        return Err(AppError::Forbidden(
            "Insufficient permissions to list all channels".to_string(),
        ));
    }

    let not_associated_group_id = query
        .not_associated_to_group
        .as_deref()
        .filter(|raw| !raw.trim().is_empty())
        .map(|raw| {
            parse_mm_or_uuid(raw)
                .ok_or_else(|| AppError::BadRequest("Invalid not_associated_to_group".to_string()))
        })
        .transpose()?;

    let page = query.page.unwrap_or(0);
    let mut per_page = query.per_page.unwrap_or(60);
    if per_page == 0 {
        per_page = 60;
    }
    per_page = per_page.min(10_000);
    let offset = page.saturating_mul(per_page) as i64;

    let repo = ChannelRepository::new(&state.db);
    let rows = repo
        .get_all_channels(
            query.include_deleted,
            query.exclude_default_channels,
            not_associated_group_id,
            per_page as i64,
            offset,
        )
        .await?;

    let channels = rows
        .into_iter()
        .map(map_channel_with_team_data_row)
        .collect::<Vec<_>>();

    if query.include_total_count {
        let total_count = repo
            .count_all_channels(
                query.include_deleted,
                query.exclude_default_channels,
                not_associated_group_id,
            )
            .await?;

        return Ok(Json(json!({
            "channels": channels,
            "total_count": total_count
        })));
    }

    Ok(Json(json!(channels)))
}

pub async fn get_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    let repo = ChannelRepository::new(&state.db);

    // Verify membership
    let _membership = repo.require_member(channel_id, auth.user_id).await?;

    let mut channel: crate::models::Channel = repo.get_by_id(channel_id).await?;

    // For Direct channels, ALWAYS compute display_name from the other participant
    // This ensures each user sees the other person's name, not their own
    if channel.channel_type == crate::models::channel::ChannelType::Direct {
        channel.display_name =
            resolve_direct_channel_display_name(&state, channel.id, auth.user_id)
                .await?
                .or_else(|| Some("Direct Message".to_string()));
    }

    Ok(Json(channel.into()))
}

/// POST /channels - Create a new channel
#[derive(serde::Deserialize)]
pub struct CreateChannelRequest {
    pub team_id: String,
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type", default = "default_channel_type")]
    pub channel_type: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub header: String,
}

fn default_channel_type() -> String {
    "O".to_string()
}

pub async fn create_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    let input: CreateChannelRequest =
        super::utils::parse_body(&headers, &body, "Invalid channel body")?;

    let team_id =
        parse_mm_or_uuid(&input.team_id).ok_or_else(|| crate::error::AppError::InvalidTeamId)?;

    let repo = ChannelRepository::new(&state.db);

    // Verify team membership
    let is_member = repo.is_team_member(team_id, auth.user_id).await?;
    if !is_member {
        return Err(crate::error::AppError::NotOnTeam);
    }

    // Map MM channel type to RustChat type
    let channel_type = match input.channel_type.as_str() {
        "O" => "public",
        "P" => "private",
        _ => "public",
    };

    let channel: Channel = repo
        .create(
            team_id,
            channel_type,
            &input.name,
            &input.display_name,
            &input.purpose,
            &input.header,
            auth.user_id,
        )
        .await?;

    // Add creator as member
    repo.add_member(channel.id, auth.user_id, "admin").await?;

    Ok(Json(channel.into()))
}

/// PUT /channels/{channel_id} - Update channel
#[derive(serde::Deserialize)]
pub struct UpdateChannelRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub header: Option<String>,
}

pub async fn update_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;

    let input: UpdateChannelRequest =
        super::utils::parse_body(&headers, &body, "Invalid channel update")?;

    let repo = ChannelRepository::new(&state.db);
    let channel: Channel = repo
        .update(
            channel_id,
            input.name.as_deref(),
            input.display_name.as_deref(),
            input.purpose.as_deref(),
            input.header.as_deref(),
        )
        .await?;

    // Broadcast ChannelUpdated event
    let broadcast = WsBroadcast {
        channel_id: Some(channel_id),
        team_id: Some(channel.team_id),
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    };
    let event = WsEnvelope::event(EventType::ChannelUpdated, &channel, Some(channel_id))
        .with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;

    Ok(Json(channel.into()))
}

/// DELETE /channels/{channel_id} - Delete/archive channel
pub async fn delete_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    // Only channel admins or system admins may delete a channel
    ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;

    let repo = ChannelRepository::new(&state.db);

    // Get channel info for the broadcast
    let channel = repo
        .get_by_id_optional(channel_id)
        .await?
        .ok_or_else(|| AppError::ChannelNotFound)?;

    if channel.deleted_at.is_some() {
        return Err(AppError::ChannelAlreadyArchived);
    }

    // Block archiving default channels
    if channel.name == "town-square" || channel.name == "off-topic" {
        return Err(AppError::BadRequest(
            "Cannot archive default channels".to_string(),
        ));
    }

    // Block archiving direct and group messages
    if channel.channel_type == ChannelType::Direct || channel.channel_type == ChannelType::Group {
        return Err(AppError::BadRequest(
            "Cannot archive direct or group message channels".to_string(),
        ));
    }

    // Soft delete the channel
    let _ = repo.soft_delete(channel_id).await?;

    // Post system message
    let username = UserRepository::new(&state.db)
        .get_username(auth.user_id)
        .await?
        .unwrap_or_else(|| "System".to_string());
    let _ = create_system_message(
        &state,
        channel_id,
        format!("{username} archived the channel."),
        Some(serde_json::json!({"type": "system_channel_archived"})),
    )
    .await;

    // Broadcast ChannelDeleted event
    let broadcast = WsBroadcast {
        channel_id: Some(channel_id),
        team_id: Some(channel.team_id),
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    };
    let event = WsEnvelope::event(
        EventType::ChannelDeleted,
        serde_json::json!({
            "channel_id": channel_id.to_string(),
            "team_id": channel.team_id.to_string(),
        }),
        Some(channel_id),
    )
    .with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// PUT /channels/{channel_id}/patch - Patch channel (partial update)
#[derive(serde::Deserialize)]
pub struct PatchChannelRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub header: Option<String>,
}

pub async fn patch_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(input): Json<PatchChannelRequest>,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;

    let repo = ChannelRepository::new(&state.db);
    let channel: Channel = repo
        .update(
            channel_id,
            input.name.as_deref(),
            input.display_name.as_deref(),
            input.purpose.as_deref(),
            input.header.as_deref(),
        )
        .await?;

    // Broadcast ChannelUpdated event
    let broadcast = WsBroadcast {
        channel_id: Some(channel_id),
        team_id: Some(channel.team_id),
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    };
    let event = WsEnvelope::event(EventType::ChannelUpdated, &channel, Some(channel_id))
        .with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;

    Ok(Json(channel.into()))
}

/// PUT /channels/{channel_id}/privacy - Update channel privacy
#[derive(serde::Deserialize)]
pub struct UpdatePrivacyRequest {
    pub privacy: String, // "O" for public, "P" for private
}

pub async fn update_channel_privacy(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(input): Json<UpdatePrivacyRequest>,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    // Only channel admins or system admins may change channel privacy
    ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;

    let channel_type = match input.privacy.as_str() {
        "O" => "public",
        "P" => "private",
        _ => {
            return Err(crate::error::AppError::BadRequest(
                "Invalid privacy value".to_string(),
            ))
        }
    };

    let repo = ChannelRepository::new(&state.db);
    let channel: Channel = repo.update_privacy(channel_id, channel_type).await?;

    Ok(Json(channel.into()))
}

/// POST /channels/{channel_id}/restore - Restore a deleted channel
pub async fn restore_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;

    // Only channel admins or system admins may restore a deleted channel
    ensure_channel_admin_or_system_manage(&state, channel_id, &auth).await?;

    let repo = ChannelRepository::new(&state.db);
    let channel: Channel = repo.restore(channel_id).await?;

    // Post system message
    let username = UserRepository::new(&state.db)
        .get_username(auth.user_id)
        .await?
        .unwrap_or_else(|| "System".to_string());
    let _ = create_system_message(
        &state,
        channel_id,
        format!("{username} unarchived the channel."),
        Some(serde_json::json!({"type": "system_channel_restored"})),
    )
    .await;

    // Broadcast ChannelRestored event
    let mm_channel: mm::Channel = channel.clone().into();
    let broadcast = WsBroadcast {
        channel_id: Some(channel_id),
        team_id: Some(channel.team_id),
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    };
    let event = WsEnvelope::event(EventType::ChannelRestored, &mm_channel, Some(channel_id))
        .with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;

    Ok(Json(channel.into()))
}

/// POST /channels/{channel_id}/move - Move channel to another team
#[derive(serde::Deserialize)]
pub struct MoveChannelRequest {
    pub team_id: String,
    #[serde(rename = "force", default)]
    pub _force: bool,
}

pub async fn move_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(input): Json<MoveChannelRequest>,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;
    let new_team_id =
        parse_mm_or_uuid(&input.team_id).ok_or_else(|| crate::error::AppError::InvalidTeamId)?;

    // Only system admins may move a channel between teams
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "System admin privileges required to move channels between teams".to_string(),
        ));
    }

    let repo = ChannelRepository::new(&state.db);

    // Verify membership in new team
    let is_team_member = repo.is_team_member(new_team_id, auth.user_id).await?;
    if !is_team_member {
        return Err(crate::error::AppError::Forbidden(
            "Not a member of the target team".to_string(),
        ));
    }

    let channel: Channel = repo.move_to_team(channel_id, new_team_id).await?;

    Ok(Json(channel.into()))
}
