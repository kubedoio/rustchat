use axum::{
    extract::{Path, State},
    response::Redirect,
    Json,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use crate::models::file::FileInfo;

pub async fn get_file_info(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(file_id_str): Path<String>,
) -> ApiResult<Json<mm::FileInfo>> {
    let file_id = parse_mm_or_uuid(&file_id_str)
        .ok_or_else(|| AppError::Validation("Invalid file_id".to_string()))?;

    let file: FileInfo = sqlx::query_as("SELECT * FROM files WHERE id = $1")
        .bind(file_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    Ok(Json(file.into()))
}

pub async fn get_file(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(file_id_str): Path<String>,
) -> ApiResult<Redirect> {
    let file_id = parse_mm_or_uuid(&file_id_str)
        .ok_or_else(|| AppError::Validation("Invalid file_id".to_string()))?;

    let file: FileInfo = sqlx::query_as("SELECT * FROM files WHERE id = $1")
        .bind(file_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    let url = state
        .s3_client
        .presigned_download_url(&file.key, 3600)
        .await?;

    Ok(Redirect::temporary(&url))
}
