use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/groups", get(get_groups).post(create_group))
        .route(
            "/groups/{group_id}",
            get(get_group).put(patch_group).delete(delete_group),
        )
        .route("/groups/{group_id}/patch", put(patch_group))
        .route("/groups/{group_id}/restore", post(restore_group))
        .route(
            "/groups/{group_id}/{syncable_type}/{syncable_id}/link",
            post(link_group_syncable).delete(unlink_group_syncable),
        )
        .route(
            "/groups/{group_id}/{syncable_type}/{syncable_id}",
            get(get_group_syncable),
        )
        .route("/groups/{group_id}/{syncable_type}", get(get_group_syncables))
        .route(
            "/groups/{group_id}/{syncable_type}/{syncable_id}/patch",
            put(patch_group_syncable),
        )
        .route("/groups/{group_id}/stats", get(get_group_stats))
        .route("/groups/{group_id}/members", get(get_group_members).post(add_group_members).delete(delete_group_members))
        .route("/groups/names", post(get_groups_by_names))
}

/// GET /api/v4/groups
async fn get_groups(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/groups
async fn create_group(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_group): Json<serde_json::Value>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({}))))
}

/// GET /api/v4/groups/{group_id}
async fn get_group(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// PUT /api/v4/groups/{group_id}/patch
async fn patch_group(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// DELETE /api/v4/groups/{group_id}
async fn delete_group(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/groups/{group_id}/restore
async fn restore_group(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/groups/{group_id}/{syncable_type}/{syncable_id}/link
async fn link_group_syncable(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_group_id, _syncable_type, _syncable_id)): Path<(String, String, String)>,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({}))))
}

/// DELETE /api/v4/groups/{group_id}/{syncable_type}/{syncable_id}/link
async fn unlink_group_syncable(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_group_id, _syncable_type, _syncable_id)): Path<(String, String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/groups/{group_id}/{syncable_type}/{syncable_id}
async fn get_group_syncable(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_group_id, _syncable_type, _syncable_id)): Path<(String, String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/groups/{group_id}/{syncable_type}
async fn get_group_syncables(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_group_id, _syncable_type)): Path<(String, String)>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// PUT /api/v4/groups/{group_id}/{syncable_type}/{syncable_id}/patch
async fn patch_group_syncable(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_group_id, _syncable_type, _syncable_id)): Path<(String, String, String)>,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/groups/{group_id}/stats
async fn get_group_stats(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "group_id": _group_id,
        "total_member_count": 0
    })))
}

/// GET /api/v4/groups/{group_id}/members
async fn get_group_members(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "members": [],
        "count": 0
    })))
}

/// POST /api/v4/groups/{group_id}/members
async fn add_group_members(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
    Json(_members): Json<serde_json::Value>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({"status": "OK"}))))
}

/// DELETE /api/v4/groups/{group_id}/members
async fn delete_group_members(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_group_id): Path<String>,
    Json(_members): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/groups/names
async fn get_groups_by_names(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_names): Json<Vec<String>>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
