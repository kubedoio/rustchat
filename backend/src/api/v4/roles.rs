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
        .route("/roles", get(get_roles))
        .route("/roles/{role_id}", get(get_role))
        .route("/roles/name/{role_name}", get(get_role_by_name))
        .route("/roles/{role_id}/patch", put(patch_role))
}

/// GET /api/v4/roles
async fn get_roles(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/roles/{role_id}
async fn get_role(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_role_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/roles/name/{role_name}
async fn get_role_by_name(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_role_name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// PUT /api/v4/roles/{role_id}/patch
async fn patch_role(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_role_id): Path<String>,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}
