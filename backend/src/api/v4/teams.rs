use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::{Channel, Team};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/teams", get(get_teams))
        .route("/teams/{team_id}", get(get_team))
        .route("/teams/{team_id}/image", get(get_team_image))
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
    Path(team_id): Path<String>,
) -> ApiResult<Json<mm::Team>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    let team: Team = sqlx::query_as("SELECT * FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(team.into()))
}

async fn get_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
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
    Path(team_id): Path<String>,
) -> ApiResult<Json<mm::TeamMember>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    let member: crate::models::TeamMember =
        sqlx::query_as("SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Forbidden("Not a member of this team".to_string())
            })?;

    Ok(Json(mm::TeamMember {
        team_id: encode_mm_id(member.team_id),
        user_id: encode_mm_id(member.user_id),
        roles: "team_user".to_string(),
        delete_at: 0,
        scheme_guest: false,
        scheme_user: true,
        scheme_admin: false,
    }))
}

async fn get_team_image(
    State(_state): State<AppState>,
    Path(team_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let _team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;

    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    Ok(([(axum::http::header::CONTENT_TYPE, "image/png")], PNG_1X1))
}
