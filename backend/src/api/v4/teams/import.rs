use axum::{
    extract::{Multipart, Path, State},
    Json,
};

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::parse_mm_or_uuid;

pub async fn import_team(
    State(_state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    mut multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let _team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;

    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Only system admins can import teams".to_string(),
        ));
    }

    // Extract the file field from multipart
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let _data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read file field: {}", e)))?;
            // File received; full import not yet implemented
            break;
        }
    }

    Ok(Json(serde_json::json!({
        "channels": 0,
        "posts": 0,
        "users": 0,
        "errors": [],
        "status": "Import functionality not yet fully implemented"
    })))
}
