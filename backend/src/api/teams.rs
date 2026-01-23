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
    error::AppError,
    models::team::{AddTeamMember, CreateTeam, Team, TeamMember, TeamMemberResponse},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_teams).post(create_team))
        .route("/public", get(list_public_teams))
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
    let teams = sqlx::query_as::<_, Team>(
        r#"
        SELECT t.* FROM teams t
        INNER JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = $1
        ORDER BY t.name
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(teams))
}

/// Create a new team
async fn create_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateTeam>,
) -> Result<Json<Team>, AppError> {
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

    let team = sqlx::query_as::<_, Team>(
        r#"
        INSERT INTO teams (id, org_id, name, display_name, description)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(team_id)
    .bind(org_id)
    .bind(&payload.name)
    .bind(&payload.display_name)
    .bind(&payload.description)
    .fetch_one(&state.db)
    .await?;

    // Auto-add creator as admin
    sqlx::query(
        r#"
        INSERT INTO team_members (team_id, user_id, role)
        VALUES ($1, $2, 'admin')
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(team))
}

/// Get a specific team
async fn get_team(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Team>, AppError> {
    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".into()))?;

    Ok(Json(team))
}

/// Delete a team
async fn delete_team(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(())
}

/// Get team members with user details
async fn get_members(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TeamMemberResponse>>, AppError> {
    let members = sqlx::query_as::<_, TeamMemberResponse>(
        r#"
        SELECT tm.team_id, tm.user_id, tm.role, tm.created_at,
               u.username, u.display_name, u.avatar_url
        FROM team_members tm
        JOIN users u ON tm.user_id = u.id
        WHERE tm.team_id = $1
        ORDER BY u.username
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(members))
}

/// Add a member to a team
async fn add_member(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddTeamMember>,
) -> Result<Json<TeamMember>, AppError> {
    let member = sqlx::query_as::<_, TeamMember>(
        r#"
        INSERT INTO team_members (team_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(payload.user_id)
    .bind(payload.role.unwrap_or_else(|| "member".into()))
    .fetch_one(&state.db)
    .await?;

    // Also add user to all public channels in the team
    sqlx::query(
        r#"
        INSERT INTO channel_members (channel_id, user_id)
        SELECT c.id, $1 FROM channels c
        WHERE c.team_id = $2 AND c.channel_type = 'public'
        ON CONFLICT (channel_id, user_id) DO NOTHING
        "#,
    )
    .bind(payload.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(member))
}

/// Remove a member from a team
async fn remove_member(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(())
}

/// List channels in a team
async fn list_team_channels(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<crate::models::channel::Channel>>, AppError> {
    let channels = sqlx::query_as::<_, crate::models::channel::Channel>(
        r#"
        SELECT * FROM channels WHERE team_id = $1
        ORDER BY name
        "#,
    )
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(channels))
}

/// List all public teams that user can join
async fn list_public_teams(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Vec<Team>>, AppError> {
    // Get all public teams, marking which ones user is already a member of
    let teams = sqlx::query_as::<_, Team>(
        r#"
        SELECT t.* FROM teams t
        WHERE t.is_public = true
        ORDER BY t.name
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(teams))
}

/// Join a public team
async fn join_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TeamMember>, AppError> {
    // Check if team exists and is public
    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".into()))?;

    if !team.is_public && !team.allow_open_invite {
        return Err(AppError::Forbidden(
            "This team does not allow open joining".into(),
        ));
    }

    // Check if already a member
    let existing: Option<TeamMember> =
        sqlx::query_as("SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest("Already a member of this team".into()));
    }

    // Add user as member
    let member = sqlx::query_as::<_, TeamMember>(
        r#"
        INSERT INTO team_members (team_id, user_id, role)
        VALUES ($1, $2, 'member')
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    // Also add user to all public channels in the team
    sqlx::query(
        r#"
        INSERT INTO channel_members (channel_id, user_id)
        SELECT c.id, $1 FROM channels c
        WHERE c.team_id = $2 AND c.channel_type = 'public'
        ON CONFLICT (channel_id, user_id) DO NOTHING
        "#,
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(member))
}

/// Leave a team
async fn leave_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Remove from all channels in team first
    sqlx::query(
        r#"
        DELETE FROM channel_members
        WHERE user_id = $1 AND channel_id IN (
            SELECT id FROM channels WHERE team_id = $2
        )
        "#,
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    // Remove from team
    sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

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
    // Check if user is admin of the team
    let member: Option<TeamMember> =
        sqlx::query_as("SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?;

    match member {
        Some(m) if m.role == "admin" || m.role == "owner" => {}
        _ => {
            return Err(AppError::Forbidden(
                "Only team admins can update team settings".into(),
            ))
        }
    }

    let team = sqlx::query_as::<_, Team>(
        r#"
        UPDATE teams SET
            name = COALESCE($1, name),
            display_name = COALESCE($2, display_name),
            description = COALESCE($3, description),
            is_public = COALESCE($4, is_public),
            allow_open_invite = COALESCE($5, allow_open_invite),
            updated_at = NOW()
        WHERE id = $6
        RETURNING *
        "#,
    )
    .bind(payload.name)
    .bind(payload.display_name)
    .bind(payload.description)
    .bind(payload.is_public)
    .bind(payload.allow_open_invite)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(team))
}
