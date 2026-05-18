use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use image::{GenericImageView, ImageFormat};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::FileInfo;
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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/files", post(upload_file))
        .route("/files/{file_id}", get(get_file))
        .route("/files/{file_id}/info", get(get_file_info))
        .route("/files/{file_id}/thumbnail", get(get_thumbnail))
        .route("/files/{file_id}/preview", get(get_preview))
        .route("/files/{file_id}/link", get(get_link))
        .route("/files/search", post(search_files_global))
        .route("/teams/{team_id}/files/search", post(search_files_for_team))
}

fn filename_extension(filename: &str) -> Option<&str> {
    filename
        .rsplit_once('.')
        .and_then(|(_, ext)| if ext.is_empty() { None } else { Some(ext) })
}

async fn upload_file(
    State(state): State<AppState>,
    auth: MmAuthUser,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let mut channel_id: Option<Uuid> = None;
    let mut client_ids: Vec<String> = Vec::new();

    struct TempFile(std::path::PathBuf);
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    struct PendingFile {
        filename: String,
        temp_path: std::path::PathBuf,
        size: u64,
        hash: String,
        head: Vec<u8>,
    }

    let mut pending_files: Vec<PendingFile> = Vec::new();

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "channel_id" {
            let txt = field.text().await.unwrap_or_default();
            if let Some(id) = parse_mm_or_uuid(&txt) {
                channel_id = Some(id);
            }
        } else if name == "client_ids" {
            let txt = field.text().await.unwrap_or_default();
            client_ids.push(txt);
        } else if !name.is_empty() {
            // Accept multiple field names: "files", "file", "attachment", or unnamed
            // React Native network client may use different field names
            if field.file_name().is_some() || field.content_type().is_some() {
                let filename = field.file_name().unwrap_or("unknown").to_string();

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

                pending_files.push(PendingFile {
                    filename,
                    temp_path,
                    size,
                    hash: hex::encode(hasher.finalize()),
                    head,
                });
            }
        }
    }

    // Enforce channel membership before associating files with a channel
    if let Some(cid) = channel_id {
        let repo = ChannelRepository::new(&state.db);
        let is_member = repo.is_channel_member(cid, auth.user_id).await?;
        if !is_member {
            return Err(AppError::Forbidden(
                "You are not a member of this channel".to_string(),
            ));
        }
    }

    let mut file_infos: Vec<mm::FileInfo> = Vec::new();

    for file in pending_files {
        let filename = file.filename.clone();
        let (content_type, extension) = crate::api::file_validation::validate_file_upload_head(
            &filename,
            &file.head,
            file.size as usize,
        )?;

        // Full validation for SVG/text files that require entire file content
        if matches!(extension.as_str(), "svg" | "txt" | "md") {
            let data = tokio::fs::read(&file.temp_path)
                .await
                .map_err(|e| AppError::Internal(format!("Read temp file: {}", e)))?;
            let _ = crate::api::file_validation::validate_file_upload(&filename, &data)?;
        }

        let file_id = Uuid::new_v4();
        let key = if extension.is_empty() {
            format!("files/{}/{}", auth.user_id, file_id)
        } else {
            format!("files/{}/{}.{}", auth.user_id, file_id, extension)
        };

        let hash = file.hash;
        let size = file.size as i64;

        let mut temp_guard = TempFile(file.temp_path);
        state
            .s3_client
            .upload_file(&key, &temp_guard.0, &content_type)
            .await?;

        // Image processing (Blocking offloaded)
        let (width, height, thumbnail_data, preview_data) = if content_type.starts_with("image/") {
            let path = std::mem::take(&mut temp_guard.0);
            std::mem::forget(temp_guard);
            tokio::task::spawn_blocking(move || {
                let result = if let Ok(img) = image::open(&path) {
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
                        // For small images, generate JPEG thumbnail for consistency
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
                        // If smaller than 1024, generate JPEG preview for consistency
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
                };
                let _ = std::fs::remove_file(&path);
                result
            })
            .await
            .unwrap_or((None, None, None, None))
        } else {
            (None, None, None, None)
        };

        let mut thumbnail_key = None;
        if let Some(t_data) = thumbnail_data {
            // Store as .jpg with image/jpeg content type for Mattermost mobile compatibility
            let t_key = format!("thumbnails/{}/{}.jpg", auth.user_id, file_id);
            if state
                .s3_client
                .upload(&t_key, t_data, "image/jpeg")
                .await
                .is_ok()
            {
                thumbnail_key = Some(t_key);
            }
        }

        // Store preview as JPEG for Mattermost mobile compatibility
        if let Some(p_data) = preview_data {
            let p_key = format!("previews/{}/{}.jpg", auth.user_id, file_id);
            let _ = state.s3_client.upload(&p_key, p_data, "image/jpeg").await;
        }

        let has_thumbnail = thumbnail_key.is_some();

        let file_repo = FileRepository::new(&state.db);
        let _file_info = file_repo
            .create_full(
                file_id,
                auth.user_id,
                channel_id,
                &filename,
                &key,
                &content_type,
                size,
                &hash,
                width,
                height,
                has_thumbnail,
                thumbnail_key.as_deref(),
            )
            .await?;

        file_infos.push(mm::FileInfo {
            id: encode_mm_id(file_id),
            user_id: encode_mm_id(auth.user_id),
            post_id: "".to_string(),
            channel_id: channel_id.map(encode_mm_id).unwrap_or_default(),
            create_at: Utc::now().timestamp_millis(),
            update_at: Utc::now().timestamp_millis(),
            delete_at: 0,
            name: filename,
            extension,
            size,
            mime_type: content_type,
            width: width.unwrap_or(0),
            height: height.unwrap_or(0),
            has_preview_image: has_thumbnail,
            mini_preview: None,
        });
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "file_infos": file_infos,
            "client_ids": client_ids
        })),
    ))
}

async fn get_file(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(file_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let file_id = parse_mm_or_uuid(&file_id)
        .ok_or_else(|| AppError::BadRequest("Invalid file_id".to_string()))?;
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(file_id)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;
    check_file_access(&state, &file, auth.user_id).await?;

    let stream = state.s3_client.download_stream(&file.key).await?;

    Ok((
        [
            (header::CONTENT_TYPE, file.mime_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{}\"", file.name),
            ),
            (
                header::CACHE_CONTROL,
                "max-age=2592000, private".to_string(),
            ),
            // Security headers matching Mattermost server
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

/// GET /files/{file_id}/info - Get file metadata
async fn get_file_info(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(file_id): Path<String>,
) -> ApiResult<Json<mm::FileInfo>> {
    let file_id = parse_mm_or_uuid(&file_id)
        .ok_or_else(|| AppError::BadRequest("Invalid file_id".to_string()))?;
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(file_id)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;
    check_file_access(&state, &file, auth.user_id).await?;

    // Get file extension from name
    let extension = filename_extension(&file.name)
        .unwrap_or_default()
        .to_string();

    Ok(Json(mm::FileInfo {
        id: encode_mm_id(file.id),
        user_id: encode_mm_id(file.uploader_id),
        post_id: file.post_id.map(encode_mm_id).unwrap_or_default(),
        channel_id: file.channel_id.map(encode_mm_id).unwrap_or_default(),
        create_at: file.created_at.timestamp_millis(),
        update_at: file.created_at.timestamp_millis(),
        delete_at: 0,
        name: file.name,
        extension,
        size: file.size,
        mime_type: file.mime_type,
        width: file.width.unwrap_or(0),
        height: file.height.unwrap_or(0),
        has_preview_image: file.has_thumbnail,
        mini_preview: None,
    }))
}

async fn get_thumbnail(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(file_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let file_id = parse_mm_or_uuid(&file_id)
        .ok_or_else(|| AppError::BadRequest("Invalid file_id".to_string()))?;
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(file_id)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;
    check_file_access(&state, &file, auth.user_id).await?;

    if file.has_thumbnail {
        if let Some(key) = file.thumbnail_key {
            let stream = state.s3_client.download_stream(&key).await?;
            let content_type = if key.ends_with(".webp") {
                "image/webp"
            } else {
                "image/jpeg"
            };
            return Ok((
                [
                    (header::CONTENT_TYPE, content_type.to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"thumb_{}\"", file.name),
                    ),
                    (
                        header::CACHE_CONTROL,
                        "max-age=2592000, private".to_string(),
                    ),
                    // Security headers
                    (
                        header::HeaderName::from_static("x-content-type-options"),
                        "nosniff".to_string(),
                    ),
                ],
                Body::new(stream.into_inner()),
            )
                .into_response());
        }
    }

    // Fallback to original if no thumbnail or just 404?
    // MM returns 404 if no thumbnail.
    Err(AppError::NotFound("Thumbnail not found".to_string()))
}

async fn get_preview(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(file_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let file_id = parse_mm_or_uuid(&file_id)
        .ok_or_else(|| AppError::BadRequest("Invalid file_id".to_string()))?;
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(file_id)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;
    check_file_access(&state, &file, auth.user_id).await?;

    if file.mime_type.starts_with("image/") {
        // Derive preview key from convention (now using .jpg for JPEG format)
        let preview_key = format!("previews/{}/{}.jpg", file.uploader_id, file.id);
        if let Ok(Some(stream)) = state.s3_client.download_stream_optional(&preview_key).await {
            return Ok((
                [
                    // Use image/jpeg for Mattermost mobile compatibility
                    (header::CONTENT_TYPE, "image/jpeg".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"preview_{}\"", file.name),
                    ),
                    (
                        header::CACHE_CONTROL,
                        "max-age=2592000, private".to_string(),
                    ),
                    // Security headers
                    (
                        header::HeaderName::from_static("x-content-type-options"),
                        "nosniff".to_string(),
                    ),
                ],
                Body::new(stream.into_inner()),
            )
                .into_response());
        }

        // If preview not found but thumbnail exists, try to serve thumbnail as fallback
        if let Some(thumb_key) = &file.thumbnail_key {
            if let Ok(Some(stream)) = state.s3_client.download_stream_optional(thumb_key).await {
                let content_type = if thumb_key.ends_with(".webp") {
                    "image/webp"
                } else {
                    "image/jpeg"
                };
                return Ok((
                    [
                        (header::CONTENT_TYPE, content_type.to_string()),
                        (
                            header::CONTENT_DISPOSITION,
                            format!("inline; filename=\"preview_{}\"", file.name),
                        ),
                        (
                            header::CACHE_CONTROL,
                            "max-age=2592000, private".to_string(),
                        ),
                        (
                            header::HeaderName::from_static("x-content-type-options"),
                            "nosniff".to_string(),
                        ),
                    ],
                    Body::new(stream.into_inner()),
                )
                    .into_response());
            }
        }
    }

    // If we can't serve a preview image, return 404 or 400.
    // Mattermost returns 404 if no preview (e.g. non-images).
    // Redirecting non-images to S3 presigned URL for "preview" endpoint confuses mobile client
    // because it expects an image or an error.
    Err(AppError::NotFound("Preview not available".to_string()))
}

async fn get_link(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(file_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let file_id = parse_mm_or_uuid(&file_id)
        .ok_or_else(|| AppError::BadRequest("Invalid file_id".to_string()))?;
    let file_repo = FileRepository::new(&state.db);
    let file = file_repo
        .get_by_id(file_id)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;
    check_file_access(&state, &file, auth.user_id).await?;

    let url = state
        .s3_client
        .presigned_download_url(&file.key, 3600)
        .await?;

    Ok(Json(serde_json::json!({"link": url})))
}

#[derive(serde::Deserialize)]
pub struct FileSearchParams {
    terms: String,
    #[serde(default)]
    _is_or_search: bool,
}

/// POST /files/search - Search files globally
async fn search_files_global(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(params): Json<FileSearchParams>,
) -> ApiResult<Json<FileSearchResult>> {
    search_files_impl(&state, auth.user_id, None, &params.terms).await
}

/// POST /teams/{team_id}/files/search - Search files within a team
async fn search_files_for_team(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Json(params): Json<FileSearchParams>,
) -> ApiResult<Json<FileSearchResult>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    search_files_impl(&state, auth.user_id, Some(team_id), &params.terms).await
}

#[derive(serde::Serialize)]
pub struct FileSearchResult {
    order: Vec<String>,
    file_infos: std::collections::HashMap<String, mm::FileInfo>,
}

async fn search_files_impl(
    state: &AppState,
    user_id: Uuid,
    team_id: Option<Uuid>,
    terms: &str,
) -> ApiResult<Json<FileSearchResult>> {
    let search_pattern = format!("%{}%", terms);

    let file_repo = FileRepository::new(&state.db);
    let files = if let Some(tid) = team_id {
        file_repo
            .search_for_team(user_id, tid, &search_pattern)
            .await?
    } else {
        file_repo.search(user_id, &search_pattern).await?
    };

    let mut order = Vec::new();
    let mut file_infos = std::collections::HashMap::new();

    for file in files {
        let id = encode_mm_id(file.id);
        order.push(id.clone());

        let extension = filename_extension(&file.name)
            .unwrap_or_default()
            .to_string();

        file_infos.insert(
            id.clone(),
            mm::FileInfo {
                id,
                user_id: encode_mm_id(file.uploader_id),
                post_id: file.post_id.map(encode_mm_id).unwrap_or_default(),
                channel_id: file.channel_id.map(encode_mm_id).unwrap_or_default(),
                create_at: file.created_at.timestamp_millis(),
                update_at: file.created_at.timestamp_millis(),
                delete_at: 0,
                name: file.name,
                extension,
                size: file.size,
                mime_type: file.mime_type,
                width: file.width.unwrap_or(0),
                height: file.height.unwrap_or(0),
                has_preview_image: file.has_thumbnail,
                mini_preview: None,
            },
        );
    }

    Ok(Json(FileSearchResult { order, file_infos }))
}
