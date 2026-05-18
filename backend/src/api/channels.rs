//! Channels API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::auth::policy::permissions;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    normalize_avatar_url, Channel, ChannelMember, ChannelType, CreateChannel, UpdateChannel,
};
use crate::realtime::events::{EventType, WsBroadcast, WsEnvelope};
use crate::repositories::{AdminRepository, ChannelRepository, UserRepository};

/// Check if user is channel creator, admin, or has system manage permission
async fn is_channel_creator_or_admin(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
) -> ApiResult<bool> {
    // Check system manage permission first
    let has_system_manage = AdminRepository::new(&state.db)
        .has_system_manage_permission(user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if has_system_manage {
        return Ok(true);
    }

    // Check if user is channel creator
    let creator_id: Option<Uuid> = ChannelRepository::new(&state.db).get_creator_id(channel_id).await?;

    if creator_id == Some(user_id) {
        return Ok(true);
    }

    // Check if user is channel admin
    let role: Option<String> = ChannelRepository::new(&state.db).get_member_role(channel_id, user_id).await?;

    let is_admin = matches!(
        role.as_deref(),
        Some("admin") | Some("channel_admin") | Some("team_admin")
    );

    Ok(is_admin)
}

/// Build channels routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_channels).post(create_channel))
        .route("/unreads", get(get_all_unread_counts))
        .route(
            "/{id}",
            get(get_channel).put(update_channel).delete(delete_channel),
        )
        .route("/{id}/members", get(list_members).post(add_member))
        .route("/{id}/members/{user_id}", delete(remove_member))
        .route("/{id}/read", post(mark_channel_as_read))
        // Mattermost-compatible endpoints that frontend expects
        .route(
            "/{id}/members/{user_id}/read",
            post(mark_channel_member_as_read),
        )
        .route(
            "/{id}/members/{user_id}/notify_props",
            put(update_notify_props),
        )
        .route("/{id}/stats", get(get_channel_stats))
}

/// Get unread counts for all channels the user is a member of
async fn get_all_unread_counts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<crate::services::unreads::ChannelUnreadOverview>>> {
    let overview = crate::services::unreads::get_unread_overview(&state, auth.user_id).await?;
    Ok(Json(overview.channels))
}

#[derive(Debug, Deserialize)]
pub struct MarkReadRequest {
    pub target_seq: Option<i64>,
}

/// Mark a channel as read
async fn mark_channel_as_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<MarkReadRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    crate::services::unreads::mark_channel_as_read(&state, auth.user_id, id, input.target_seq)
        .await?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

#[derive(Debug, Deserialize)]
pub struct ListChannelsQuery {
    pub team_id: Uuid,
    pub include_archived: Option<bool>,
    pub available_to_join: Option<bool>,
}

async fn hydrate_direct_channel_display_name(
    state: &AppState,
    viewer_id: Uuid,
    channel: &mut Channel,
) -> ApiResult<()> {
    // For Direct channels, ALWAYS compute display_name from the other participant
    // This ensures each user sees the other person's name, not their own
    if channel.channel_type != ChannelType::Direct {
        return Ok(());
    }

    let display_name = ChannelRepository::new(&state.db)
        .get_dm_display_name(channel.id, viewer_id)
        .await?;

    channel.display_name = display_name.or_else(|| Some("Direct Message".to_string()));
    Ok(())
}

/// List channels in a team
async fn list_channels(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListChannelsQuery>,
) -> ApiResult<Json<Vec<Channel>>> {
    let include_archived = query.include_archived.unwrap_or(false);
    let available_to_join = query.available_to_join.unwrap_or(false);

    if available_to_join {
        // First check if user is a member of the team
        if !ChannelRepository::new(&state.db).is_team_member(query.team_id, auth.user_id).await? {
            return Err(AppError::Forbidden("Not a member of this team".to_string()));
        }

        // List public and private channels user is NOT in
        let channels: Vec<Channel> = ChannelRepository::new(&state.db)
            .list_joinable(query.team_id, auth.user_id)
            .await?;

        return Ok(Json(channels));
    }

    // Default behavior: List channels user is a member of
    let mut channels: Vec<Channel> = ChannelRepository::new(&state.db)
        .list_for_user(query.team_id, auth.user_id, include_archived)
        .await?;

    for channel in &mut channels {
        hydrate_direct_channel_display_name(&state, auth.user_id, channel).await?;
    }

    Ok(Json(channels))
}

/// Create a new channel
async fn create_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateChannel>,
) -> ApiResult<Json<Channel>> {
    // Special handling for Direct Messages
    if input.channel_type == crate::models::ChannelType::Direct {
        let target_id = input.target_user_id.ok_or_else(|| {
            AppError::Validation("target_user_id is required for direct messages".to_string())
        })?;

        // Deterministic name: sorted user IDs
        let mut ids = vec![auth.user_id, target_id];
        ids.sort();
        let dm_name = crate::models::canonical_direct_channel_name(ids[0], ids[1]);
        let legacy_dm_name = crate::models::legacy_direct_channel_name(ids[0], ids[1]);

        // Check if DM channel already exists in this team
        let existing = ChannelRepository::new(&state.db)
            .find_dm_channel(input.team_id, &dm_name, &legacy_dm_name)
            .await?;

        if let Some(mut channel) = existing {
            // Re-add both users as members just in case they left (resurrect DM)
            let _ = crate::services::posts::ensure_dm_membership(&state, channel.id).await;
            hydrate_direct_channel_display_name(&state, auth.user_id, &mut channel).await?;
            return Ok(Json(channel));
        }

        // Validate target user exists in the team
        if !ChannelRepository::new(&state.db).is_team_member(input.team_id, target_id).await? {
            return Err(AppError::Forbidden(
                "Target user is not a member of this team".to_string(),
            ));
        }

        let teammate_display_name = UserRepository::new(&state.db)
            .get_display_name_or_username(target_id)
            .await?;
        let display_name = input
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or(teammate_display_name)
            .unwrap_or_else(|| "Direct Message".to_string());

        // Create DM channel
        let channel = ChannelRepository::new(&state.db)
            .create(
                input.team_id,
                &input.channel_type.to_string(),
                &dm_name,
                &display_name,
                &input.purpose.unwrap_or_default(),
                "",
                auth.user_id,
            )
            .await?;

        // Add both users as members
        let repo = ChannelRepository::new(&state.db);
        for user_id in ids {
            repo.add_member(channel.id, user_id, "member").await?;

            // Broadcast event to each user individually
            let event =
                WsEnvelope::event(EventType::ChannelCreated, channel.clone(), Some(channel.id))
                    .with_broadcast(WsBroadcast {
                        user_id: Some(user_id),
                        channel_id: None,
                        team_id: None,
                        exclude_user_id: None,
                    });
            state.ws_hub.broadcast(event).await;
        }

        return Ok(Json(channel));
    }

    // Standard channel creation (Public/Private)
    if input.name.len() < 2 {
        return Err(AppError::Validation(
            "Channel name must be at least 2 characters".to_string(),
        ));
    }

    if !auth.has_permission(&permissions::CHANNEL_CREATE)
        && !auth.has_permission(&permissions::CHANNEL_MANAGE)
    {
        return Err(AppError::Forbidden(
            "Missing permission to create channels".to_string(),
        ));
    }

    // Check if team exists and user is member
    if !ChannelRepository::new(&state.db).is_team_member(input.team_id, auth.user_id).await? {
        return Err(AppError::Forbidden("Not a member of this team".to_string()));
    }

    // Create channel
    let channel = ChannelRepository::new(&state.db)
        .create(
            input.team_id,
            &input.channel_type.to_string(),
            &input.name,
            &input.display_name.unwrap_or_default(),
            &input.purpose.unwrap_or_default(),
            "",
            auth.user_id,
        )
        .await?;

    // Add creator as admin member
    ChannelRepository::new(&state.db)
        .add_member(channel.id, auth.user_id, "admin")
        .await?;

    // Broadcast event
    let broadcast = if channel.channel_type == crate::models::ChannelType::Public {
        // Broadcast to entire team
        WsBroadcast {
            team_id: Some(input.team_id),
            channel_id: None,
            user_id: None,
            exclude_user_id: None,
        }
    } else {
        // Private channel: broadcast only to creator (for now)
        WsBroadcast {
            user_id: Some(auth.user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        }
    };

    let event = WsEnvelope::event(EventType::ChannelCreated, channel.clone(), Some(channel.id))
        .with_broadcast(broadcast);

    state.ws_hub.broadcast(event).await;

    Ok(Json(channel))
}

/// Get a specific channel
async fn get_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Channel>> {
    // Check membership
    let _member = ChannelRepository::new(&state.db)
        .require_member(id, auth.user_id)
        .await?;

    let mut channel = ChannelRepository::new(&state.db)
        .get_by_id(id)
        .await?;

    hydrate_direct_channel_display_name(&state, auth.user_id, &mut channel).await?;

    Ok(Json(channel))
}

/// Update a channel
async fn update_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateChannel>,
) -> ApiResult<Json<Channel>> {
    // Check if user is creator or admin
    let can_update = is_channel_creator_or_admin(&state, id, auth.user_id).await?;

    if !can_update {
        return Err(AppError::Forbidden(
            "Only channel creator or admin can update this channel".to_string(),
        ));
    }

    // Validate name if provided (must be at least 2 chars and lowercase/no spaces)
    if let Some(ref name) = input.name {
        if name.len() < 2 {
            return Err(AppError::Validation(
                "Channel name must be at least 2 characters".to_string(),
            ));
        }
        // Check name doesn't contain spaces
        if name.contains(' ') {
            return Err(AppError::Validation(
                "Channel name cannot contain spaces".to_string(),
            ));
        }
    }

    // Update channel using a single query with COALESCE for optional fields
    let channel: Channel = ChannelRepository::new(&state.db)
        .update(
            id,
            input.name.as_deref(),
            input.display_name.as_deref(),
            input.purpose.as_deref(),
            input.header.as_deref(),
        )
        .await?;

    // Broadcast ChannelUpdated event
    let broadcast = WsBroadcast {
        channel_id: Some(id),
        team_id: Some(channel.team_id),
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    };
    let event =
        WsEnvelope::event(EventType::ChannelUpdated, &channel, Some(id)).with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;

    Ok(Json(channel))
}

/// Delete a channel (soft delete)
async fn delete_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check if user is creator or admin
    let can_delete = is_channel_creator_or_admin(&state, id, auth.user_id).await?;

    if !can_delete {
        return Err(AppError::Forbidden(
            "Only channel creator or admin can delete this channel".to_string(),
        ));
    }

    // Get channel info for the broadcast
    let channel = ChannelRepository::new(&state.db)
        .get_by_id(id)
        .await
        .map_err(|_| AppError::NotFound("Channel not found".to_string()))?;

    // Soft delete the channel
    ChannelRepository::new(&state.db).soft_delete(id).await?;

    // Broadcast ChannelDeleted event
    let broadcast = WsBroadcast {
        channel_id: Some(id),
        team_id: Some(channel.team_id),
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    };
    let event = WsEnvelope::event(
        EventType::ChannelDeleted,
        serde_json::json!({
            "channel_id": id.to_string(),
            "team_id": channel.team_id.to_string(),
        }),
        Some(id),
    )
    .with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// List channel members
async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<ChannelMember>>> {
    let repo = ChannelRepository::new(&state.db);
    let _ = repo.require_member(id, auth.user_id).await?;
    let mut members: Vec<ChannelMember> = repo.list_members(id).await?;

    for member in &mut members {
        member.avatar_url = normalize_avatar_url(member.user_id, member.avatar_url.as_deref());
    }

    Ok(Json(members))
}

/// Add a member to channel
async fn add_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<AddMemberRequest>,
) -> ApiResult<Json<ChannelMember>> {
    // Check permissions
    if auth.user_id == input.user_id {
        // User joining themselves
        let channel: Channel = ChannelRepository::new(&state.db).get_by_id(id).await?;

        if channel.channel_type != crate::models::ChannelType::Public {
            let member = ChannelRepository::new(&state.db)
                .require_member(id, auth.user_id)
                .await?;

            if member.role != "admin" && !auth.has_permission(&permissions::CHANNEL_MANAGE) {
                return Err(AppError::Forbidden(
                    "Cannot join private channel without invite".to_string(),
                ));
            }
        }
        // If public, allow proceed
    } else {
        // Adding someone else - require admin
        let member = ChannelRepository::new(&state.db)
            .require_member(id, auth.user_id)
            .await?;

        if member.role != "admin" && !auth.has_permission(&permissions::CHANNEL_MANAGE) {
            return Err(AppError::Forbidden(
                "Not an admin of this channel".to_string(),
            ));
        }
    }

    let new_member: ChannelMember = ChannelRepository::new(&state.db)
        .upsert_member(id, input.user_id, input.role.as_deref().unwrap_or("member"))
        .await?;

    // Announce join in public channels
    let channel = ChannelRepository::new(&state.db)
        .get_by_id(id)
        .await?;

    if channel.channel_type == crate::models::ChannelType::Public {
        let username = UserRepository::new(&state.db)
            .get_username(input.user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let _ = crate::services::posts::create_system_message(
            &state,
            id,
            format!("@{} has joined the channel.", username),
            None,
        )
        .await;
    }

    Ok(Json(new_member))
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: Option<String>,
}

/// Remove a member from channel
async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check admin membership (or user removing themselves)
    if auth.user_id != user_id {
        let member = ChannelRepository::new(&state.db)
            .require_member(channel_id, auth.user_id)
            .await?;

        if member.role != "admin" && !auth.has_permission(&permissions::CHANNEL_MANAGE) {
            return Err(AppError::Forbidden(
                "Not an admin of this channel".to_string(),
            ));
        }
    }

    ChannelRepository::new(&state.db)
        .remove_member(channel_id, user_id)
        .await?;

    Ok(Json(serde_json::json!({"status": "removed"})))
}

/// Mark a channel as read for a specific user (Mattermost-compatible endpoint)
/// POST /channels/{id}/members/{user_id}/read
async fn mark_channel_member_as_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, user_id)): Path<(Uuid, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    // Handle "me" as current user
    let target_user_id = if user_id == "me" {
        auth.user_id
    } else {
        user_id
            .parse::<Uuid>()
            .map_err(|_| AppError::BadRequest("Invalid user_id".to_string()))?
    };

    // Users can only mark their own channels as read
    if target_user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot mark channel as read for other users".to_string(),
        ));
    }

    // Verify membership
    let _membership = ChannelRepository::new(&state.db)
        .require_member(channel_id, auth.user_id)
        .await?;

    // Update last_viewed_at to mark all messages as read
    let repo = ChannelRepository::new(&state.db);
    repo.mark_channel_read(channel_id, auth.user_id).await?;

    // Also update channel_reads table
    repo.update_channel_reads_to_latest(auth.user_id, channel_id).await?;

    // Broadcast channel viewed event
    let broadcast = WsEnvelope::event(
        EventType::ChannelViewed,
        serde_json::json!({
            "channel_id": channel_id.to_string(),
        }),
        Some(channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: None,
        team_id: None,
        user_id: Some(auth.user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// Update channel member notification properties
/// PUT /channels/{id}/members/{user_id}/notify_props
async fn update_notify_props(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, user_id)): Path<(Uuid, String)>,
    Json(input): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    // Handle "me" as current user
    let target_user_id = if user_id == "me" {
        auth.user_id
    } else {
        user_id
            .parse::<Uuid>()
            .map_err(|_| AppError::BadRequest("Invalid user_id".to_string()))?
    };

    // Users can only update their own notify props
    if target_user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot update notification properties for other users".to_string(),
        ));
    }

    // Verify membership
    let _membership = ChannelRepository::new(&state.db)
        .require_member(channel_id, auth.user_id)
        .await?;

    ChannelRepository::new(&state.db)
        .update_member_notify_props(channel_id, auth.user_id, &input)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// Channel stats response
#[derive(Debug, serde::Serialize)]
struct ChannelStats {
    channel_id: String,
    member_count: i64,
}

/// Get channel statistics
/// GET /channels/{id}/stats
async fn get_channel_stats(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> ApiResult<Json<ChannelStats>> {
    // Verify membership
    let _membership = ChannelRepository::new(&state.db)
        .require_member(channel_id, auth.user_id)
        .await?;

    let member_count = ChannelRepository::new(&state.db)
        .count_members(channel_id)
        .await?;

    Ok(Json(ChannelStats {
        channel_id: channel_id.to_string(),
        member_count,
    }))
}
