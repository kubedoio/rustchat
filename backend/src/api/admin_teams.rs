//! Admin team, channel, and group management endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use uuid::Uuid;

use crate::api::{admin::require_admin, AppState};
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    normalize_avatar_url, AddTeamMember, AdminChannelResponse, AdminTeamResponse, CreateChannel,
    TeamMember, TeamMemberResponse, UpdateChannel,
};
use crate::repositories::{AdminRepository, ChannelRepository};
use crate::services::team_membership::apply_default_channel_membership_for_team_join;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/teams", get(list_admin_teams))
        .route(
            "/admin/teams/{id}",
            get(get_admin_team)
                .put(update_admin_team)
                .delete(delete_admin_team),
        )
        .route(
            "/admin/teams/{id}/members",
            get(list_team_members).post(add_team_member),
        )
        .route(
            "/admin/teams/{id}/members/{user_id}",
            axum::routing::delete(remove_team_member),
        )
        .route(
            "/admin/channels",
            get(list_admin_channels).post(create_admin_channel),
        )
        .route(
            "/admin/channels/{id}",
            patch(update_admin_channel).delete(delete_admin_channel),
        )
}

// ============ Teams & Channels Management ============

#[derive(Debug, serde::Deserialize)]
pub struct ListTeamsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminTeamsListResponse {
    pub teams: Vec<AdminTeamResponse>,
    pub total: i64,
}

pub async fn list_admin_teams(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListTeamsQuery>,
) -> ApiResult<Json<AdminTeamsListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let repo = AdminRepository::new(&state.db);
    let teams = repo
        .list_teams(query.search.as_deref(), per_page, offset)
        .await?;
    let total = repo.count_teams().await?;

    Ok(Json(AdminTeamsListResponse { teams, total }))
}

pub async fn get_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AdminTeamResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let team = repo
        .get_team_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Team {} not found", id)))?;

    Ok(Json(team))
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct UpdateTeamRequest {
    display_name: Option<String>,
    description: Option<String>,
}

pub async fn update_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTeamRequest>,
) -> ApiResult<Json<AdminTeamResponse>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    repo.update_team(
        id,
        payload.display_name.as_deref(),
        payload.description.as_deref(),
    )
    .await?;

    let team = repo
        .get_team_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Team {} not found", id)))?;

    Ok(Json(team))
}

pub async fn delete_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    repo.delete_team(id).await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    let team_id = id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::TeamDelete,
            "team",
            Some(team_id),
            serde_json::Value::Null,
        )
        .await;
    });

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

pub async fn list_team_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<TeamMemberResponse>>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let mut members = repo.list_team_members(id).await?;

    for member in &mut members {
        member.avatar_url = normalize_avatar_url(member.user_id, member.avatar_url.as_deref());
    }

    Ok(Json(members))
}

pub async fn add_team_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddTeamMember>,
) -> ApiResult<Json<TeamMember>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let member = repo
        .add_team_member(id, payload.user_id, payload.role.as_deref())
        .await?;

    if let Err(err) =
        apply_default_channel_membership_for_team_join(&state, id, payload.user_id).await
    {
        tracing::warn!(
            team_id = %id,
            user_id = %payload.user_id,
            error = %err,
            "Default channel auto-join failed after admin add_team_member"
        );
    }

    let db = state.db.clone();
    let actor = auth.user_id;
    let team_id = id;
    let user_id = payload.user_id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::TeamMemberAdd,
            "team",
            Some(team_id),
            serde_json::json!({ "user_id": user_id }),
        )
        .await;
    });

    Ok(Json(member))
}

pub async fn remove_team_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let channel_ids = ChannelRepository::new(&state.db)
        .get_user_channel_ids_in_team(user_id, id)
        .await?;
    repo.remove_team_member(id, user_id).await?;

    // Revoke WebSocket subscriptions for all team channels and the team itself
    for channel_id in channel_ids {
        state.ws_hub.unsubscribe_channel(user_id, channel_id).await;
    }
    state.ws_hub.unsubscribe_team(user_id, id).await;

    let db = state.db.clone();
    let actor = auth.user_id;
    let team_id = id;
    let member_id = user_id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::TeamMemberRemove,
            "team",
            Some(team_id),
            serde_json::json!({ "user_id": member_id }),
        )
        .await;
    });

    Ok(Json(serde_json::json!({"status": "removed"})))
}

#[derive(Debug, serde::Deserialize)]
pub struct ListChannelsQuery {
    pub team_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminChannelsListResponse {
    pub channels: Vec<AdminChannelResponse>,
    pub total: i64,
}

pub async fn list_admin_channels(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListChannelsQuery>,
) -> ApiResult<Json<AdminChannelsListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let repo = AdminRepository::new(&state.db);
    let channels = repo
        .list_channels(query.team_id, query.search.as_deref(), per_page, offset)
        .await?;
    let total = repo.count_channels(query.team_id).await?;

    Ok(Json(AdminChannelsListResponse { channels, total }))
}

pub async fn create_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateChannel>,
) -> ApiResult<Json<crate::models::Channel>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    let channel = repo
        .create_channel(
            input.team_id,
            &input.name,
            input.display_name.as_deref(),
            input.purpose.as_deref(),
            input.channel_type,
            auth.user_id,
        )
        .await?;

    // Broadcast event
    let broadcast = if channel.channel_type == crate::models::ChannelType::Public {
        // Broadcast to entire team
        crate::realtime::WsBroadcast {
            team_id: Some(input.team_id),
            channel_id: None,
            user_id: None,
            exclude_user_id: None,
        }
    } else {
        // Private channel: broadcast only to creator (admin)
        crate::realtime::WsBroadcast {
            user_id: Some(auth.user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        }
    };

    let event = crate::realtime::WsEnvelope::event(
        crate::realtime::events::EventType::ChannelCreated,
        channel.clone(),
        Some(channel.id),
    )
    .with_broadcast(broadcast);

    state.ws_hub.broadcast(event).await;

    Ok(Json(channel))
}

pub async fn update_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateChannel>,
) -> ApiResult<Json<crate::models::Channel>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);

    if let Some(ref display_name) = input.display_name {
        repo.update_channel_display_name(id, display_name).await?;
    }
    if let Some(ref purpose) = input.purpose {
        repo.update_channel_purpose(id, purpose).await?;
    }
    if let Some(ref header) = input.header {
        repo.update_channel_header(id, header).await?;
    }

    let channel = repo
        .get_channel_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Channel {} not found", id)))?;

    Ok(Json(channel))
}

pub async fn delete_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AdminRepository::new(&state.db);
    repo.delete_channel(id).await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    let channel_id = id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::ChannelDelete,
            "channel",
            Some(channel_id),
            serde_json::Value::Null,
        )
        .await;
    });

    Ok(Json(serde_json::json!({"status": "deleted"})))
}
