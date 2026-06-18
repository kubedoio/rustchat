use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

pub fn router(state: AppState) -> Router<AppState> {
    let search_routes = Router::new()
        .route(
            "/data_retention/policies/{policy_id}/teams/search",
            post(search_teams_for_retention_policy),
        )
        .route(
            "/data_retention/policies/{policy_id}/channels/search",
            post(search_channels_for_retention_policy),
        )
        .layer(middleware::from_fn_with_state(
            state,
            crate::middleware::rate_limit::search_ip_rate_limit,
        ));

    Router::new()
        .route(
            "/data_retention/policy",
            get(get_global_data_retention_policy),
        )
        .route(
            "/data_retention/policies_count",
            get(get_data_retention_policies_count),
        )
        .route("/data_retention/policies", get(get_data_retention_policies))
        .route(
            "/data_retention/policies/{policy_id}",
            get(get_data_retention_policy),
        )
        .route(
            "/data_retention/policies/{policy_id}/teams",
            get(get_teams_for_retention_policy).post(add_teams_to_retention_policy),
        )
        .route(
            "/data_retention/policies/{policy_id}/channels",
            get(get_channels_for_retention_policy).post(add_channels_to_retention_policy),
        )
        // User-specific data retention policies (for mobile compatibility)
        .route(
            "/users/{user_id}/data_retention/team_policies",
            get(get_user_team_policies),
        )
        .route(
            "/users/{user_id}/data_retention/channel_policies",
            get(get_user_channel_policies),
        )
        // NPS endpoint stub
        .route("/nps", post(submit_nps))
        .merge(search_routes)
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

/// GET /users/{user_id}/data_retention/team_policies - User's team retention policies
async fn get_user_team_policies(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"policies": [], "total_count": 0})))
}

/// GET /users/{user_id}/data_retention/channel_policies - User's channel retention policies
async fn get_user_channel_policies(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"policies": [], "total_count": 0})))
}

/// POST /nps - Submit NPS feedback
async fn submit_nps(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_feedback): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
