use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/emoji", get(list_emoji))
        .route("/emoji/search", post(search_emoji))
        .route("/emoji/autocomplete", get(get_emoji_autocomplete))
        .route("/emoji/{emoji_id}", get(get_emoji))
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

    let emoji = emoji.ok_or_else(|| AppError::NotFound("Emoji not found".to_string()))?;

    Ok(Json(emoji))
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
    .bind(name)
    .fetch_optional(&state.db)
    .await?;

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
