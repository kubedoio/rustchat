use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};

use super::helpers::{check_channel_permission, resolve_channel_id};
use super::lifecycle::StatusResponse;

/// POST /plugins/com.mattermost.calls/calls/{channel_id}/recording/start
pub(crate) async fn start_recording(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_or_call_uuid = resolve_channel_id(&state, &channel_id).await?;
    let call = match state
        .call_state_manager
        .get_call_by_channel(&channel_or_call_uuid)
        .await
    {
        Some(c) => Some(c),
        None => {
            state
                .call_state_manager
                .get_call(channel_or_call_uuid)
                .await
        }
    };
    if let Some(call) = call {
        check_channel_permission(&state, auth.user_id, call.channel_id).await?;
    }
    Err(AppError::BadRequest(
        "Call recording is not supported by this server".to_string(),
    ))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/recording/stop
pub(crate) async fn stop_recording(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_or_call_uuid = resolve_channel_id(&state, &channel_id).await?;
    let call = match state
        .call_state_manager
        .get_call_by_channel(&channel_or_call_uuid)
        .await
    {
        Some(c) => Some(c),
        None => {
            state
                .call_state_manager
                .get_call(channel_or_call_uuid)
                .await
        }
    };
    if let Some(call) = call {
        check_channel_permission(&state, auth.user_id, call.channel_id).await?;
    }
    Err(AppError::BadRequest(
        "Call recording is not supported by this server".to_string(),
    ))
}
