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
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/uploads", post(create_upload))
        .route("/uploads/{upload_id}", get(get_upload).post(upload_data))
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
        .ok_or_else(|| AppError::BadRequest("Invalid channel_id".to_string()))?;

    // Verify user has access to channel
    let _: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Create upload session
    let session_id = Uuid::new_v4();
    let now = Utc::now();
    let expires_at = now + chrono::Duration::hours(24);

    sqlx::query(
        r#"
        INSERT INTO upload_sessions (id, user_id, channel_id, filename, file_size, file_offset, created_at, expires_at)
        VALUES ($1, $2, $3, $4, $5, 0, $6, $7)
        "#,
    )
    .bind(session_id)
    .bind(auth.user_id)
    .bind(channel_id)
    .bind(&input.filename)
    .bind(input.file_size)
    .bind(now)
    .bind(expires_at)
    .execute(&state.db)
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

#[derive(sqlx::FromRow)]
struct UploadSessionRow {
    id: Uuid,
    user_id: Uuid,
    channel_id: Uuid,
    filename: String,
    file_size: i64,
    file_offset: i64,
    created_at: chrono::DateTime<Utc>,
}

/// GET /api/v4/uploads/{upload_id} - Get upload session details
async fn get_upload(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(upload_id): Path<String>,
) -> ApiResult<Json<mm::UploadSession>> {
    let upload_id = parse_mm_or_uuid(&upload_id)
        .ok_or_else(|| AppError::BadRequest("Invalid upload_id".to_string()))?;

    let session: UploadSessionRow = sqlx::query_as(
        r#"
        SELECT id, user_id, channel_id, filename, file_size, file_offset, created_at
        FROM upload_sessions
        WHERE id = $1 AND expires_at > NOW()
        "#,
    )
    .bind(upload_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Upload session not found".to_string()))?;

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
        .ok_or_else(|| AppError::BadRequest("Invalid upload_id".to_string()))?;

    let session: UploadSessionRow = sqlx::query_as(
        r#"
        SELECT id, user_id, channel_id, filename, file_size, file_offset, created_at
        FROM upload_sessions
        WHERE id = $1 AND expires_at > NOW()
        "#,
    )
    .bind(upload_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Upload session not found".to_string()))?;

    if session.user_id != auth.user_id {
        return Err(AppError::Forbidden("Not your upload session".to_string()));
    }

    let new_offset = session.file_offset + body.len() as i64;

    // Append data to session
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET file_data = COALESCE(file_data, ''::bytea) || $1,
            file_offset = $2
        WHERE id = $3
        "#,
    )
    .bind(body.as_ref())
    .bind(new_offset)
    .bind(upload_id)
    .execute(&state.db)
    .await?;

    // Check if upload is complete
    if new_offset >= session.file_size {
        // Retrieve full file data
        let file_data: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT file_data FROM upload_sessions WHERE id = $1")
                .bind(upload_id)
                .fetch_one(&state.db)
                .await?;

        let file_data = file_data.unwrap_or_default();

        // Create file record and upload to S3
        let file_id = Uuid::new_v4();
        let now = Utc::now();
        let extension = session
            .filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_string();
        let mime_type = mime_guess::from_path(&session.filename)
            .first_or_octet_stream()
            .to_string();

        // Generate S3 key
        let key = format!("files/{}/{}.{}", auth.user_id, file_id, extension);

        // Calculate hash
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let hash = hex::encode(hasher.finalize());

        // Upload to S3
        state.s3_client.upload(&key, file_data.clone(), &mime_type).await?;

        // Image processing for thumbnails (blocking operation offloaded)
        let (width, height, thumbnail_data): (Option<i32>, Option<i32>, Option<(Vec<u8>, String)>) = 
            if mime_type.starts_with("image/") {
                let data_clone = file_data.clone();
                let user_id = auth.user_id;
                let file_id_clone = file_id;

                tokio::task::spawn_blocking(move || {
                    if let Ok(img) = image::load_from_memory(&data_clone) {
                        let (w, h) = img.dimensions();

                        // Generate thumbnail if image is large
                        let thumb = if w > 400 || h > 400 {
                            let thumb_img = img.thumbnail(400, 400);
                            let mut buf = Vec::new();
                            if thumb_img.write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP).is_ok() {
                                Some((buf, format!("thumbnails/{}/{}.webp", user_id, file_id_clone)))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        (Some(w as i32), Some(h as i32), thumb)
                    } else {
                        (None, None, None)
                    }
                })
                .await
                .unwrap_or((None, None, None))
            } else {
                (None, None, None)
            };

        // Upload thumbnail to S3 if generated
        let thumbnail_key: Option<String> = if let Some((thumb_data, thumb_key)) = thumbnail_data {
            if state.s3_client.upload(&thumb_key, thumb_data, "image/webp").await.is_ok() {
                Some(thumb_key)
            } else {
                None
            }
        } else {
            None
        };

        let has_thumbnail = thumbnail_key.is_some();

        // Insert into files table with correct schema
        sqlx::query(
            r#"
            INSERT INTO files (id, uploader_id, channel_id, name, key, mime_type, size, sha256, width, height, has_thumbnail, thumbnail_key, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(file_id)
        .bind(auth.user_id)
        .bind(session.channel_id)
        .bind(&session.filename)
        .bind(&key)
        .bind(&mime_type)
        .bind(session.file_size)
        .bind(&hash)
        .bind(width)
        .bind(height)
        .bind(has_thumbnail)
        .bind(&thumbnail_key)
        .bind(now)
        .execute(&state.db)
        .await?;

        // Delete upload session
        sqlx::query("DELETE FROM upload_sessions WHERE id = $1")
            .bind(upload_id)
            .execute(&state.db)
            .await?;

        // Return FileInfo
        let file_info = mm::FileInfo {
            id: encode_mm_id(file_id),
            user_id: encode_mm_id(auth.user_id),
            create_at: now.timestamp_millis(),
            update_at: now.timestamp_millis(),
            delete_at: 0,
            name: session.filename,
            extension,
            size: session.file_size,
            mime_type,
            width: width.unwrap_or(0),
            height: height.unwrap_or(0),
            has_preview_image: has_thumbnail,
        };

        Ok((StatusCode::CREATED, Json(serde_json::to_value(file_info).unwrap())).into_response())
    } else {
        // Upload incomplete
        Ok(StatusCode::NO_CONTENT.into_response())
    }
}
