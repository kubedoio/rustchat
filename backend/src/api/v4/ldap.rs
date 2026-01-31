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
        .route("/ldap/sync", post(sync_ldap))
        .route("/ldap/test", post(test_ldap))
        .route("/ldap/test_connection", post(test_ldap_connection))
        .route("/ldap/test_diagnostics", post(test_ldap_diagnostics))
        .route("/ldap/groups", get(get_ldap_groups))
        .route("/ldap/groups/{remote_id}/link", post(link_ldap_group))
        .route("/ldap/migrateid", post(ldap_migrate_id))
        .route(
            "/ldap/certificate/public",
            post(add_ldap_public_certificate).delete(remove_ldap_public_certificate),
        )
        .route(
            "/ldap/certificate/private",
            post(add_ldap_private_certificate).delete(remove_ldap_private_certificate),
        )
        .route(
            "/ldap/users/{user_id}/group_sync_memberships",
            get(get_ldap_user_group_sync_memberships),
        )
}

/// POST /api/v4/ldap/sync
async fn sync_ldap(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/ldap/test
async fn test_ldap(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/ldap/test_connection
async fn test_ldap_connection(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/ldap/test_diagnostics
async fn test_ldap_diagnostics(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/ldap/groups
async fn get_ldap_groups(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/ldap/groups/{remote_id}/link
async fn link_ldap_group(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_remote_id): Path<String>,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok((axum::http::StatusCode::CREATED, Json(json!({"status": "OK"}))))
}

/// POST /api/v4/ldap/migrateid
async fn ldap_migrate_id(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/ldap/certificate/public
async fn add_ldap_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// DELETE /api/v4/ldap/certificate/public
async fn remove_ldap_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/ldap/certificate/private
async fn add_ldap_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// DELETE /api/v4/ldap/certificate/private
async fn remove_ldap_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/ldap/users/{user_id}/group_sync_memberships
async fn get_ldap_user_group_sync_memberships(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}
