//! Files API endpoints

use axum::{
    extract::{Multipart, Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use image::{GenericImageView, ImageFormat};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{FileInfo, FileUploadResponse, PresignedUploadUrl};

/// Build files routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/files", post(upload_file))
        .route("/files/presign", post(get_presigned_upload))
        .route("/files/{id}", get(get_file).delete(delete_file))
        .route("/files/{id}/download", get(download_file))
}

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub channel_id: Option<Uuid>,
}

/// Upload a file (multipart)
async fn upload_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<UploadQuery>,
    mut multipart: Multipart,
) -> ApiResult<Json<FileUploadResponse>> {
    let mut file_data: Option<(String, String, Vec<u8>)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?;
            file_data = Some((filename, content_type, data.to_vec()));
            break;
        }
    }

    let (filename, content_type, data) =
        file_data.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    // Generate unique key
    let file_id = Uuid::new_v4();
    let extension = filename.rsplit('.').next().unwrap_or("");
    let key = format!("files/{}/{}.{}", auth.user_id, file_id, extension);

    // Calculate SHA256
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hex::encode(hasher.finalize());

    let size = data.len() as i64;

    // Upload to S3
    state
        .s3_client
        .upload(&key, data.clone(), &content_type)
        .await?;

    // Save metadata to DB
    let file_info: FileInfo = sqlx::query_as(
        r#"
        INSERT INTO files (id, uploader_id, channel_id, name, key, mime_type, size, sha256)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(file_id)
    .bind(auth.user_id)
    .bind(query.channel_id)
    .bind(&filename)
    .bind(&key)
    .bind(&content_type)
    .bind(size)
    .bind(&hash)
    .fetch_one(&state.db)
    .await?;

    // Generate download URL
    let url = state.s3_client.presigned_download_url(&key, 3600).await?;

    // --- Image Processing (Background) ---
    if content_type.starts_with("image/") {
        let state_clone = state.clone();
        let data_clone = data.clone();
        let auth_id = auth.user_id;

        tokio::spawn(async move {
            if let Ok(img) = image::load_from_memory(&data_clone) {
                let (w, h) = img.dimensions();
                let width = Some(w as i32);
                let height = Some(h as i32);
                let mut has_thumbnail = false;
                let mut thumbnail_key = None;

                // Generate thumbnail if image is significantly larger than thumbnail size
                if w > 400 || h > 400 {
                    let thumb = img.thumbnail(400, 400);
                    let mut thumb_data = Vec::new();
                    if thumb
                        .write_to(&mut Cursor::new(&mut thumb_data), ImageFormat::WebP)
                        .is_ok()
                    {
                        let t_key = format!("thumbnails/{}/{}.webp", auth_id, file_id);
                        if state_clone
                            .s3_client
                            .upload(&t_key, thumb_data, "image/webp")
                            .await
                            .is_ok()
                        {
                            has_thumbnail = true;
                            thumbnail_key = Some(t_key);
                        }
                    }
                }

                // Update metadata in DB
                let _ = sqlx::query(
                    "UPDATE files SET width = $1, height = $2, has_thumbnail = $3, thumbnail_key = $4 WHERE id = $5"
                )
                .bind(width)
                .bind(height)
                .bind(has_thumbnail)
                .bind(thumbnail_key)
                .bind(file_id)
                .execute(&state_clone.db)
                .await;
            }
        });
    }

    Ok(Json(FileUploadResponse {
        id: file_info.id,
        name: file_info.name,
        mime_type: file_info.mime_type,
        size: file_info.size,
        url,
        thumbnail_url: None, // Will be populated when the record is fetched later
    }))
}

#[derive(Debug, Deserialize)]
pub struct PresignRequest {
    pub filename: String,
    pub content_type: String,
    pub _channel_id: Option<Uuid>,
}

/// Get a presigned upload URL
async fn get_presigned_upload(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<PresignRequest>,
) -> ApiResult<Json<PresignedUploadUrl>> {
    let file_id = Uuid::new_v4();
    let extension = input.filename.rsplit('.').next().unwrap_or("");
    let key = format!("files/{}/{}.{}", auth.user_id, file_id, extension);

    let upload_url = state
        .s3_client
        .presigned_upload_url(&key, &input.content_type, 3600)
        .await?;

    Ok(Json(PresignedUploadUrl {
        upload_url,
        file_key: key,
        expires_in: 3600,
    }))
}

/// Get file info
async fn get_file(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<FileInfo>> {
    let file: FileInfo = sqlx::query_as("SELECT * FROM files WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    Ok(Json(file))
}

/// Get presigned download URL
async fn download_file(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let file: FileInfo = sqlx::query_as("SELECT * FROM files WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    let url = state
        .s3_client
        .presigned_download_url(&file.key, 3600)
        .await?;

    Ok(Json(serde_json::json!({
        "url": url,
        "filename": file.name,
        "content_type": file.mime_type
    })))
}

/// Delete a file
async fn delete_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let file: FileInfo = sqlx::query_as("SELECT * FROM files WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    // Only uploader or admin can delete
    if file.uploader_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Cannot delete this file".to_string()));
    }

    // Delete from S3
    state.s3_client.delete(&file.key).await?;

    // Delete from DB
    sqlx::query("DELETE FROM files WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}
