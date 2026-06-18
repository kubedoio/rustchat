use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::json;

pub fn router(state: AppState) -> Router<AppState> {
    let search_routes = Router::new()
        .route(
            "/access_control_policies/search",
            post(search_access_control_policies),
        )
        .route(
            "/access_control_policies/{policy_id}/resources/channels/search",
            post(search_access_control_policy_channels),
        )
        .layer(middleware::from_fn_with_state(
            state,
            crate::middleware::rate_limit::search_ip_rate_limit,
        ));

    Router::new()
        .route(
            "/access_control_policies",
            put(create_access_control_policy).get(get_access_control_policies_legacy),
        )
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
        .route(
            "/access_control_policies/cel/autocomplete/fields",
            get(get_access_control_cel_autocomplete_fields),
        )
        .route(
            "/access_control_policies/{policy_id}",
            get(get_access_control_policy).delete(delete_access_control_policy),
        )
        .route(
            "/access_control_policies/{policy_id}/activate",
            get(update_access_control_policy_active_status)
                .post(activate_access_control_policy_legacy),
        )
        .route(
            "/access_control_policies/{policy_id}/assign",
            post(assign_access_control_policy),
        )
        .route(
            "/access_control_policies/{policy_id}/unassign",
            delete(unassign_access_control_policy).post(unassign_access_control_policy_legacy),
        )
        .route(
            "/access_control_policies/{policy_id}/resources/channels",
            get(get_access_control_policy_channels),
        )
        .route(
            "/access_control_policies/cel/visual_ast",
            post(get_access_control_cel_visual_ast).get(get_access_control_cel_visual_ast_legacy),
        )
        .route(
            "/access_control_policies/activate",
            put(activate_access_control_policies).post(activate_access_control_policies_legacy),
        )
        .merge(search_routes)
}

fn ensure_manage_system(auth: &crate::api::v4::extractors::MmAuthUser) -> ApiResult<()> {
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Insufficient permissions to access access-control policies".to_string(),
        ));
    }

    Ok(())
}

/// PUT /api/v4/access_control_policies
async fn create_access_control_policy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({})))
}

/// Temporary compatibility shim for legacy RustChat GET behavior.
async fn get_access_control_policies_legacy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    ensure_manage_system(&auth)?;
    Ok(Json(vec![]))
}

/// POST /api/v4/access_control_policies/cel/check
async fn check_access_control_cel(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"allowed": true})))
}

/// POST /api/v4/access_control_policies/cel/validate_requester
async fn validate_access_control_requester(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"valid": true})))
}

/// POST /api/v4/access_control_policies/cel/test
async fn test_access_control_cel(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/access_control_policies/search
async fn search_access_control_policies(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    ensure_manage_system(&auth)?;
    Ok(Json(vec![]))
}

/// GET /api/v4/access_control_policies/cel/autocomplete/fields
async fn get_access_control_cel_autocomplete_fields(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    ensure_manage_system(&auth)?;
    Ok(Json(vec![]))
}

/// GET /api/v4/access_control_policies/{policy_id}
async fn get_access_control_policy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({})))
}

/// DELETE /api/v4/access_control_policies/{policy_id}
async fn delete_access_control_policy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/access_control_policies/{policy_id}/activate
async fn update_access_control_policy_active_status(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// Temporary compatibility shim for legacy RustChat POST behavior.
async fn activate_access_control_policy_legacy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/access_control_policies/{policy_id}/assign
async fn assign_access_control_policy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// DELETE /api/v4/access_control_policies/{policy_id}/unassign
async fn unassign_access_control_policy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// Temporary compatibility shim for legacy RustChat POST behavior.
async fn unassign_access_control_policy_legacy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/access_control_policies/{policy_id}/resources/channels
async fn get_access_control_policy_channels(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    ensure_manage_system(&auth)?;
    Ok(Json(vec![]))
}

/// POST /api/v4/access_control_policies/{policy_id}/resources/channels/search
async fn search_access_control_policy_channels(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    ensure_manage_system(&auth)?;
    Ok(Json(vec![]))
}

/// POST /api/v4/access_control_policies/cel/visual_ast
async fn get_access_control_cel_visual_ast(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({})))
}

/// Temporary compatibility shim for legacy RustChat GET behavior.
async fn get_access_control_cel_visual_ast_legacy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({})))
}

/// PUT /api/v4/access_control_policies/activate
async fn activate_access_control_policies(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}

/// Temporary compatibility shim for legacy RustChat POST behavior.
async fn activate_access_control_policies_legacy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(json!({"status": "OK"})))
}
