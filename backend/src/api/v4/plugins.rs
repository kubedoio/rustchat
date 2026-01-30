use axum::{extract::State, routing::get, Json, Router};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/plugins/webapp", get(get_webapp_plugins))
        .route("/plugins/statuses", get(get_plugin_statuses))
}

async fn get_webapp_plugins(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

async fn get_plugin_statuses(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::PluginStatus>>> {
    Ok(Json(vec![]))
}
