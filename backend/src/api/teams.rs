//! Teams API handlers

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use super::AppState;
use crate::{
    auth::middleware::AuthUser,
    auth::policy::permissions,
    error::AppError,
    models::{
        normalize_avatar_url,
        team::{AddTeamMember, CreateTeam, Team, TeamMember, TeamMemberResponse},
    },
    repositories::TeamRepository,
    services::team_membership::{
        apply_default_channel_membership_for_team_join, ensure_default_channels_for_team,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_teams).post(create_team))
        .route("/public", get(list_public_teams))
        .route("/all", get(list_all_teams))
        .route("/{id}", get(get_team).delete(delete_team).put(update_team))
        .route("/{id}/join", post(join_team))
        .route("/{id}/leave", post(leave_team))
        .route("/{id}/members", get(get_members).post(add_member))
        .route("/{id}/members/{user_id}", delete(remove_member))
        .route("/{team_id}/channels", get(list_team_channels))
}

/// List all teams the current user belongs to
async fn list_teams(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Team>>, AppError> {
    let repo = TeamRepository::new(&state.db);
    let teams = repo.list_teams_for_user(auth.user_id).await?;

    Ok(Json(teams))
}

/// List all teams (for users with TEAM_MANAGE permission)
/// Used for membership policy management and other admin functions
async fn list_all_teams(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Team>>, AppError> {
    // Check if user has permission to manage teams
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(AppError::Forbidden(
            "Missing permission to view all teams".to_string(),
        ));
    }

    let repo = TeamRepository::new(&state.db);
    let teams = repo.list_teams().await?;

    Ok(Json(teams))
}

/// Create a new team
async fn create_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateTeam>,
) -> Result<Json<Team>, AppError> {
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(AppError::Forbidden(
            "Missing permission to create teams".to_string(),
        ));
    }

    let team_id = Uuid::new_v4();

    // Get user's org_id
    let org_id = if let Some(id) = auth.org_id {
        id
    } else {
        // User has no org, create one based on team info
        let new_org_id = Uuid::new_v4();

        let _ = sqlx::query(
            "INSERT INTO organizations (id, name, display_name, description) VALUES ($1, $2, $3, $4)"
        )
        .bind(new_org_id)
        .bind(&payload.name) // Use team name as org name
        .bind(&payload.display_name)
        .bind(format!("Organization for {}", payload.name))
        .execute(&state.db)
        .await?;

        // Update user to belong to this org
        let _ = sqlx::query("UPDATE users SET org_id = $1 WHERE id = $2")
            .bind(new_org_id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;

        new_org_id
    };

    let repo = TeamRepository::new(&state.db);
    let team = repo
        .create_team(
            team_id,
            org_id,
            &payload.name,
            payload.display_name.as_deref(),
            payload.description.as_deref(),
        )
        .await?;

    // Auto-add creator as admin
    repo.add_team_member(team_id, auth.user_id, "admin").await?;

    ensure_default_channels_for_team(&state, team_id, auth.user_id).await?;
    if let Err(err) =
        apply_default_channel_membership_for_team_join(&state, team_id, auth.user_id).await
    {
        tracing::warn!(
            team_id = %team_id,
            user_id = %auth.user_id,
            error = %err,
            "Default channel auto-join failed after team creation"
        );
    }

    Ok(Json(team))
}

/// Get a specific team
async fn get_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Team>, AppError> {
    let repo = TeamRepository::new(&state.db);
    let team = repo
        .get_team_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".into()))?;

    // Verify the user is a member of the team (or an admin)
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        let is_member = repo.is_team_member(id, auth.user_id).await?;

        if !is_member {
            return Err(AppError::Forbidden(
                "You do not have access to this team".into(),
            ));
        }
    }

    Ok(Json(team))
}

async fn ensure_team_management_access(
    state: &AppState,
    auth: &AuthUser,
    team_id: Uuid,
) -> Result<(), AppError> {
    if auth.has_permission(&permissions::TEAM_MANAGE) {
        return Ok(());
    }

    let repo = TeamRepository::new(&state.db);
    let member = repo.get_team_member(team_id, auth.user_id).await?;

    match member {
        Some(member) if member.role == "admin" || member.role == "owner" => Ok(()),
        _ => Err(AppError::Forbidden(
            "Only team admins can update team settings".into(),
        )),
    }
}

/// Delete a team
async fn delete_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    ensure_team_management_access(&state, &auth, id).await?;

    let repo = TeamRepository::new(&state.db);
    repo.delete_team(id).await?;

    Ok(())
}

/// Get team members with user details
async fn get_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TeamMemberResponse>>, AppError> {
    let repo = TeamRepository::new(&state.db);

    // Verify the user is a member of the team (or an admin)
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        let is_member = repo.is_team_member(id, auth.user_id).await?;

        if !is_member {
            return Err(AppError::Forbidden(
                "You do not have access to this team".into(),
            ));
        }
    }

    let mut members = repo.list_team_members(id).await?;

    for member in &mut members {
        member.avatar_url = normalize_avatar_url(member.user_id, member.avatar_url.as_deref());
    }

    Ok(Json(members))
}

/// Add a member to a team
async fn add_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddTeamMember>,
) -> Result<Json<TeamMember>, AppError> {
    let repo = TeamRepository::new(&state.db);

    // Permission check
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        let requester_role = repo.get_member_role(id, auth.user_id).await?;

        match requester_role.as_deref() {
            Some("admin") | Some("owner") => {} // Allow
            _ => return Err(AppError::Forbidden("Only admins can add members".into())),
        }
    }

    let member = repo
        .add_team_member(
            id,
            payload.user_id,
            &payload.role.unwrap_or_else(|| "member".into()),
        )
        .await?;

    if let Err(err) =
        apply_default_channel_membership_for_team_join(&state, id, payload.user_id).await
    {
        tracing::warn!(
            team_id = %id,
            user_id = %payload.user_id,
            error = %err,
            "Default channel auto-join failed after add_member"
        );
    }

    Ok(Json(member))
}

/// Remove a member from a team
async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    let repo = TeamRepository::new(&state.db);

    // Permission check
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        let requester_role = repo.get_member_role(id, auth.user_id).await?;

        match requester_role.as_deref() {
            Some("admin") | Some("owner") => {
                // Check target role
                let target_role = repo.get_member_role(id, user_id).await?;

                if let Some(target) = target_role {
                    if target == "admin" || target == "owner" {
                        return Err(AppError::Forbidden("Cannot remove other admins".into()));
                    }
                }
            }
            _ => return Err(AppError::Forbidden("Only admins can remove members".into())),
        }
    }

    repo.remove_team_member(id, user_id).await?;

    // Revoke team WebSocket subscription
    state.ws_hub.unsubscribe_team(user_id, id).await;

    Ok(())
}

/// List channels in a team
async fn list_team_channels(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<crate::models::channel::Channel>>, AppError> {
    let repo = TeamRepository::new(&state.db);
    let channels = repo.list_team_channels(team_id, auth.user_id).await?;

    Ok(Json(channels))
}

/// List all public teams that user can join
async fn list_public_teams(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Vec<Team>>, AppError> {
    // Get all public teams, marking which ones user is already a member of
    let repo = TeamRepository::new(&state.db);
    let teams = repo.list_public_teams().await?;

    Ok(Json(teams))
}

/// Join a public team
async fn join_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TeamMember>, AppError> {
    let repo = TeamRepository::new(&state.db);

    // Check if team exists and is public
    let team = repo
        .get_team_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".into()))?;

    if !team.is_public && !team.allow_open_invite {
        return Err(AppError::Forbidden(
            "This team does not allow open joining".into(),
        ));
    }

    // Check if already a member
    let existing = repo.get_team_member(id, auth.user_id).await?;

    if existing.is_some() {
        return Err(AppError::BadRequest("Already a member of this team".into()));
    }

    // Add user as member
    let member = repo.add_team_member(id, auth.user_id, "member").await?;

    if let Err(err) = apply_default_channel_membership_for_team_join(&state, id, auth.user_id).await
    {
        tracing::warn!(
            team_id = %id,
            user_id = %auth.user_id,
            error = %err,
            "Default channel auto-join failed after join_team"
        );
    }

    Ok(Json(member))
}

/// Leave a team
async fn leave_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = TeamRepository::new(&state.db);

    // Remove from all channels in team first
    let removed_channel_ids = repo
        .remove_user_from_team_channels(auth.user_id, id)
        .await?;

    for channel_id in removed_channel_ids {
        state
            .ws_hub
            .unsubscribe_channel(auth.user_id, channel_id)
            .await;
    }

    // Remove from team
    repo.remove_team_member(id, auth.user_id).await?;

    // Revoke team WebSocket subscription
    state.ws_hub.unsubscribe_team(auth.user_id, id).await;

    Ok(Json(serde_json::json!({"status": "left"})))
}

/// DTO for updating a team
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateTeam {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub allow_open_invite: Option<bool>,
}

/// Update a team
async fn update_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTeam>,
) -> Result<Json<Team>, AppError> {
    ensure_team_management_access(&state, &auth, id).await?;

    let repo = TeamRepository::new(&state.db);
    let team = repo
        .update_team(
            id,
            payload.name.as_deref(),
            payload.display_name.as_deref(),
            payload.description.as_deref(),
            payload.is_public,
            payload.allow_open_invite,
        )
        .await?;

    Ok(Json(team))
}
