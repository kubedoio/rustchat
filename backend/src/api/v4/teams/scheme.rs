use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::ensure_team_member;
use super::utils::status_ok;
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

#[derive(Deserialize)]
pub struct UpdateTeamSchemeRequest {
    scheme_id: String,
}

pub async fn get_team_scheme(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    ensure_team_member(&state, team_id, auth.user_id).await?;

    let row: (Option<Uuid>,) =
        sqlx::query_as("SELECT scheme_id FROM teams WHERE id = $1 AND deleted_at IS NULL")
            .bind(team_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::TeamNotFound)?;

    let scheme_id = row.0;

    Ok(Json(serde_json::json!({
        "team_id": encode_mm_id(team_id),
        "scheme_id": scheme_id.map(encode_mm_id),
        "scheme_name": serde_json::Value::Null,
        "default_team_admin_role": "team_admin",
        "default_team_user_role": "team_user",
        "default_team_guest_role": "team_guest",
    })))
}

pub async fn update_team_scheme(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Json(input): Json<UpdateTeamSchemeRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden("System admin required".to_string()));
    }

    let new_scheme_id: Option<Uuid> = if input.scheme_id.is_empty() {
        None
    } else {
        Some(parse_mm_or_uuid(&input.scheme_id).ok_or_else(|| AppError::InvalidSchemeId)?)
    };

    let rows_affected = sqlx::query(
        "UPDATE teams SET scheme_id = $1, updated_at = NOW() WHERE id = $2 AND deleted_at IS NULL",
    )
    .bind(new_scheme_id)
    .bind(team_id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::TeamNotFound);
    }

    Ok(status_ok())
}
