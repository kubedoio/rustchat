use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};

pub async fn get_webapp_plugins(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Return empty list as RustChat doesn't have a webapp plugin system compatible with MM yet
    Ok(Json(vec![]))
}

pub async fn get_plugin_statuses(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::PluginStatus>>> {
    // Return empty list or basic status
    Ok(Json(vec![]))
}
