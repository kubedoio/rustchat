use axum::{extract::Path, routing::get, Json, Router};

use crate::api::AppState;
use crate::error::{ApiResult, AppError};

#[derive(serde::Serialize)]
struct CustomEmoji {
    id: String,
    name: String,
    creator_id: String,
    create_at: i64,
    update_at: i64,
    delete_at: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/emoji", get(list_emoji))
        .route("/emoji/autocomplete", get(emoji_autocomplete))
        .route("/emoji/name/{name}", get(get_emoji_by_name))
}

async fn list_emoji() -> ApiResult<Json<Vec<CustomEmoji>>> {
    Ok(Json(vec![]))
}

async fn emoji_autocomplete() -> ApiResult<Json<Vec<CustomEmoji>>> {
    Ok(Json(vec![]))
}

async fn get_emoji_by_name(Path(name): Path<String>) -> ApiResult<Json<CustomEmoji>> {
    Err(AppError::NotFound(format!(
        "Custom emoji '{}' not found",
        name
    )))
}
