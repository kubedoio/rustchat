use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

use super::broadcast::{broadcast_call_event, broadcast_call_state_event, broadcast_host_changed_event, broadcast_screen_share_event};
use super::helpers::resolve_channel_id;
use super::lifecycle::StatusResponse;
use super::state_helpers::{can_manage_call, normalize_call_host_if_stale, reconcile_after_participant_left, schedule_empty_call_timeout};

#[derive(Debug, Deserialize)]
pub(crate) struct HostControlRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HostMakeRequest {
    new_host_id: String,
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/host/screen-off
pub(crate) async fn host_screen_off(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<HostControlRequest>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    let target_session_id = parse_mm_or_uuid(&payload.session_id)
        .ok_or_else(|| AppError::BadRequest("Invalid session_id".to_string()))?;

    let call_manager = state.call_state_manager.as_ref();
    let mut call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;
    call = normalize_call_host_if_stale(&state, call).await;

    if !can_manage_call(&auth, &call) {
        return Err(AppError::Forbidden(
            "Only the host can stop screen sharing".to_string(),
        ));
    }

    let target_user_id = call
        .participants
        .values()
        .find(|p| p.session_id == target_session_id)
        .map(|p| p.user_id)
        .ok_or_else(|| AppError::NotFound("Participant not found in call".to_string()))?;

    call_manager
        .set_screen_sharing(call.call_id, target_user_id, false)
        .await;
    if call.screen_sharer == Some(target_user_id) {
        call_manager.set_screen_sharer(call.call_id, None).await;
    }

    if let Some(sfu) = state.sfu_manager.get_sfu(call.call_id).await {
        sfu.set_screen_sharing(target_session_id, false).await;
    }

    broadcast_screen_share_event(
        &state,
        channel_uuid,
        target_user_id,
        target_session_id,
        false,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/host/mute
/// Mute a participant by host
pub(crate) async fn host_mute(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<HostControlRequest>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    let target_session_id = parse_mm_or_uuid(&payload.session_id)
        .ok_or_else(|| AppError::BadRequest("Invalid session_id".to_string()))?;

    let call_manager = state.call_state_manager.as_ref();
    let mut call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;
    call = normalize_call_host_if_stale(&state, call).await;

    // Authorize: Only host can mute others
    if !can_manage_call(&auth, &call) {
        return Err(AppError::Forbidden(
            "Only the host can mute other participants".to_string(),
        ));
    }

    // Find target user by session_id
    let target_user_id = call
        .participants
        .values()
        .find(|p| p.session_id == target_session_id)
        .map(|p| p.user_id)
        .ok_or_else(|| AppError::NotFound("Participant not found in call".to_string()))?;

    // Mute in state
    call_manager
        .set_muted(call.call_id, target_user_id, true)
        .await;

    // Send host_mute event to the target user
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_host_mute",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "session_id": payload.session_id,
        }),
        Some(target_user_id),
    )
    .await;

    // Also broadcast regular muted event for UI updates
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_muted",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "user_id": encode_mm_id(target_user_id),
            "session_id": payload.session_id,
            "muted": true,
        }),
        None,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/host/mute-others
/// Mute all participants except host
pub(crate) async fn host_mute_others(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;

    let call_manager = state.call_state_manager.as_ref();
    let mut call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;
    call = normalize_call_host_if_stale(&state, call).await;

    if !can_manage_call(&auth, &call) {
        return Err(AppError::Forbidden(
            "Only the host can mute other participants".to_string(),
        ));
    }

    for participant in call.participants.values() {
        if participant.user_id == auth.user_id {
            continue;
        }

        call_manager
            .set_muted(call.call_id, participant.user_id, true)
            .await;

        // Signal each user
        broadcast_call_event(
            &state,
            "custom_com.mattermost.calls_host_mute",
            &channel_uuid,
            serde_json::json!({
                "channel_id": channel_id,
                "session_id": participant.session_id.to_string(),
            }),
            Some(participant.user_id),
        )
        .await;

        // Broadcast for UI
        broadcast_call_event(
            &state,
            "custom_com.mattermost.calls_user_muted",
            &channel_uuid,
            serde_json::json!({
                "channel_id": channel_id,
                "user_id": encode_mm_id(participant.user_id),
                "session_id": participant.session_id.to_string(),
                "muted": true,
            }),
            None,
        )
        .await;
    }

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/host/remove
/// Remove a participant from the call
pub(crate) async fn host_remove_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<HostControlRequest>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    let target_session_id = parse_mm_or_uuid(&payload.session_id)
        .ok_or_else(|| AppError::BadRequest("Invalid session_id".to_string()))?;

    let call_manager = state.call_state_manager.as_ref();
    let mut call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;
    call = normalize_call_host_if_stale(&state, call).await;

    if !can_manage_call(&auth, &call) {
        return Err(AppError::Forbidden(
            "Only the host can remove participants".to_string(),
        ));
    }

    let target_user_id = call
        .participants
        .values()
        .find(|p| p.session_id == target_session_id)
        .map(|p| p.user_id)
        .ok_or_else(|| AppError::NotFound("Participant not found in call".to_string()))?;

    if target_user_id == auth.user_id {
        return Err(AppError::BadRequest(
            "Host cannot remove themselves with this endpoint; use leave_call instead".to_string(),
        ));
    }

    // Signal host removal to target
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_host_removed",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "session_id": payload.session_id,
        }),
        Some(target_user_id),
    )
    .await;

    // Remove from state
    call_manager
        .remove_participant(call.call_id, target_user_id)
        .await;

    // Remove from SFU
    if let Some(sfu) = state.sfu_manager.get_sfu(call.call_id).await {
        let _ = sfu.remove_participant(target_session_id).await;
    }

    // Broadcast user_left for everyone
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_left",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "user_id": encode_mm_id(target_user_id),
            "session_id": payload.session_id,
        }),
        None,
    )
    .await;

    let remaining =
        reconcile_after_participant_left(&state, call.call_id, channel_uuid, target_user_id).await;
    if remaining <= 1 {
        schedule_empty_call_timeout(&state, call.call_id, channel_uuid);
    }

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/host/lower-hand
/// Lower a participant's hand
pub(crate) async fn host_lower_hand(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<HostControlRequest>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    let target_session_id = parse_mm_or_uuid(&payload.session_id)
        .ok_or_else(|| AppError::BadRequest("Invalid session_id".to_string()))?;

    let call_manager = state.call_state_manager.as_ref();
    let mut call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;
    call = normalize_call_host_if_stale(&state, call).await;

    if !can_manage_call(&auth, &call) {
        return Err(AppError::Forbidden(
            "Only the host can lower hands".to_string(),
        ));
    }

    let target_user_id = call
        .participants
        .values()
        .find(|p| p.session_id == target_session_id)
        .map(|p| p.user_id)
        .ok_or_else(|| AppError::NotFound("Participant not found in call".to_string()))?;

    // Lower hand in state
    call_manager
        .set_hand_raised(call.call_id, target_user_id, false)
        .await;

    // Signal target user
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_host_lower_hand",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "session_id": payload.session_id,
            "call_id": encode_mm_id(call.call_id),
            "host_id": encode_mm_id(auth.user_id),
        }),
        Some(target_user_id),
    )
    .await;

    let payload_json = serde_json::json!({
        "channel_id": channel_id,
        "user_id": encode_mm_id(target_user_id),
        "raised_hand": 0,
        "session_id": payload.session_id,
    });
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_unraise_hand",
        &channel_uuid,
        payload_json.clone(),
        None,
    )
    .await;
    // Legacy alias kept for compatibility with existing rustchat consumers.
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_lower_hand",
        &channel_uuid,
        payload_json,
        None,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/host/make
/// Transfer host status
pub(crate) async fn host_make_moderator(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<HostMakeRequest>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    let new_host_uuid = parse_mm_or_uuid(&payload.new_host_id)
        .ok_or_else(|| AppError::BadRequest("Invalid new_host_id".to_string()))?;

    let call_manager = state.call_state_manager.as_ref();
    let mut call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;
    call = normalize_call_host_if_stale(&state, call).await;

    if !can_manage_call(&auth, &call) {
        return Err(AppError::Forbidden(
            "Only the host can transfer host status".to_string(),
        ));
    }

    // Verify new host is a participant
    if !call.participants.contains_key(&new_host_uuid) {
        return Err(AppError::BadRequest(
            "New host must be a participant in the call".to_string(),
        ));
    }

    // Transfer host in state
    call_manager.set_host(call.call_id, new_host_uuid).await;

    broadcast_host_changed_event(&state, channel_uuid, new_host_uuid).await;
    broadcast_call_state_event(&state, channel_uuid, None).await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
