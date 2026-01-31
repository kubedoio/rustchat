use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/schemes", get(get_schemes).post(create_scheme))
        .route(
            "/schemes/{scheme_id}",
            get(get_scheme).put(patch_scheme).delete(delete_scheme),
        )
        .route("/schemes/{scheme_id}/patch", put(patch_scheme))
        .route("/schemes/{scheme_id}/teams", get(get_scheme_teams))
        .route("/schemes/{scheme_id}/channels", get(get_scheme_channels))
}

/// GET /api/v4/schemes
async fn get_schemes(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/schemes
async fn create_scheme(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_scheme): Json<serde_json::Value>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({}))))
}

/// GET /api/v4/schemes/{scheme_id}
async fn get_scheme(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_scheme_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// PUT /api/v4/schemes/{scheme_id}/patch
async fn patch_scheme(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_scheme_id): Path<String>,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// DELETE /api/v4/schemes/{scheme_id}
async fn delete_scheme(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_scheme_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/schemes/{scheme_id}/teams
async fn get_scheme_teams(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_scheme_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/schemes/{scheme_id}/channels
async fn get_scheme_channels(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_scheme_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
