use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use super::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use crate::constants::MAX_IMAGE_SIZE;
use crate::models::legacy_avatar_key_from_url;

/// GET /users/{user_id}/image - Get user profile image (requires auth)
pub async fn get_user_image(
    State(state): State<AppState>,
    _auth: MmAuthUser, // Require authentication - images are only accessible to logged-in users
    Path(user_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let user_uuid = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;

    // Try to fetch from S3
    let key = format!("avatars/{}.png", user_uuid);
    let mut data = state.s3_client.download_optional(&key).await?;

    if data.is_none() {
        let legacy_avatar_url: Option<String> =
            sqlx::query_scalar("SELECT avatar_url FROM users WHERE id = $1")
                .bind(user_uuid)
                .fetch_optional(&state.db)
                .await?
                .flatten();

        let legacy_key = legacy_avatar_url
            .as_deref()
            .and_then(|url| legacy_avatar_key_from_url(user_uuid, url));

        if let Some(ref key) = legacy_key {
            data = state.s3_client.download_optional(key).await?;
        }

        tracing::info!(
            user_id = %user_uuid,
            has_legacy_key = legacy_key.is_some(),
            legacy_hit = data.is_some(),
            "Avatar resolution: canonical miss, tried legacy fallback"
        );
    }

    match data {
        Some(bytes) => {
            // Detect content type from magic bytes
            let content_type = if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                "image/png"
            } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
                "image/jpeg"
            } else if bytes.starts_with(b"GIF") {
                "image/gif"
            } else if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
                "image/webp"
            } else {
                "image/png"
            };

            Ok((
                [
                    (axum::http::header::CONTENT_TYPE, content_type),
                    (axum::http::header::CACHE_CONTROL, "private, max-age=86400"),
                ],
                bytes,
            )
                .into_response())
        }
        None => {
            tracing::info!(
                user_id = %user_uuid,
                "Avatar resolution: returning default image"
            );
            // Return default 1x1 transparent PNG if no image uploaded
            const PNG_1X1: &[u8] = &[
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
                1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0,
                1, 0, 0, 5, 0, 1, 13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
            ];

            Ok((
                [
                    (axum::http::header::CONTENT_TYPE, "image/png"),
                    (axum::http::header::CACHE_CONTROL, "private, max-age=86400"),
                ],
                PNG_1X1.to_vec(),
            )
                .into_response())
        }
    }
}

/// POST /users/{user_id}/image - Upload user profile image
pub async fn upload_user_image(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let user_uuid = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;

    if user_uuid != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot update other user's image".to_string(),
        ));
    }

    // Process multipart upload
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

        // Accept field named "image", "file", "picture", "avatar", or any field with:
        // - image content type
        // - a filename present (indicates it's a file upload)
        let is_image_field = name == "image"
            || name == "file"
            || name == "picture"
            || name == "avatar"
            || name.is_empty();
        let is_image_type = content_type.starts_with("image/");
        let has_filename = filename.is_some();

        if is_image_field && (is_image_type || has_filename) {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?
                .to_vec();

            if data.is_empty() {
                continue;
            }

            if data.len() > MAX_IMAGE_SIZE {
                return Err(AppError::Validation(format!(
                    "Image too large: {} bytes (max {})",
                    data.len(),
                    MAX_IMAGE_SIZE
                )));
            }

            // Determine content type from data if not provided
            let final_content_type = if is_image_type {
                content_type.clone()
            } else {
                // Try to detect from magic bytes
                if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                    "image/png".to_string()
                } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    "image/jpeg".to_string()
                } else if data.starts_with(b"GIF") {
                    "image/gif".to_string()
                } else if data.starts_with(b"RIFF") && data.len() > 12 && &data[8..12] == b"WEBP" {
                    "image/webp".to_string()
                } else {
                    "image/png".to_string() // default to PNG
                }
            };

            // Upload to S3
            let key = format!("avatars/{}.png", user_uuid);
            state
                .s3_client
                .upload(&key, data, &final_content_type)
                .await?;

            // Update user avatar_url
            let avatar_url = format!("/api/v4/users/{}/image", encode_mm_id(user_uuid));
            sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
                .bind(&avatar_url)
                .bind(user_uuid)
                .execute(&state.db)
                .await?;

            return Ok(Json(serde_json::json!({"status": "OK"})));
        }
    }

    Err(AppError::BadRequest(
        "No image field found in upload".to_string(),
    ))
}

pub async fn get_user_image_default(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    get_user_image(State(state), auth, Path(user_id)).await
}
