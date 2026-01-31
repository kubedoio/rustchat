use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

/// Enterprise feature response - returns 501 Not Implemented
fn enterprise_required() -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "id": "api.license.enterprise_needed.error",
            "message": "This feature requires an Enterprise license.",
            "detailed_error": "LDAP authentication is an enterprise feature. Please upgrade your license.",
            "status_code": 501
        }))
    ))
}

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
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/ldap/test
async fn test_ldap(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/ldap/test_connection
async fn test_ldap_connection(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/ldap/test_diagnostics
async fn test_ldap_diagnostics(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// GET /api/v4/ldap/groups
async fn get_ldap_groups(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/ldap/groups/{remote_id}/link
async fn link_ldap_group(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_remote_id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/ldap/migrateid
async fn ldap_migrate_id(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/ldap/certificate/public
async fn add_ldap_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// DELETE /api/v4/ldap/certificate/public
async fn remove_ldap_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/ldap/certificate/private
async fn add_ldap_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// DELETE /api/v4/ldap/certificate/private
async fn remove_ldap_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// GET /api/v4/ldap/users/{user_id}/group_sync_memberships
async fn get_ldap_user_group_sync_memberships(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_user_id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}
