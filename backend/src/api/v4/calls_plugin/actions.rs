use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use tracing::info;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::encode_mm_id;

use super::broadcast::{
    broadcast_call_event, broadcast_raise_hand_event, broadcast_screen_share_event,
};
use super::helpers::resolve_channel_id;
use super::lifecycle::StatusResponse;

#[derive(Debug, Deserialize)]
pub(crate) struct ReactionRequest {
    emoji: String,
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/react
/// Send a reaction during call
pub(crate) async fn send_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<ReactionRequest>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    let timestamp = Utc::now().timestamp_millis();
    let emoji_name = crate::mattermost_compat::emoji_data::get_short_name_for_emoji(&payload.emoji);

    let call_manager = state.call_state_manager.as_ref();
    let call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;
    let session_id = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .map(|p| p.session_id.to_string())
        .ok_or_else(|| AppError::Forbidden("You are not in this call".to_string()))?;

    // Broadcast reaction event
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_reacted",
        &channel_uuid,
        serde_json::json!({
            "user_id": encode_mm_id(auth.user_id),
            "session_id": session_id,
            "reaction": payload.emoji,
            "timestamp": timestamp,
            "emoji": {
                "name": emoji_name,
                "literal": payload.emoji,
            },
        }),
        None,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/screen-share
/// Toggle screen sharing
pub(crate) async fn toggle_screen_share(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;

    // Check if user is in call
    let participant = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .ok_or_else(|| AppError::Forbidden("You are not in this call".to_string()))?;

    // Toggle screen sharing
    let is_sharing = !participant.screen_sharing;
    call_manager
        .set_screen_sharing(call.call_id, auth.user_id, is_sharing)
        .await;

    // Update SFU screen sharing state for track forwarding
    if let Some(sfu) = state.sfu_manager.get_sfu(call.call_id).await {
        sfu.set_screen_sharing(participant.session_id, is_sharing)
            .await;
        info!(
            call_id = %call.call_id,
            session_id = %participant.session_id,
            is_sharing = is_sharing,
            "SFU screen sharing state updated"
        );
    }

    // Update global screen sharer
    if is_sharing {
        call_manager
            .set_screen_sharer(call.call_id, Some(auth.user_id))
            .await;
    } else if call.screen_sharer == Some(auth.user_id) {
        call_manager.set_screen_sharer(call.call_id, None).await;
    }

    broadcast_screen_share_event(
        &state,
        channel_uuid,
        auth.user_id,
        participant.session_id,
        is_sharing,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/mute
/// Mute self
pub(crate) async fn mute_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;

    // Verify user is in the call
    let participant = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .ok_or_else(|| AppError::Forbidden("You are not in this call".to_string()))?;
    let session_id = participant.session_id.to_string();

    // Set muted
    call_manager
        .set_muted(call.call_id, auth.user_id, true)
        .await;

    // Broadcast user_muted event
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_muted",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "user_id": encode_mm_id(auth.user_id),
            "session_id": session_id,
            "muted": true,
        }),
        None,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/unmute
/// Unmute self
pub(crate) async fn unmute_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;

    // Verify user is in the call
    let participant = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .ok_or_else(|| AppError::Forbidden("You are not in this call".to_string()))?;
    let session_id = participant.session_id.to_string();

    // Set unmuted
    call_manager
        .set_muted(call.call_id, auth.user_id, false)
        .await;

    // Broadcast user_unmuted event
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_unmuted",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "user_id": encode_mm_id(auth.user_id),
            "session_id": session_id,
            "muted": false,
        }),
        None,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/raise-hand
/// Raise hand
pub(crate) async fn raise_hand(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;

    let participant = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .ok_or_else(|| AppError::Forbidden("You are not in this call".to_string()))?;

    // Set hand raised
    call_manager
        .set_hand_raised(call.call_id, auth.user_id, true)
        .await;

    broadcast_raise_hand_event(
        &state,
        channel_uuid,
        auth.user_id,
        participant.session_id,
        true,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/lower-hand
/// Lower hand
pub(crate) async fn lower_hand(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = call_manager
        .get_call_by_channel(&channel_uuid)
        .await
        .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?;

    let participant = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .ok_or_else(|| AppError::Forbidden("You are not in this call".to_string()))?;

    // Set hand lowered
    call_manager
        .set_hand_raised(call.call_id, auth.user_id, false)
        .await;

    broadcast_raise_hand_event(
        &state,
        channel_uuid,
        auth.user_id,
        participant.session_id,
        false,
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
