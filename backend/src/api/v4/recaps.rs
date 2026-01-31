use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/recaps", get(get_recaps))
        .route("/recaps/{recap_id}", get(get_recap))
        .route("/recaps/{recap_id}/read", post(mark_recap_read))
        .route("/recaps/{recap_id}/regenerate", post(regenerate_recap))
}

/// GET /api/v4/recaps
async fn get_recaps(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/recaps/{recap_id}
async fn get_recap(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_recap_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/recaps/{recap_id}/read
async fn mark_recap_read(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_recap_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/recaps/{recap_id}/regenerate
async fn regenerate_recap(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_recap_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
