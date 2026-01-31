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
        .route("/ip_filtering", get(get_ip_filters))
        .route("/ip_filtering/my_ip", get(get_my_ip))
}

async fn get_ip_filters(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

async fn get_my_ip(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"ip": "127.0.0.1"})))
}
