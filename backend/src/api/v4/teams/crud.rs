use axum::{
    body::Bytes,
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use super::utils::parse_body;
use super::{ensure_team_admin_or_system_manage, ensure_team_member};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use crate::models::Team;
use crate::repositories::TeamRepository;

pub async fn get_teams(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Team>>> {
    // Return teams the user is member of
    let teams: Vec<Team> = TeamRepository::new(&state.db)
        .list_teams_for_user(auth.user_id)
        .await?;

    let mm_teams: Vec<mm::Team> = teams.into_iter().map(|t| t.into()).collect();
    Ok(Json(mm_teams))
}

pub async fn get_team(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<mm::Team>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_member(&state, team_id, auth.user_id).await?;
    let team: Team = TeamRepository::new(&state.db)
        .get_team_by_id(team_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;

    Ok(Json(team.into()))
}

pub async fn patch_team(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Team>> {
    let _value: serde_json::Value = parse_body(&headers, &body, "Invalid patch body")?;
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;
    let team: Team = TeamRepository::new(&state.db)
        .get_team_by_id(team_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;
    Ok(Json(team.into()))
}

#[derive(Deserialize)]
pub struct UpdatePrivacyRequest {
    privacy: String, // "O" for open, "I" for invite
}

pub async fn update_team_privacy(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Json(input): Json<UpdatePrivacyRequest>,
) -> ApiResult<Json<mm::Team>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;
    let privacy = match input.privacy.as_str() {
        "O" => "open",
        "I" => "invite",
        _ => {
            return Err(AppError::BadRequest(
                "Invalid privacy value: must be 'O' (open) or 'I' (invite)".to_string(),
            ))
        }
    };
    let updated: Team = TeamRepository::new(&state.db)
        .update_team_privacy(team_id, privacy)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;
    Ok(Json(updated.into()))
}

pub async fn restore_team(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<mm::Team>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;
    // Verify team exists and is archived (deleted_at IS NOT NULL)
    let team: Team = TeamRepository::new(&state.db)
        .get_team_by_id(team_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;
    if team.deleted_at.is_none() {
        return Err(AppError::BadRequest("Team is not archived".to_string()));
    }
    let restored: Team = TeamRepository::new(&state.db)
        .restore_team(team_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;
    Ok(Json(restored.into()))
}

pub async fn get_team_by_name(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(name): Path<String>,
) -> ApiResult<Json<mm::Team>> {
    let team: Team = TeamRepository::new(&state.db)
        .get_team_by_name(&name)
        .await?
        .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;
    ensure_team_member(&state, team.id, auth.user_id).await?;
    Ok(Json(team.into()))
}

pub async fn team_name_exists(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let exists = TeamRepository::new(&state.db)
        .team_name_exists(&name)
        .await?;
    Ok(Json(serde_json::json!({"exists": exists})))
}
