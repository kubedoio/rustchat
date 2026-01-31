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
        .route("/access_control_policies", get(get_access_control_policies))
        .route(
            "/access_control_policies/cel/check",
            post(check_access_control_cel),
        )
        .route(
            "/access_control_policies/cel/validate_requester",
            post(validate_access_control_requester),
        )
        .route(
            "/access_control_policies/cel/test",
            post(test_access_control_cel),
        )
        .route("/access_control_policies/search", post(search_access_control_policies))
        .route(
            "/access_control_policies/cel/autocomplete/fields",
            get(get_access_control_cel_autocomplete_fields),
        )
        .route("/access_control_policies/{policy_id}", get(get_access_control_policy))
        .route(
            "/access_control_policies/{policy_id}/activate",
            post(activate_access_control_policy),
        )
        .route(
            "/access_control_policies/{policy_id}/assign",
            post(assign_access_control_policy),
        )
        .route(
            "/access_control_policies/{policy_id}/unassign",
            post(unassign_access_control_policy),
        )
        .route(
            "/access_control_policies/{policy_id}/resources/channels",
            get(get_access_control_policy_channels),
        )
        .route(
            "/access_control_policies/{policy_id}/resources/channels/search",
            post(search_access_control_policy_channels),
        )
        .route(
            "/access_control_policies/cel/visual_ast",
            get(get_access_control_cel_visual_ast),
        )
        .route("/access_control_policies/activate", post(activate_access_control_policies))
}

/// GET /api/v4/access_control_policies
async fn get_access_control_policies(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/access_control_policies/cel/check
async fn check_access_control_cel(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"allowed": true})))
}

/// POST /api/v4/access_control_policies/cel/validate_requester
async fn validate_access_control_requester(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"valid": true})))
}

/// POST /api/v4/access_control_policies/cel/test
async fn test_access_control_cel(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/access_control_policies/search
async fn search_access_control_policies(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/access_control_policies/cel/autocomplete/fields
async fn get_access_control_cel_autocomplete_fields(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/access_control_policies/{policy_id}
async fn get_access_control_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/access_control_policies/{policy_id}/activate
async fn activate_access_control_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/access_control_policies/{policy_id}/assign
async fn assign_access_control_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/access_control_policies/{policy_id}/unassign
async fn unassign_access_control_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/access_control_policies/{policy_id}/resources/channels
async fn get_access_control_policy_channels(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/access_control_policies/{policy_id}/resources/channels/search
async fn search_access_control_policy_channels(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/access_control_policies/cel/visual_ast
async fn get_access_control_cel_visual_ast(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/access_control_policies/activate
async fn activate_access_control_policies(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
