use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/emoji", get(list_emoji))
        .route("/emoji/search", post(search_emoji))
        .route("/emoji/autocomplete", get(get_emoji_autocomplete))
        .route("/emoji/names", post(get_emojis_by_names))
        .route("/emoji/{emoji_id}", get(get_emoji))
        .route("/emoji/{emoji_id}/image", get(get_emoji_image))
        .route("/emoji/name/{name}", get(get_emoji_by_name))
}

#[derive(serde::Deserialize)]
pub struct EmojiSearchRequest {
    pub term: String,
}

pub async fn list_emoji(
    State(state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Emoji>>> {
    let emojis: Vec<mm::Emoji> = sqlx::query_as(
        "SELECT id::text, name, creator_id::text, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis WHERE delete_at IS NULL"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(emojis))
}

pub async fn search_emoji(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(input): Json<EmojiSearchRequest>,
) -> ApiResult<Json<Vec<mm::Emoji>>> {
    let term = format!("%{}%", input.term);
    let emojis: Vec<mm::Emoji> = sqlx::query_as(
        "SELECT id::text, name, creator_id::text, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis 
         WHERE name ILIKE $1 AND delete_at IS NULL"
    )
    .bind(term)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(emojis))
}

pub async fn get_emoji(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(emoji_id_str): Path<String>,
) -> ApiResult<Json<mm::Emoji>> {
    let emoji_id = parse_mm_or_uuid(&emoji_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid emoji_id".to_string()))?;

    let emoji: Option<mm::Emoji> = sqlx::query_as(
        "SELECT id::text, name, creator_id::text, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis WHERE id = $1 AND delete_at IS NULL"
    )
    .bind(emoji_id)
    .fetch_optional(&state.db)
    .await?;

    match emoji {
        Some(emoji) => Ok(Json(emoji)),
        None => Err(AppError::NotFound("Emoji not found".to_string())),
    }
}

pub async fn get_emoji_by_name(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(name): Path<String>,
) -> ApiResult<Json<mm::Emoji>> {
    let emoji: Option<mm::Emoji> = sqlx::query_as(
        "SELECT id::text, name, creator_id::text, 
                (extract(epoch from create_at)*1000)::bigint as create_at, 
                (extract(epoch from update_at)*1000)::bigint as update_at, 
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at 
         FROM custom_emojis WHERE name = $1 AND delete_at IS NULL"
    )
    .bind(&name)
    .fetch_optional(&state.db)
    .await?;

    if let Some(emoji) = emoji {
        return Ok(Json(emoji));
    }

    if name.chars().any(|ch| !ch.is_ascii()) {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        let hash = hasher.finalize();
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hash[..16]);
        let uuid = Uuid::from_bytes(bytes);

        let emoji = mm::Emoji {
            id: encode_mm_id(uuid),
            create_at: 0,
            update_at: 0,
            delete_at: 0,
            creator_id: encode_mm_id(Uuid::nil()),
            name,
        };

        return Ok(Json(emoji));
    }

    let emoji = emoji.ok_or_else(|| AppError::NotFound("Emoji not found".to_string()))?;

    Ok(Json(emoji))
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

    // Get the emoji's image URL from database
    let image_url: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(image_url, '') FROM custom_emojis WHERE id = $1 AND delete_at IS NULL"
    )
    .bind(emoji_id)
    .fetch_optional(&state.db)
    .await?;

    match image_url {
        Some(url) if !url.is_empty() => {
            // Redirect to the actual image
            Ok((
                axum::http::StatusCode::FOUND,
                [(axum::http::header::LOCATION, url)],
                "Redirecting to emoji image",
            ).into_response())
        }
        _ => {
            // Return a placeholder or 404
            Err(AppError::NotFound("Emoji image not found".to_string()))
        }
    }
}

/// POST /emoji/names - Get emojis by names (batch)
#[derive(serde::Deserialize)]
pub struct GetEmojisByNamesRequest {
    names: Vec<String>,
}

pub async fn get_emojis_by_names(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(input): Json<Vec<String>>,
) -> ApiResult<Json<Vec<mm::Emoji>>> {
    if input.is_empty() {
        return Ok(Json(vec![]));
    }

    let emojis: Vec<mm::Emoji> = sqlx::query_as(
        r#"
        SELECT id::text, name, creator_id::text, 
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

    Ok(Json(emojis))
}

