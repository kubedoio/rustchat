use axum::{
    extract::{Multipart, Path, State},
    response::IntoResponse,
    Json,
};

use super::ensure_team_admin_or_system_manage;
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::parse_mm_or_uuid;
use crate::models::Team;

pub async fn get_team_image(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    // Verify team exists (even though we return placeholder, ensures valid team_id)
    let _: Team = sqlx::query_as("SELECT * FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::TeamNotFound)?;

    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    Ok(([(axum::http::header::CONTENT_TYPE, "image/png")], PNG_1X1))
}

/// POST /teams/{team_id}/image - Upload/update the team icon
pub async fn update_team_icon(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    mut multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let team_uuid = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    ensure_team_admin_or_system_manage(&state, team_uuid, &auth).await?;

    const MAX_SIZE: usize = 1024 * 1024; // 1 MB

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().map(|s| s.to_string());
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let is_image_field = name == "image" || name.is_empty();
        let has_filename = filename.is_some();
        let is_image_type = content_type.starts_with("image/");

        if !(is_image_field && (is_image_type || has_filename)) {
            continue;
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?
            .to_vec();

        if data.is_empty() {
            continue;
        }

        if data.len() > MAX_SIZE {
            return Err(AppError::BadRequest(
                "Image exceeds maximum size of 1MB".to_string(),
            ));
        }

        // Detect format from magic bytes and validate
        let ext = if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            "png"
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            "jpg"
        } else if data.starts_with(b"GIF") {
            "gif"
        } else {
            return Err(AppError::BadRequest(
                "Unsupported image format. Supported formats: PNG, JPEG, GIF".to_string(),
            ));
        };

        let timestamp = chrono::Utc::now().timestamp();
        // Note: Currently storing icon path in database only. Full object storage
        // integration for team icons is not yet implemented.
        let icon_path = format!("teams/{}/icon.{}.{}", team_uuid, timestamp, ext);

        sqlx::query("UPDATE teams SET icon_path = $1 WHERE id = $2")
            .bind(&icon_path)
            .bind(team_uuid)
            .execute(&state.db)
            .await?;

        return Ok(Json(serde_json::json!({"status": "OK"})));
    }

    Err(AppError::BadRequest(
        "No image field found in upload".to_string(),
    ))
}

/// DELETE /teams/{team_id}/image - Remove the team icon
pub async fn delete_team_icon(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_uuid = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    ensure_team_admin_or_system_manage(&state, team_uuid, &auth).await?;

    // Note: Currently only clearing the icon_path in database. Full object storage
    // cleanup for team icons is not yet implemented.
    sqlx::query("UPDATE teams SET icon_path = NULL WHERE id = $1")
        .bind(team_uuid)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}
