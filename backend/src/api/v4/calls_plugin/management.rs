use axum::{extract::State, Json};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};

pub(crate) async fn plugin_management_enable_not_implemented(
    State(_state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Insufficient permissions to manage system plugins".to_string(),
        ));
    }

    Ok(crate::api::v4::mm_not_implemented(
        "api.plugins.enable.not_implemented.app_error",
        "Plugin enable is not implemented.",
        "POST /api/v4/plugins/{plugin_id}/enable is not supported in this server.",
    ))
}

pub(crate) async fn plugin_management_disable_not_implemented(
    State(_state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Insufficient permissions to manage system plugins".to_string(),
        ));
    }

    Ok(crate::api::v4::mm_not_implemented(
        "api.plugins.disable.not_implemented.app_error",
        "Plugin disable is not implemented.",
        "POST /api/v4/plugins/{plugin_id}/disable is not supported in this server.",
    ))
}
