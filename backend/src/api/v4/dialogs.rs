use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/actions/dialogs/open", post(open_dialog))
        .route("/actions/dialogs/submit", post(submit_dialog))
        .route("/actions/dialogs/lookup", post(lookup_dialog))
}

async fn open_dialog(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

async fn submit_dialog(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

async fn lookup_dialog(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}
