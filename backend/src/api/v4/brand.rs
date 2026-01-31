use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/brand/image", get(get_brand_image).post(upload_brand_image).delete(delete_brand_image))
}

/// GET /api/v4/brand/image
async fn get_brand_image(
    State(_state): State<AppState>,
) -> ApiResult<axum::response::Response> {
    Ok((axum::http::StatusCode::NOT_FOUND, "No brand image").into_response())
}

/// POST /api/v4/brand/image
async fn upload_brand_image(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({"status": "OK"}))))
}

/// DELETE /api/v4/brand/image
async fn delete_brand_image(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
