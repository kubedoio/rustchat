use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

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

/// GET /api/v4/saml/metadata
async fn get_saml_metadata(
    State(_state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    // Return a dummy XML metadata
    Ok((
        [("Content-Type", "application/xml")],
        "<?xml version=\"1.0\"?><EntityDescriptor xmlns=\"urn:oasis:names:tc:SAML:2.0:metadata\"></EntityDescriptor>",
    ))
}

/// POST /api/v4/saml/metadatafromidp
async fn get_saml_metadata_from_idp(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/saml/certificate/idp
async fn add_saml_idp_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// DELETE /api/v4/saml/certificate/idp
async fn remove_saml_idp_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/saml/certificate/public
async fn add_saml_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// DELETE /api/v4/saml/certificate/public
async fn remove_saml_public_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/saml/certificate/private
async fn add_saml_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// DELETE /api/v4/saml/certificate/private
async fn remove_saml_private_certificate(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/saml/certificate/status
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
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"num_affected": 0})))
}
