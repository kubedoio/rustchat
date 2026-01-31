use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/emoji", get(list_emoji).post(create_emoji))
        .route("/emoji/search", post(search_emoji))
        .route("/emoji/autocomplete", get(get_emoji_autocomplete))
        .route("/emoji/names", post(get_emojis_by_names))
        .route(
            "/emoji/{emoji_id}",
            get(get_emoji).delete(delete_emoji),
        )
        .route("/emoji/{emoji_id}/image", get(get_emoji_image))
        .route("/emoji/name/{name}", get(get_emoji_by_name))
}

#[derive(serde::Deserialize)]
pub struct EmojiSearchRequest {
    pub term: String,
}

#[derive(sqlx::FromRow)]
struct DbEmoji {
    id: Uuid,
    name: String,
    creator_id: Uuid,
    create_at: i64,
    update_at: i64,
    delete_at: i64,
}

pub async fn list_emoji(
    State(state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Emoji>>> {
    let emojis: Vec<DbEmoji> = sqlx::query_as(
        "SELECT id, name, creator_id, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis WHERE delete_at IS NULL"
    )
    .fetch_all(&state.db)
    .await?;

    let mm_emojis: Vec<mm::Emoji> = emojis.into_iter().map(map_emoji).collect();
    Ok(Json(mm_emojis))
}

pub async fn search_emoji(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(input): Json<EmojiSearchRequest>,
) -> ApiResult<Json<Vec<mm::Emoji>>> {
    let term = format!("%{}%", input.term);
    let emojis: Vec<DbEmoji> = sqlx::query_as(
        "SELECT id, name, creator_id, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis 
         WHERE name ILIKE $1 AND delete_at IS NULL"
    )
    .bind(term)
    .fetch_all(&state.db)
    .await?;

    let mm_emojis: Vec<mm::Emoji> = emojis.into_iter().map(map_emoji).collect();
    Ok(Json(mm_emojis))
}

pub async fn get_emoji(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(emoji_id_str): Path<String>,
) -> ApiResult<Json<mm::Emoji>> {
    let emoji_id = parse_mm_or_uuid(&emoji_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid emoji_id".to_string()))?;

    let emoji: Option<DbEmoji> = sqlx::query_as(
        "SELECT id, name, creator_id, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis WHERE id = $1 AND delete_at IS NULL"
    )
    .bind(emoji_id)
    .fetch_optional(&state.db)
    .await?;

    match emoji {
        Some(emoji) => Ok(Json(map_emoji(emoji))),
        None => Err(AppError::NotFound("Emoji not found".to_string())),
    }
}

pub async fn get_emoji_by_name(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(name): Path<String>,
) -> ApiResult<Json<mm::Emoji>> {
    // First check if it's a Unicode emoji (starts with emoji codepoints)
    // Mobile client sends actual emoji characters like 👍 instead of "thumbsup"
    let is_unicode_emoji = name.chars().next()
        .map(|c| c > '\u{1F300}' || c == '❤' || c == '✅' || c == '❓' || c == '❗')
        .unwrap_or(false);
    
    if is_unicode_emoji {
        // For Unicode emojis, return a synthetic "system emoji" response
        // The emoji is valid because the user sent it - just return it as-is
        return Ok(Json(mm::Emoji {
            id: "system".to_string(),
            name: name.clone(),
            creator_id: "".to_string(),
            create_at: 0,
            update_at: 0,
            delete_at: 0,
        }));
    }
    
    // Check if it's a known system emoji name
    if crate::mattermost_compat::emoji_data::is_system_emoji(&name) {
        return Ok(Json(mm::Emoji {
            id: "system".to_string(),
            name: name.clone(),
            creator_id: "".to_string(),
            create_at: 0,
            update_at: 0,
            delete_at: 0,
        }));
    }
    
    // Check custom emojis in DB
    let emoji: Option<DbEmoji> = sqlx::query_as(
        "SELECT id, name, creator_id, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis WHERE name = $1 AND delete_at IS NULL"
    )
    .bind(&name)
    .fetch_optional(&state.db)
    .await?;

    let emoji = emoji.ok_or_else(|| AppError::NotFound("Emoji not found".to_string()))?;

    Ok(Json(map_emoji(emoji)))
}

pub async fn get_emoji_autocomplete(
    State(state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Emoji>>> {
    // For now, just return all emojis as autocomplete
    list_emoji(State(state), _auth).await
}

/// GET /emoji/{emoji_id}/image - Get emoji image
pub async fn get_emoji_image(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(emoji_id_str): Path<String>,
) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;

    let emoji_id = parse_mm_or_uuid(&emoji_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid emoji_id".to_string()))?;

    // Get the emoji's image path from database
    let image_url: Option<String> = sqlx::query_scalar(
        "SELECT image_url FROM custom_emojis WHERE id = $1 AND delete_at IS NULL",
    )
    .bind(emoji_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(key) = image_url {
        if !key.is_empty() {
            // Generate presigned URL for S3
            let url = state.s3_client.presigned_download_url(&key, 3600).await?;
            return Ok(axum::response::Redirect::temporary(&url).into_response());
        }
    }

    Err(AppError::NotFound("Emoji image not found".to_string()))
}

/// POST /emoji - Create custom emoji
pub async fn create_emoji(
    State(state): State<AppState>,
    auth: MmAuthUser,
    mut multipart: axum::extract::Multipart,
) -> ApiResult<(axum::http::StatusCode, Json<mm::Emoji>)> {
    use image::GenericImageView;
    use std::io::Cursor;

    let mut name = String::new();
    let mut image_data = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "emoji" {
            let json_str = field
                .text()
                .await
                .map_err(|_| AppError::BadRequest("Invalid emoji JSON".to_string()))?;
            let emoji_json: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|_| AppError::BadRequest("Invalid emoji JSON format".to_string()))?;
            if let Some(n) = emoji_json.get("name").and_then(|v| v.as_str()) {
                name = n.to_string();
            }
        } else if field_name == "image" {
            image_data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?
                .to_vec();
        }
    }

    if name.is_empty() {
        return Err(AppError::BadRequest("Missing emoji name".to_string()));
    }

    // Validation
    if !crate::mattermost_compat::emoji_data::is_valid_emoji_name(&name) {
        return Err(AppError::BadRequest("Invalid emoji name".to_string()));
    }

    if crate::mattermost_compat::emoji_data::is_system_emoji(&name) {
        return Err(AppError::BadRequest(
            "Emoji name already used by system emoji".to_string(),
        ));
    }

    // Check for duplicate custom emoji
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM custom_emojis WHERE name = $1 AND delete_at IS NULL)",
    )
    .bind(&name)
    .fetch_one(&state.db)
    .await?;
    if exists {
        return Err(AppError::BadRequest("Emoji name already exists".to_string()));
    }

    if image_data.is_empty() {
        return Err(AppError::BadRequest("Missing emoji image".to_string()));
    }

    // Resize image (128x128 max)
    let resized_image = tokio::task::spawn_blocking(move || -> ApiResult<Vec<u8>> {
        let img = image::load_from_memory(&image_data)
            .map_err(|_| AppError::BadRequest("Invalid image data".to_string()))?;
        let (w, h) = img.dimensions();
        if w > 128 || h > 128 {
            let thumb = img.thumbnail(128, 128);
            let mut buf = Vec::new();
            thumb
                .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
                .map_err(|_| AppError::Internal("Image processing failed".to_string()))?;
            Ok(buf)
        } else {
            Ok(image_data)
        }
    })
    .await
    .map_err(|_| AppError::Internal("Task join error".to_string()))??;

    // Upload to S3
    let emoji_id = Uuid::new_v4();
    let key = format!("emojis/{}/image.png", emoji_id);
    state
        .s3_client
        .upload(&key, resized_image, "image/png")
        .await?;

    // DB Insert
    let db_emoji: DbEmoji = sqlx::query_as(
        r#"
         INSERT INTO custom_emojis (id, name, creator_id, image_url)
         VALUES ($1, $2, $3, $4)
         RETURNING id, name, creator_id, 
                   (extract(epoch from create_at)*1000)::bigint as create_at, 
                   (extract(epoch from update_at)*1000)::bigint as update_at, 
                   COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         "#,
    )
    .bind(emoji_id)
    .bind(&name)
    .bind(auth.user_id)
    .bind(&key)
    .fetch_one(&state.db)
    .await?;

    Ok((axum::http::StatusCode::CREATED, Json(map_emoji(db_emoji))))
}

/// DELETE /emoji/{emoji_id} - Soft delete custom emoji
pub async fn delete_emoji(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(emoji_id_str): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let emoji_id = parse_mm_or_uuid(&emoji_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid emoji_id".to_string()))?;

    let emoji: DbEmoji = sqlx::query_as(
        r#"
        SELECT id, name, creator_id, 
               (extract(epoch from create_at)*1000)::bigint as create_at, 
               (extract(epoch from update_at)*1000)::bigint as update_at, 
               COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
        FROM custom_emojis WHERE id = $1 AND delete_at IS NULL
        "#,
    )
    .bind(emoji_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Emoji not found".to_string()))?;

    // Authorization
    if emoji.creator_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Cannot delete emoji created by another user".to_string(),
        ));
    }

    // Soft delete
    sqlx::query("UPDATE custom_emojis SET delete_at = NOW() WHERE id = $1")
        .bind(emoji_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn get_emojis_by_names(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(input): Json<Vec<String>>,
) -> ApiResult<Json<Vec<mm::Emoji>>> {
    if input.is_empty() {
        return Ok(Json(vec![]));
    }

    let emojis: Vec<DbEmoji> = sqlx::query_as(
        r#"
        SELECT id, name, creator_id, 
               (extract(epoch from create_at)*1000)::bigint as create_at, 
               (extract(epoch from update_at)*1000)::bigint as update_at, 
               COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
        FROM custom_emojis 
        WHERE name = ANY($1) AND delete_at IS NULL
        "#
    )
    .bind(&input)
    .fetch_all(&state.db)
    .await?;

    let mm_emojis: Vec<mm::Emoji> = emojis.into_iter().map(map_emoji).collect();
    Ok(Json(mm_emojis))
}

fn map_emoji(emoji: DbEmoji) -> mm::Emoji {
    mm::Emoji {
        id: encode_mm_id(emoji.id),
        create_at: emoji.create_at,
        update_at: emoji.update_at,
        delete_at: emoji.delete_at,
        creator_id: encode_mm_id(emoji.creator_id),
        name: emoji.name,
    }
}

