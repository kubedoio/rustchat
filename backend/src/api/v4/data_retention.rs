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
        .route("/data_retention/policy", get(get_global_data_retention_policy))
        .route("/data_retention/policies_count", get(get_data_retention_policies_count))
        .route("/data_retention/policies", get(get_data_retention_policies))
        .route("/data_retention/policies/{policy_id}", get(get_data_retention_policy))
        .route("/data_retention/policies/{policy_id}/teams", get(get_teams_for_retention_policy).post(add_teams_to_retention_policy))
        .route("/data_retention/policies/{policy_id}/teams/search", post(search_teams_for_retention_policy))
        .route("/data_retention/policies/{policy_id}/channels", get(get_channels_for_retention_policy).post(add_channels_to_retention_policy))
        .route("/data_retention/policies/{policy_id}/channels/search", post(search_channels_for_retention_policy))
}

/// GET /api/v4/data_retention/policy
async fn get_global_data_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/data_retention/policies_count
async fn get_data_retention_policies_count(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"total_count": 0})))
}

/// GET /api/v4/data_retention/policies
async fn get_data_retention_policies(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"policies": [], "total_count": 0})))
}

/// GET /api/v4/data_retention/policies/{policy_id}
async fn get_data_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/data_retention/policies/{policy_id}/teams
async fn get_teams_for_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/data_retention/policies/{policy_id}/teams
async fn add_teams_to_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/data_retention/policies/{policy_id}/teams/search
async fn search_teams_for_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/data_retention/policies/{policy_id}/channels
async fn get_channels_for_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/data_retention/policies/{policy_id}/channels
async fn add_channels_to_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/data_retention/policies/{policy_id}/channels/search
async fn search_channels_for_retention_policy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_policy_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
