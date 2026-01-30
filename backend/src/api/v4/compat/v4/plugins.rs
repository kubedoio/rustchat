    extract::{State},
    Json,
};
use crate::api::AppState;
use crate::error::{ApiResult};
use crate::mattermost_compat::{models as mm};

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
