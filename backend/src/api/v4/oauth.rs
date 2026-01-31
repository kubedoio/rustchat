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
        .route("/oauth/apps", get(get_oauth_apps).post(create_oauth_app))
        .route(
            "/oauth/apps/{app_id}",
            get(get_oauth_app).put(update_oauth_app).delete(delete_oauth_app),
        )
        .route("/oauth/apps/{app_id}/info", get(get_oauth_app_info))
        .route(
            "/oauth/apps/{app_id}/regen_secret",
            post(regenerate_oauth_app_secret),
        )
        .route("/oauth/apps/register", post(register_oauth_client))
        .route(
            "/oauth/outgoing_connections",
            get(list_outgoing_oauth_connections).post(create_outgoing_oauth_connection),
        )
        .route(
            "/oauth/outgoing_connections/{connection_id}",
            get(get_outgoing_oauth_connection)
                .put(update_outgoing_oauth_connection)
                .delete(delete_outgoing_oauth_connection),
        )
        .route(
            "/oauth/outgoing_connections/validate",
            post(validate_outgoing_oauth_connection_credentials),
        )
}

/// GET /api/v4/oauth/apps
async fn get_oauth_apps(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/oauth/apps
async fn create_oauth_app(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_app): Json<serde_json::Value>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({}))))
}

/// GET /api/v4/oauth/apps/{app_id}
async fn get_oauth_app(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_app_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// PUT /api/v4/oauth/apps/{app_id}
async fn update_oauth_app(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_app_id): Path<String>,
    Json(_app): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// DELETE /api/v4/oauth/apps/{app_id}
async fn delete_oauth_app(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_app_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/oauth/apps/{app_id}/info
async fn get_oauth_app_info(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_app_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/oauth/apps/{app_id}/regen_secret
async fn regenerate_oauth_app_secret(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_app_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/oauth/apps/register
async fn register_oauth_client(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({}))))
}

/// GET /api/v4/oauth/outgoing_connections
async fn list_outgoing_oauth_connections(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/oauth/outgoing_connections
async fn create_outgoing_oauth_connection(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_connection): Json<serde_json::Value>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({}))))
}

/// GET /api/v4/oauth/outgoing_connections/{connection_id}
async fn get_outgoing_oauth_connection(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_connection_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// PUT /api/v4/oauth/outgoing_connections/{connection_id}
async fn update_outgoing_oauth_connection(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_connection_id): Path<String>,
    Json(_connection): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// DELETE /api/v4/oauth/outgoing_connections/{connection_id}
async fn delete_outgoing_oauth_connection(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_connection_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/oauth/outgoing_connections/validate
async fn validate_outgoing_oauth_connection_credentials(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
