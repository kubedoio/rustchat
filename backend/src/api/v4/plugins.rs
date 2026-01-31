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
        .route("/plugins", get(get_plugins).post(upload_plugin))
        .route("/plugins/install_from_url", post(install_plugin_from_url))
        .route(
            "/plugins/{plugin_id}",
            get(get_plugin_status).delete(remove_plugin),
        )
        .route("/plugins/{plugin_id}/enable", post(enable_plugin))
        .route("/plugins/{plugin_id}/disable", post(disable_plugin))
        .route("/plugins/statuses", get(get_plugin_statuses))
        .route("/plugins/webapp", get(get_webapp_plugins))
        .route("/plugins/marketplace", get(get_marketplace_plugins))
        .route(
            "/plugins/marketplace/first_admin_visit",
            post(first_admin_visit_marketplace),
        )
}

/// GET /api/v4/plugins
async fn get_plugins(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"active": [], "inactive": []})))
}

/// POST /api/v4/plugins
async fn upload_plugin(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({"status": "OK"}))))
}

/// POST /api/v4/plugins/install_from_url
async fn install_plugin_from_url(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({"status": "OK"}))))
}

/// GET /api/v4/plugins/{plugin_id}
async fn get_plugin_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_plugin_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// DELETE /api/v4/plugins/{plugin_id}
async fn remove_plugin(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_plugin_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/plugins/{plugin_id}/enable
async fn enable_plugin(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_plugin_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/plugins/{plugin_id}/disable
async fn disable_plugin(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_plugin_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/plugins/statuses
async fn get_plugin_statuses(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/plugins/webapp
async fn get_webapp_plugins(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/plugins/marketplace
async fn get_marketplace_plugins(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/plugins/marketplace/first_admin_visit
async fn first_admin_visit_marketplace(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
