use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/terms_of_service", get(get_tos).post(create_tos))
}

async fn get_tos(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"text": "", "id": ""})))
}

async fn create_tos(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"id": "stub_tos_id"})))
}
