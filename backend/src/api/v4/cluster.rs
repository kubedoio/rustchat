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
        .route("/cluster/status", get(get_cluster_status))
        .route("/remotecluster", get(get_remote_clusters))
        .route("/remotecluster/{remote_id}", get(get_remote_cluster))
        .route(
            "/remotecluster/{remote_id}/generate_invite",
            post(generate_remote_cluster_invite),
        )
        .route("/remotecluster/accept_invite", post(accept_remote_cluster_invite))
        .route(
            "/remotecluster/{remote_id}/sharedchannelremotes",
            get(get_remote_cluster_shared_channels),
        )
        .route(
            "/remotecluster/{remote_id}/channels/{channel_id}/invite",
            post(invite_remote_cluster_to_channel),
        )
        .route(
            "/remotecluster/{remote_id}/channels/{channel_id}/uninvite",
            post(uninvite_remote_cluster_from_channel),
        )
}

/// GET /api/v4/cluster/status
async fn get_cluster_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/remotecluster
async fn get_remote_clusters(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/remotecluster/{remote_id}
async fn get_remote_cluster(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_remote_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/remotecluster/{remote_id}/generate_invite
async fn generate_remote_cluster_invite(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_remote_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"invite_token": ""})))
}

/// POST /api/v4/remotecluster/accept_invite
async fn accept_remote_cluster_invite(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/remotecluster/{remote_id}/sharedchannelremotes
async fn get_remote_cluster_shared_channels(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_remote_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/remotecluster/{remote_id}/channels/{channel_id}/invite
async fn invite_remote_cluster_to_channel(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_remote_id, _channel_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/remotecluster/{remote_id}/channels/{channel_id}/uninvite
async fn uninvite_remote_cluster_from_channel(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_remote_id, _channel_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
