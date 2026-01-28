use axum::{routing::get, Json, Router};

use crate::api::AppState;
use crate::error::ApiResult;

pub fn router() -> Router<AppState> {
    Router::new().route("/plugins/webapp", get(get_webapp_plugins))
}

async fn get_webapp_plugins() -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
