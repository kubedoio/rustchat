use axum::Json;
use crate::error::ApiResult;

pub async fn get_client_config() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "Version": "5.37.0",
        "SiteName": "RustChat",
        "EnableDiagnostics": "false"
    })))
}
