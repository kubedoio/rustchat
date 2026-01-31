use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
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
            "detailed_error": "SAML authentication is an enterprise feature. Please upgrade your license.",
            "status_code": 501
        }))
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/saml/metadata", get(get_saml_metadata))
        .route("/saml/metadatafromidp", post(get_saml_metadata_from_idp))
        .route(
            "/saml/certificate/idp",
            post(add_saml_idp_certificate).delete(remove_saml_idp_certificate),
        )
        .route(
            "/saml/certificate/public",
            post(add_saml_public_certificate).delete(remove_saml_public_certificate),
        )
        .route(
            "/saml/certificate/private",
            post(add_saml_private_certificate).delete(remove_saml_private_certificate),
        )
        .route("/saml/certificate/status", get(get_saml_certificate_status))
        .route("/saml/reset_auth_data", post(reset_saml_auth_data))
}

/// GET /api/v4/saml/metadata - returns empty XML indicating SAML not configured
async fn get_saml_metadata(
    State(_state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        "<?xml version=\"1.0\"?><EntityDescriptor xmlns=\"urn:oasis:names:tc:SAML:2.0:metadata\"><Error>SAML not configured - Enterprise license required</Error></EntityDescriptor>",
    ))
}

/// POST /api/v4/saml/metadatafromidp
async fn get_saml_metadata_from_idp(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/saml/certificate/idp
async fn add_saml_idp_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// DELETE /api/v4/saml/certificate/idp
async fn remove_saml_idp_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/saml/certificate/public
async fn add_saml_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// DELETE /api/v4/saml/certificate/public
async fn remove_saml_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// POST /api/v4/saml/certificate/private
async fn add_saml_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// DELETE /api/v4/saml/certificate/private
async fn remove_saml_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}

/// GET /api/v4/saml/certificate/status - returns disabled status
async fn get_saml_certificate_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "idp_certificate_file": false,
        "public_certificate_file": false,
        "private_key_file": false
    })))
}

/// POST /api/v4/saml/reset_auth_data
async fn reset_saml_auth_data(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    enterprise_required()
}
