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

