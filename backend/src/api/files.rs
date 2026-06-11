//! Files API endpoints

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use image::{GenericImageView, ImageFormat};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::AppState;
use crate::auth::policy::permissions;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{FileInfo, FileUploadResponse};
use crate::repositories::{ChannelRepository, FileRepository};

/// Verify the requesting user can access a file (must be member of the file's channel).
/// Files without a channel_id (e.g. profile photos) are accessible to any authenticated user.
async fn check_file_access(state: &AppState, file: &FileInfo, user_id: Uuid) -> ApiResult<()> {
    let Some(channel_id) = file.channel_id else {
        return Ok(());
    };
    let repo = ChannelRepository::new(&state.db);
    let is_member = repo.is_channel_member(channel_id, user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "You do not have access to this file".to_string(),
        ));
    }
    Ok(())
}

/// Verify the requesting user can associate an upload with the target channel.
async fn check_upload_channel_access(
    state: &AppState,
    channel_id: Option<Uuid>,
    user_id: Uuid,
) -> ApiResult<()> {
    let Some(channel_id) = channel_id else {
        return Ok(());
    };

    let repo = ChannelRepository::new(&state.db);
    let is_member = repo.is_channel_member(channel_id, user_id).await?;
    if !is_member {
        return Err(AppError::Forbidden(
            "You are not a member of this channel".to_string(),
        ));
    }

    Ok(())
}

fn native_file_content_url(file_id: Uuid) -> String {
    format!("/api/v1/files/{file_id}/content")
}

fn content_disposition_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|ch| match ch {
            '"' | '\\' | ';' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect()
}

/// Build files routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/files", post(upload_file))
        .route("/files/presign", post(get_presigned_upload))
        .route("/files/{id}", get(get_file).delete(delete_file))
        .route("/files/{id}/download", get(download_file))
        .route("/files/{id}/content", get(download_file_content))
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
    check_upload_channel_access(&state, query.channel_id, auth.user_id).await?;

    struct TempFile(std::path::PathBuf);
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    let mut file_info: Option<(String, String, TempFile, u64, String, Vec<u8>)> = None;

    while let Some(mut field) = multipart
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

            let temp_path =
                std::env::temp_dir().join(format!("rustchat_upload_{}", Uuid::new_v4()));
            let mut file = tokio::fs::File::create(&temp_path)
                .await
                .map_err(|e| AppError::Internal(format!("temp file create error: {}", e)))?;
            let mut hasher = Sha256::new();
            let mut size = 0u64;
            let mut head = Vec::with_capacity(8192);

            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?
            {
                let bytes = chunk.as_ref();
                if head.len() < 8192 {
                    let remaining = 8192 - head.len();
                    let to_copy = bytes.len().min(remaining);
                    head.extend_from_slice(&bytes[..to_copy]);
                }
                hasher.update(bytes);
                size += bytes.len() as u64;
                file.write_all(bytes)
                    .await
                    .map_err(|e| AppError::Internal(format!("temp file write error: {}", e)))?;
            }

            file_info = Some((
                filename,
                content_type,
                TempFile(temp_path),
                size,
                hex::encode(hasher.finalize()),
                head,
            ));
            break;
        }
    }

    let (filename, _content_type, temp_file, size, hash, head) =
        file_info.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    let (content_type, extension) =
        crate::api::file_validation::validate_file_upload_head(&filename, &head, size as usize)?;

    if matches!(extension.as_str(), "svg" | "txt" | "md") {
        let data = tokio::fs::read(&temp_file.0)
            .await
            .map_err(|e| AppError::Internal(format!("Read temp file: {}", e)))?;
        let _ = crate::api::file_validation::validate_file_upload(&filename, &data)?;
    }

    // Generate unique key
    let file_id = Uuid::new_v4();
    let key = if extension.is_empty() {
        format!("files/{}/{}", auth.user_id, file_id)
    } else {
        format!("files/{}/{}.{}", auth.user_id, file_id, extension)
    };

    let size_i64 = size as i64;

    // Upload to S3
    state
        .s3_client
        .upload_file(&key, &temp_file.0, &content_type)
        .await?;

    // Save metadata to DB
    let file_repo = FileRepository::new(&state.db);
    let file_info = file_repo
        .create_simple(
            file_id,
            auth.user_id,
            query.channel_id,
            &filename,
            &key,
            &content_type,
            size_i64,
            &hash,
        )
        .await?;

    // --- Image Processing (Background) ---
    if content_type.starts_with("image/") {
        let state_clone = state.clone();
        let path_clone = temp_file.0.clone();
        let auth_id = auth.user_id;
        std::mem::forget(temp_file);
        tokio::spawn(async move {
            if let Ok(img) = image::open(&path_clone) {
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
                let file_repo = FileRepository::new(&state_clone.db);
                let _ = file_repo
                    .update_dimensions(
                        file_id,
                        width,
                        height,
                        has_thumbnail,
                        thumbnail_key.as_deref(),
                    )
                    .await;
            }
            let _ = tokio::fs::remove_file(&path_clone).await;
        });
    }

    Ok(Json(FileUploadResponse {
        id: file_info.id,
        name: file_info.name,
        mime_type: file_info.mime_type,
        size: file_info.size,
        width: file_info.width.unwrap_or(0),
        height: file_info.height.unwrap_or(0),
        url: native_file_content_url(file_info.id),
        thumbnail_url: None, // Will be populated when the record is fetched later
    }))
}

#[derive(Debug, Deserialize)]
pub struct PresignRequest {
    pub filename: String,
    pub content_type: String,
    pub channel_id: Option<Uuid>,
}

/// Get a presigned upload URL
async fn get_presigned_upload(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Json(input): Json<PresignRequest>,
) -> ApiResult<impl IntoResponse> {
    let PresignRequest {
        filename: _filename,
        content_type: _content_type,
        channel_id: _channel_id,
    } = input;

    Ok((
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": {
                "code": "NOT_IMPLEMENTED",
                "message": "Presigned file upload URLs are not available. Use multipart POST /api/v1/files.",
                "details": null
            }
        })),
    ))
}

/// Get file info
async fn get_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<FileInfo>> {
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::FileNotFound)?;

    check_file_access(&state, &file, auth.user_id).await?;

    Ok(Json(file))
}

/// Get authenticated download URL
async fn download_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::FileNotFound)?;

    check_file_access(&state, &file, auth.user_id).await?;

    Ok(Json(serde_json::json!({
        "url": native_file_content_url(file.id),
        "filename": file.name,
        "content_type": file.mime_type
    })))
}

/// Stream file bytes through the backend after authorization.
async fn download_file_content(
    State(state): State<AppState>,
    auth: super::v4::extractors::MmAuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::FileNotFound)?;

    if let Err(e) = check_file_access(&state, &file, auth.user_id).await {
        if matches!(e, AppError::Forbidden(_)) {
            let _ = crate::services::audit::audit(
                &state.db,
                auth.user_id,
                crate::services::audit::AuditAction::FileDownloadDenied,
                "file",
                Some(id),
                serde_json::json!({ "file_name": file.name }),
            )
            .await;
        }
        return Err(e);
    }

    let stream = state.s3_client.download_stream(&file.key).await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    let file_name = file.name.clone();
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            actor,
            crate::services::audit::AuditAction::FileDownload,
            "file",
            Some(id),
            serde_json::json!({ "file_name": file_name }),
        )
        .await;
    });

    Ok((
        [
            (header::CONTENT_TYPE, file.mime_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "inline; filename=\"{}\"",
                    content_disposition_filename(&file.name)
                ),
            ),
            (
                header::CACHE_CONTROL,
                "max-age=2592000, private".to_string(),
            ),
            (
                header::HeaderName::from_static("x-content-type-options"),
                "nosniff".to_string(),
            ),
            (
                header::HeaderName::from_static("x-frame-options"),
                "DENY".to_string(),
            ),
            (
                header::HeaderName::from_static("content-security-policy"),
                "Frame-ancestors 'none'".to_string(),
            ),
        ],
        Body::new(stream.into_inner()),
    ))
}

/// Delete a file
async fn delete_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::FileNotFound)?;

    // Only uploader or admin can delete
    if !auth.can_access_owned(file.uploader_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot delete this file".to_string()));
    }

    // Delete from S3
    state.s3_client.delete(&file.key).await?;

    // Delete from DB
    file_repo.delete(id).await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}
