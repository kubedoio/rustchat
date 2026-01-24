//! Channels API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{Channel, ChannelMember, CreateChannel, UpdateChannel};
use crate::realtime::events::{EventType, WsBroadcast, WsEnvelope};

/// Build channels routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_channels).post(create_channel))
        .route("/unreads", get(get_all_unread_counts))
        .route(
            "/{id}",
            get(get_channel).put(update_channel).delete(archive_channel),
        )
        .route("/{id}/members", get(list_members).post(add_member))
        .route("/{id}/members/{user_id}", delete(remove_member))
        .route("/{id}/read", post(mark_channel_as_read))
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
        let team_member =
            sqlx::query("SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2")
                .bind(query.team_id)
                .bind(auth.user_id)
                .fetch_optional(&state.db)
                .await?;

        if team_member.is_none() {
            return Err(AppError::Forbidden("Not a member of this team".to_string()));
        }

        // List public channels user is NOT in
        let channels: Vec<Channel> = sqlx::query_as(
            r#"
            SELECT c.* FROM channels c
            WHERE c.team_id = $1 
            AND c.type = 'public'::channel_type
            AND c.is_archived = false
            AND c.id NOT IN (
                SELECT channel_id FROM channel_members WHERE user_id = $2
            )
            ORDER BY c.name
            "#,
        )
        .bind(query.team_id)
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await?;

        return Ok(Json(channels));
    }

    // Default behavior: List channels user is a member of
    let channels: Vec<Channel> = if include_archived {
        sqlx::query_as(
            r#"
            SELECT c.* FROM channels c
            INNER JOIN channel_members cm ON cm.channel_id = c.id
            WHERE c.team_id = $1 AND cm.user_id = $2
            ORDER BY c.name
            "#,
        )
        .bind(query.team_id)
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT c.* FROM channels c
            INNER JOIN channel_members cm ON cm.channel_id = c.id
            WHERE c.team_id = $1 AND cm.user_id = $2 AND c.is_archived = false
            ORDER BY c.name
            "#,
        )
        .bind(query.team_id)
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await?
    };

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
        let dm_name = format!("dm_{}_{}", ids[0], ids[1]);

        // Check if DM channel already exists in this team
        let existing = sqlx::query_as::<_, Channel>(
            "SELECT * FROM channels WHERE team_id = $1 AND name = $2 AND type = 'direct'::channel_type",
        )
        .bind(input.team_id)
        .bind(&dm_name)
        .fetch_optional(&state.db)
        .await?;

        if let Some(channel) = existing {
            // Re-add both users as members just in case they left (resurrect DM)
            let _ = crate::services::posts::ensure_dm_membership(&state, channel.id).await;
            return Ok(Json(channel));
        }

        // Validate target user exists in the team
        let is_target_member =
            sqlx::query("SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2")
                .bind(input.team_id)
                .bind(target_id)
                .fetch_optional(&state.db)
                .await?;

        if is_target_member.is_none() {
            return Err(AppError::Forbidden(
                "Target user is not a member of this team".to_string(),
            ));
        }

        // Create DM channel
        let channel: Channel = sqlx::query_as(
            r#"
            INSERT INTO channels (team_id, name, display_name, purpose, type, creator_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(input.team_id)
        .bind(&dm_name)
        .bind(
            input
                .display_name
                .as_ref()
                .unwrap_or(&"Direct Message".to_string()),
        )
        .bind(&input.purpose)
        .bind(input.channel_type)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        // Add both users as members
        for user_id in ids {
            sqlx::query(
                "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING"
            )
            .bind(channel.id)
            .bind(user_id)
            .execute(&state.db)
            .await?;

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

    // Check if team exists and user is member
    let member = sqlx::query("SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(input.team_id)
        .bind(auth.user_id)
        .fetch_optional(&state.db)
        .await?;

    if member.is_none() {
        return Err(AppError::Forbidden("Not a member of this team".to_string()));
    }

    // Create channel
    let channel: Channel = sqlx::query_as(
        r#"
        INSERT INTO channels (team_id, name, display_name, purpose, type, creator_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(input.team_id)
    .bind(&input.name)
    .bind(&input.display_name)
    .bind(&input.purpose)
    .bind(input.channel_type)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    // Add creator as admin member
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'admin')")
        .bind(channel.id)
        .bind(auth.user_id)
        .execute(&state.db)
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
    let _member: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    let channel: Channel = sqlx::query_as("SELECT * FROM channels WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(channel))
}

/// Update a channel
async fn update_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateChannel>,
) -> ApiResult<Json<Channel>> {
    // Check admin membership
    let member: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    if member.role != "admin" && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Not an admin of this channel".to_string(),
        ));
    }

    // Update fields
    if let Some(ref display_name) = input.display_name {
        sqlx::query("UPDATE channels SET display_name = $1 WHERE id = $2")
            .bind(display_name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref purpose) = input.purpose {
        sqlx::query("UPDATE channels SET purpose = $1 WHERE id = $2")
            .bind(purpose)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref header) = input.header {
        sqlx::query("UPDATE channels SET header = $1 WHERE id = $2")
            .bind(header)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    let channel: Channel = sqlx::query_as("SELECT * FROM channels WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(channel))
}

/// Archive a channel
async fn archive_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Channel>> {
    // Check admin
    let member: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    if member.role != "admin" && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Not an admin of this channel".to_string(),
        ));
    }

    let channel: Channel =
        sqlx::query_as("UPDATE channels SET is_archived = true WHERE id = $1 RETURNING *")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(channel))
}

/// List channel members
async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<ChannelMember>>> {
    // Check membership
    let _: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    let members: Vec<ChannelMember> = sqlx::query_as(
        r#"
        SELECT cm.*, u.username, u.display_name, u.avatar_url, u.presence
        FROM channel_members cm
        INNER JOIN users u ON cm.user_id = u.id
        WHERE cm.channel_id = $1
        ORDER BY u.username ASC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

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
        let channel: Channel = sqlx::query_as("SELECT * FROM channels WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

        if channel.channel_type != crate::models::ChannelType::Public {
            let member: ChannelMember = sqlx::query_as(
                "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2",
            )
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

            if member.role != "admin" && auth.role != "system_admin" {
                return Err(AppError::Forbidden(
                    "Cannot join private channel without invite".to_string(),
                ));
            }
        }
        // If public, allow proceed
    } else {
        // Adding someone else - require admin
        let member: ChannelMember =
            sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
                .bind(id)
                .bind(auth.user_id)
                .fetch_optional(&state.db)
                .await?
                .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

        if member.role != "admin" && auth.role != "system_admin" {
            return Err(AppError::Forbidden(
                "Not an admin of this channel".to_string(),
            ));
        }
    }

    let new_member: ChannelMember = sqlx::query_as(
        r#"
        INSERT INTO channel_members (channel_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (channel_id, user_id) DO UPDATE SET role = $3
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(input.user_id)
    .bind(input.role.as_deref().unwrap_or("member"))
    .fetch_one(&state.db)
    .await?;

    // Announce join in public channels
    let channel_type = sqlx::query_scalar::<_, crate::models::ChannelType>(
        "SELECT type FROM channels WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    if channel_type == crate::models::ChannelType::Public {
        let username = sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
            .bind(input.user_id)
            .fetch_one(&state.db)
            .await?;

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
        let member: ChannelMember =
            sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
                .bind(channel_id)
                .bind(auth.user_id)
                .fetch_optional(&state.db)
                .await?
                .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

        if member.role != "admin" && auth.role != "system_admin" {
            return Err(AppError::Forbidden(
                "Not an admin of this channel".to_string(),
            ));
        }
    }

    sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
        .bind(channel_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "removed"})))
}
