//! Uploads API endpoints for resumable file uploads

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use image::{GenericImageView, ImageFormat};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::repositories::{ChannelRepository, UploadRepository};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/uploads", post(create_upload))
        .route("/uploads/{upload_id}", get(get_upload).post(upload_data))
}

fn filename_extension(filename: &str) -> Option<&str> {
    filename
        .rsplit_once('.')
        .and_then(|(_, ext)| if ext.is_empty() { None } else { Some(ext) })
}

fn image_mime_and_extension_from_bytes(data: &[u8]) -> Option<(&'static str, &'static str)> {
    let format = image::guess_format(data).ok()?;
    match format {
        ImageFormat::Png => Some(("image/png", "png")),
        ImageFormat::Jpeg => Some(("image/jpeg", "jpg")),
        ImageFormat::Gif => Some(("image/gif", "gif")),
        ImageFormat::WebP => Some(("image/webp", "webp")),
        ImageFormat::Bmp => Some(("image/bmp", "bmp")),
        ImageFormat::Tiff => Some(("image/tiff", "tiff")),
        _ => None,
    }
}

fn extension_from_mime(mime_type: &str) -> Option<&'static str> {
    match mime_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        "image/tiff" => Some("tiff"),
        _ => None,
    }
}

fn normalize_upload_session_file_metadata(filename: &str, data: &[u8]) -> (String, String, String) {
    let mut normalized_filename = filename.to_string();
    let mut mime_type = mime_guess::from_path(filename)
        .first_or_octet_stream()
        .to_string();

    if let Some((detected_mime, detected_ext)) = image_mime_and_extension_from_bytes(data) {
        if mime_type == "application/octet-stream" || !mime_type.starts_with("image/") {
            mime_type = detected_mime.to_string();
        }

        if filename_extension(&normalized_filename).is_none() {
            normalized_filename = format!("{}.{}", normalized_filename, detected_ext);
        }
    } else if filename_extension(&normalized_filename).is_none() {
        if let Some(ext) = extension_from_mime(&mime_type) {
            normalized_filename = format!("{}.{}", normalized_filename, ext);
        }
    }

    let extension = filename_extension(&normalized_filename)
        .unwrap_or_default()
        .to_string();

    (normalized_filename, mime_type, extension)
}

#[derive(Debug, Deserialize)]
struct CreateUploadRequest {
    channel_id: String,
    filename: String,
    file_size: i64,
}

/// POST /api/v4/uploads - Create a new upload session
async fn create_upload(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreateUploadRequest>,
) -> ApiResult<(StatusCode, Json<mm::UploadSession>)> {
    let channel_id = parse_mm_or_uuid(&input.channel_id)
        .ok_or_else(|| AppError::InvalidChannelId)?;

    // Verify user has access to channel
    let _ = ChannelRepository::new(&state.db)
        .require_member(channel_id, auth.user_id)
        .await?;

    // Create upload session
    let session_id = Uuid::new_v4();
    let now = Utc::now();
    let expires_at = now + chrono::Duration::hours(24);

    UploadRepository::new(&state.db)
        .create_session(
            session_id,
            auth.user_id,
            channel_id,
            &input.filename,
            input.file_size,
            now,
            expires_at,
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(mm::UploadSession {
            id: encode_mm_id(session_id),
            user_id: encode_mm_id(auth.user_id),
            channel_id: encode_mm_id(channel_id),
            filename: input.filename,
            file_size: input.file_size,
            file_offset: 0,
            create_at: now.timestamp_millis(),
        }),
    ))
}

/// GET /api/v4/uploads/{upload_id} - Get upload session details
async fn get_upload(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(upload_id): Path<String>,
) -> ApiResult<Json<mm::UploadSession>> {
    let upload_id = parse_mm_or_uuid(&upload_id)
        .ok_or_else(|| AppError::InvalidUploadId)?;

    let session = UploadRepository::new(&state.db)
        .get_session(upload_id)
        .await?
        .ok_or_else(|| AppError::UploadNotFound)?;

    // Only the creator can view the session
    if session.user_id != auth.user_id {
        return Err(AppError::Forbidden("Not your upload session".to_string()));
    }

    Ok(Json(mm::UploadSession {
        id: encode_mm_id(session.id),
        user_id: encode_mm_id(session.user_id),
        channel_id: encode_mm_id(session.channel_id),
        filename: session.filename,
        file_size: session.file_size,
        file_offset: session.file_offset,
        create_at: session.created_at.timestamp_millis(),
    }))
}

/// POST /api/v4/uploads/{upload_id} - Upload file data (resumable)
async fn upload_data(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(upload_id): Path<String>,
    body: Bytes,
) -> ApiResult<impl IntoResponse> {
    let upload_id = parse_mm_or_uuid(&upload_id)
        .ok_or_else(|| AppError::InvalidUploadId)?;

    let session = UploadRepository::new(&state.db)
        .get_session(upload_id)
        .await?
        .ok_or_else(|| AppError::UploadNotFound)?;

    if session.user_id != auth.user_id {
        return Err(AppError::Forbidden("Not your upload session".to_string()));
    }

    let new_offset = session.file_offset + body.len() as i64;

    // Append data to session
    UploadRepository::new(&state.db)
        .append_data(upload_id, body.as_ref(), new_offset)
        .await?;

    // Check if upload is complete
    if new_offset >= session.file_size {
        // Retrieve full file data
        let file_data = UploadRepository::new(&state.db)
            .get_file_data(upload_id)
            .await?
            .unwrap_or_default();

        // Create file record and upload to S3
        let file_id = Uuid::new_v4();
        let now = Utc::now();
        let (filename, mime_type, extension) =
            normalize_upload_session_file_metadata(&session.filename, &file_data);

        // Generate S3 key
        let key = if extension.is_empty() {
            format!("files/{}/{}", auth.user_id, file_id)
        } else {
            format!("files/{}/{}.{}", auth.user_id, file_id, extension)
        };

        // Calculate hash
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let hash = hex::encode(hasher.finalize());

        // Upload to S3
        state
            .s3_client
            .upload(&key, file_data.clone(), &mime_type)
            .await?;

        // Image processing for thumbnails (blocking operation offloaded)
        let (width, height, thumbnail_data, preview_data) = if mime_type.starts_with("image/") {
            let data_clone = file_data.clone();

            tokio::task::spawn_blocking(move || {
                if let Ok(img) = image::load_from_memory(&data_clone) {
                    let (w, h) = img.dimensions();
                    let w_out = Some(w as i32);
                    let h_out = Some(h as i32);

                    // Generate thumbnail (400x400 max) as JPEG for Mattermost mobile compatibility
                    let thumb_data = if w > 400 || h > 400 {
                        let thumb = img.thumbnail(400, 400);
                        let mut buf = Vec::new();
                        if thumb
                            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
                            .is_ok()
                        {
                            Some(buf)
                        } else {
                            None
                        }
                    } else {
                        let mut buf = Vec::new();
                        if img
                            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
                            .is_ok()
                        {
                            Some(buf)
                        } else {
                            None
                        }
                    };

                    // Generate preview (1024x1024 max) as JPEG for Mattermost mobile compatibility
                    let preview_data = if w > 1024 || h > 1024 {
                        let preview = img.thumbnail(1024, 1024);
                        let mut buf = Vec::new();
                        if preview
                            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
                            .is_ok()
                        {
                            Some(buf)
                        } else {
                            None
                        }
                    } else {
                        let mut buf = Vec::new();
                        if img
                            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
                            .is_ok()
                        {
                            Some(buf)
                        } else {
                            None
                        }
                    };

                    (w_out, h_out, thumb_data, preview_data)
                } else {
                    (None, None, None, None)
                }
            })
            .await
            .unwrap_or((None, None, None, None))
        } else {
            (None, None, None, None)
        };

        // Upload thumbnail to S3 if generated
        let thumbnail_key: Option<String> = if let Some(thumb_data) = thumbnail_data {
            let thumb_key = format!("thumbnails/{}/{}.jpg", auth.user_id, file_id);
            if state
                .s3_client
                .upload(&thumb_key, thumb_data, "image/jpeg")
                .await
                .is_ok()
            {
                Some(thumb_key)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(preview_data) = preview_data {
            let preview_key = format!("previews/{}/{}.jpg", auth.user_id, file_id);
            let _ = state
                .s3_client
                .upload(&preview_key, preview_data, "image/jpeg")
                .await;
        }

        let has_thumbnail = thumbnail_key.is_some();

        // Insert into files table with correct schema
        UploadRepository::new(&state.db)
            .create_file(
                file_id,
                auth.user_id,
                session.channel_id,
                &filename,
                &key,
                &mime_type,
                session.file_size,
                &hash,
                width,
                height,
                has_thumbnail,
                &thumbnail_key,
                now,
            )
            .await?;

        // Delete upload session
        UploadRepository::new(&state.db)
            .delete_session(upload_id)
            .await?;

        // Return FileInfo
        let file_info = mm::FileInfo {
            id: encode_mm_id(file_id),
            user_id: encode_mm_id(auth.user_id),
            post_id: "".to_string(),
            channel_id: encode_mm_id(session.channel_id),
            create_at: now.timestamp_millis(),
            update_at: now.timestamp_millis(),
            delete_at: 0,
            name: filename,
            extension,
            size: session.file_size,
            mime_type,
            width: width.unwrap_or(0),
            height: height.unwrap_or(0),
            has_preview_image: has_thumbnail,
            mini_preview: None,
        };

        Ok((StatusCode::CREATED, Json(serde_json::to_value(file_info)?)).into_response())
    } else {
        // Upload incomplete
        Ok(StatusCode::NO_CONTENT.into_response())
    }
}
