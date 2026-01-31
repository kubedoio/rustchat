use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sharedchannels/{team_id}", get(get_shared_channels))
        .route("/sharedchannels/remote_info/{remote_id}", get(get_remote_cluster_info))
        .route("/sharedchannels/{channel_id}/remotes", get(get_channel_remotes))
        .route("/sharedchannels/users/{user_id}/can_dm/{other_user_id}", get(can_dm_user))
}

async fn get_shared_channels(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_team_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

async fn get_remote_cluster_info(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_remote_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

async fn get_channel_remotes(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

async fn can_dm_user(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_user_id, _other_user_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!(true)))
}
