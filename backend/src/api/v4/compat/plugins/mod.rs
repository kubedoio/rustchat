use axum::Json;
use crate::error::ApiResult;

pub async fn get_plugins() -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
