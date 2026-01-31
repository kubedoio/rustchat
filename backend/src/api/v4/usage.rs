use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/usage/posts", get(get_usage_posts))
        .route("/usage/storage", get(get_usage_storage))
}

/// GET /api/v4/usage/posts
async fn get_usage_posts(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"count": 0})))
}

/// GET /api/v4/usage/storage
async fn get_usage_storage(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"bytes": 0})))
}
