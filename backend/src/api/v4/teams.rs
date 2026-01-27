use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::models::{Team, Channel};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/teams", get(get_teams))
        .route("/teams/{team_id}", get(get_team))
        .route("/teams/{team_id}/members/me", get(get_team_member_me))
        .route("/teams/{team_id}/channels", get(get_team_channels))
}

async fn get_teams(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Team>>> {
    // Return teams the user is member of
    let teams: Vec<Team> = sqlx::query_as(
        r#"
        SELECT t.* FROM teams t
        JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_teams: Vec<mm::Team> = teams.into_iter().map(|t| t.into()).collect();
    Ok(Json(mm_teams))
}

async fn get_team(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<mm::Team>> {
    let team: Team = sqlx::query_as("SELECT * FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(team.into()))
}

async fn get_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.* FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

async fn get_team_member_me(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<mm::TeamMember>> {
    let member: crate::models::TeamMember = sqlx::query_as(
        "SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2",
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this team".to_string()))?;

    Ok(Json(mm::TeamMember {
        team_id: member.team_id.to_string(),
        user_id: member.user_id.to_string(),
        roles: "team_user".to_string(),
        delete_at: 0,
        scheme_guest: false,
        scheme_user: true,
        scheme_admin: false,
    }))
}
