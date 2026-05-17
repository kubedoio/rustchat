use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::encode_mm_id;

use super::broadcast::{broadcast_call_event, broadcast_call_state_event, broadcast_ringing_event};
use super::helpers::{check_channel_permission, resolve_channel_id};
use super::lifecycle::StatusResponse;
use super::posts::ensure_call_thread_id;

/// POST /plugins/com.mattermost.calls/calls/{channel_id}/ring
/// Send ringing notification to all channel participants
pub(crate) async fn ring_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    check_channel_permission(&state, auth.user_id, channel_uuid).await?;

    // Check if call exists
    let call_manager = state.call_state_manager.as_ref();
    let call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call to ring".to_string()))?;

    let thread_id = ensure_call_thread_id(&state, &call).await;

    // Mattermost-mobile compatibility: stock mobile clients trigger incoming call UX
    // from calls_call_start and do not handle calls_ringing directly.
    // Note: thread_id is used as post_id for call posts in Mattermost
    let thread_id_str = thread_id.map(encode_mm_id).unwrap_or_default();
    // Fetch caller info for better mobile client support
    let caller_info: Option<(String, String)> = sqlx::query_as(
        "SELECT username, COALESCE(display_name, '') as display_name FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let (username, display_name) =
        caller_info.unwrap_or_else(|| (encode_mm_id(auth.user_id), String::new()));

    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_call_start",
        &channel_uuid,
        serde_json::json!({
            "id": encode_mm_id(call.call_id),
            "channelID": encode_mm_id(channel_uuid),
            "start_at": call.started_at,
            "owner_id": encode_mm_id(call.owner_id),
            "host_id": encode_mm_id(call.host_id),
            "thread_id": thread_id_str.clone(),
            "post_id": thread_id_str,  // Mobile expects post_id for navigation
            "call_id": encode_mm_id(call.call_id),
            "channel_id": encode_mm_id(channel_uuid),
            "user_id": encode_mm_id(call.owner_id),
            "sender_id": encode_mm_id(auth.user_id),
            "caller_name": if display_name.is_empty() { username } else { display_name },
        }),
        Some(auth.user_id),
    )
    .await;

    broadcast_ringing_event(&state, channel_uuid, call.call_id, auth.user_id, None).await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/dismiss-notification
/// Dismiss incoming call ringing notification
pub(crate) async fn dismiss_notification(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    check_channel_permission(&state, auth.user_id, channel_uuid).await?;
    let call = state
        .call_state_manager
        .get_call_by_channel(&channel_uuid)
        .await;
    let call_id = if let Some(call) = call {
        state
            .call_state_manager
            .dismiss_user_notification(call.call_id, auth.user_id)
            .await;
        encode_mm_id(call.call_id)
    } else {
        String::new()
    };

    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_dismissed_notification",
        &channel_uuid,
        serde_json::json!({
            "userID": encode_mm_id(auth.user_id),
            "user_id": encode_mm_id(auth.user_id),
            "callID": call_id,
            "call_id": call_id,
        }),
        None,
    )
    .await;
    broadcast_call_state_event(&state, channel_uuid, None).await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
