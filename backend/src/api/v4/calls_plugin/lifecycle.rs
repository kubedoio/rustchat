use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::encode_mm_id;

use super::broadcast::{broadcast_call_event, broadcast_call_state_event, broadcast_ringing_event};
use super::helpers::{
    build_call_state_response, channel_calls_enabled, check_channel_permission, resolve_channel_id,
};
use super::posts::ensure_call_thread_id;
use super::signaling::spawn_signaling_forwarder;
use super::state::{CallState, Participant};
use super::state_helpers::{
    can_manage_call, end_call, is_host_session_active, normalize_call_host_if_stale,
    reconcile_after_participant_left, schedule_empty_call_timeout,
    schedule_unanswered_call_timeout, EMPTY_CALL_TIMEOUT_SECS,
};

#[derive(Debug, Serialize)]
pub(crate) struct StartCallResponse {
    pub(crate) id: String,
    pub(crate) id_raw: String,
    pub(crate) channel_id: String,
    pub(crate) channel_id_raw: String,
    pub(crate) start_at: i64,
    pub(crate) owner_id: String,
    pub(crate) owner_id_raw: String,
    pub(crate) host_id: String,
    pub(crate) host_id_raw: String,
}
#[derive(Debug, Serialize)]
pub(crate) struct CallStateResponse {
    pub(crate) id: String,
    pub(crate) id_raw: String,
    pub(crate) channel_id: String,
    pub(crate) channel_id_raw: String,
    pub(crate) start_at: i64,
    pub(crate) owner_id: String,
    pub(crate) owner_id_raw: String,
    pub(crate) host_id: String,
    pub(crate) host_id_raw: String,
    pub(crate) participants: Vec<String>,
    pub(crate) participants_raw: Vec<String>,
    pub(crate) sessions: HashMap<String, CallSessionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) screen_sharing_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) screen_sharing_id_raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) screen_sharing_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) screen_sharing_session_id_raw: Option<String>,
    pub(crate) thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recording: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dismissed_notification: Option<HashMap<String, bool>>,
}
#[derive(Debug, Serialize)]
pub(crate) struct CallSessionResponse {
    pub(crate) session_id: String,
    pub(crate) session_id_raw: String,
    pub(crate) user_id: String,
    pub(crate) user_id_raw: String,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) unmuted: bool,
    pub(crate) raised_hand: i32,
}

#[derive(Debug, Serialize)]
pub(crate) struct StatusResponse {
    pub(crate) status: String,
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/start
/// Starts a new call in a channel
pub(crate) async fn start_call(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StartCallResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    info!(
        user_id = %auth.user_id,
        channel_id = %channel_uuid,
        "calls.start_call requested"
    );

    // Check channel permissions
    check_channel_permission(&state, auth.user_id, channel_uuid).await?;
    if !channel_calls_enabled(channel_uuid) {
        return Err(AppError::Forbidden(
            "Calls are disabled in this channel".to_string(),
        ));
    }

    // Get or initialize call state manager
    let call_manager = state.call_state_manager.as_ref();

    // Check if call already exists
    if let Some(call) = call_manager.get_call_by_channel(&channel_uuid).await {
        info!(
            user_id = %auth.user_id,
            channel_id = %channel_uuid,
            call_id = %call.call_id,
            owner_id = %call.owner_id,
            "calls.start_call reused existing active call"
        );
        return Ok(Json(StartCallResponse {
            id: encode_mm_id(call.call_id),
            id_raw: call.call_id.to_string(),
            channel_id: channel_id.clone(),
            channel_id_raw: channel_uuid.to_string(),
            start_at: call.started_at,
            owner_id: encode_mm_id(call.owner_id),
            owner_id_raw: call.owner_id.to_string(),
            host_id: encode_mm_id(call.host_id),
            host_id_raw: call.host_id.to_string(),
        }));
    }

    // Create new call
    let call_id = Uuid::new_v4();
    let now = Utc::now().timestamp_millis();

    let call = CallState {
        call_id,
        channel_id: channel_uuid,
        owner_id: auth.user_id,
        host_id: auth.user_id,
        started_at: now,
        participants: HashMap::new(),
        screen_sharer: None,
        thread_id: None,
        dismissed_users: HashSet::new(),
    };

    call_manager.add_call(call.clone()).await;
    debug!(
        call_id = %call_id,
        channel_id = %channel_uuid,
        owner_id = %auth.user_id,
        "calls.start_call call state created"
    );

    let thread_id = ensure_call_thread_id(&state, &call).await;

    // Add owner as first participant (muted by default)
    let participant = Participant {
        user_id: auth.user_id,
        session_id: Uuid::new_v4(),
        joined_at: now,
        muted: true,
        screen_sharing: false,
        hand_raised: false,
    };

    call_manager
        .add_participant(call_id, participant.clone())
        .await;
    debug!(
        call_id = %call_id,
        user_id = %auth.user_id,
        session_id = %participant.session_id,
        "calls.start_call owner participant added"
    );

    // Get or create SFU for this call
    let sfu = state
        .sfu_manager
        .get_or_create_sfu(call_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create SFU: {}", e)))?;

    // Add owner as participant in the SFU
    let (_, signaling_rx) = sfu
        .add_participant(auth.user_id, participant.session_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to add participant to SFU: {}", e)))?;
    spawn_signaling_forwarder(
        &state,
        channel_uuid,
        auth.user_id,
        participant.session_id,
        signaling_rx,
    );
    debug!(
        call_id = %call_id,
        user_id = %auth.user_id,
        session_id = %participant.session_id,
        "calls.start_call signaling forwarder spawned"
    );

    // Broadcast call_start event
    // Note: thread_id is used as post_id for call posts in Mattermost
    let thread_id_str = thread_id.map(encode_mm_id).unwrap_or_default();
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_call_start",
        &channel_uuid,
        serde_json::json!({
            "id": encode_mm_id(call_id),
            "channel_id": channel_id,
            "channelID": encode_mm_id(channel_uuid),
            "user_id": encode_mm_id(auth.user_id),
            "call_id": encode_mm_id(call_id),
            "start_at": now,
            "owner_id": encode_mm_id(auth.user_id),
            "host_id": encode_mm_id(auth.user_id),
            "thread_id": thread_id_str.clone(),
            "post_id": thread_id_str,  // Mobile expects post_id for navigation
        }),
        Some(auth.user_id), // Exclude sender
    )
    .await;

    // Broadcast user_joined event
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_joined",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "user_id": encode_mm_id(auth.user_id),
            "session_id": participant.session_id.to_string(),
            "muted": true,
            "raised_hand": false,
        }),
        None,
    )
    .await;

    // Send ringing notifications to all channel members
    // This ensures push notifications are sent for calls in ALL channel types
    // (DMs, groups, and regular channels)
    broadcast_ringing_event(
        &state,
        channel_uuid,
        call_id,
        auth.user_id,
        Some(auth.user_id),
    )
    .await;

    broadcast_call_state_event(&state, channel_uuid, None).await;

    // Mattermost-compatible behavior: if nobody else joins, drop the call after a ring timeout.
    schedule_unanswered_call_timeout(&state, call_id, channel_uuid);

    info!(
        call_id = %call_id,
        channel_id = %channel_uuid,
        owner_id = %auth.user_id,
        session_id = %participant.session_id,
        "calls.start_call completed"
    );

    Ok(Json(StartCallResponse {
        id: encode_mm_id(call_id),
        id_raw: call_id.to_string(),
        channel_id: channel_id.clone(),
        channel_id_raw: channel_uuid.to_string(),
        start_at: now,
        owner_id: encode_mm_id(auth.user_id),
        owner_id_raw: auth.user_id.to_string(),
        host_id: encode_mm_id(auth.user_id),
        host_id_raw: auth.user_id.to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/join
/// Join an existing call
pub(crate) async fn join_call(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    info!(
        user_id = %auth.user_id,
        channel_id = %channel_uuid,
        "calls.join_call requested"
    );

    // Check channel permissions
    check_channel_permission(&state, auth.user_id, channel_uuid).await?;
    if !channel_calls_enabled(channel_uuid) {
        return Err(AppError::Forbidden(
            "Calls are disabled in this channel".to_string(),
        ));
    }

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find active call in channel
    let call = match call_manager.get_call_by_channel(&channel_uuid).await {
        Some(c) => c,
        None => call_manager
            .get_call(channel_uuid)
            .await
            .ok_or_else(|| AppError::NoActiveCall)?,
    };

    // Check if user already in call
    if call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .is_some()
    {
        info!(
            user_id = %auth.user_id,
            channel_id = %channel_uuid,
            call_id = %call.call_id,
            "calls.join_call user already in call"
        );
        return Ok(Json(StatusResponse {
            status: "OK".to_string(),
        }));
    }

    // Add participant
    let now = Utc::now().timestamp_millis();
    let participant = Participant {
        user_id: auth.user_id,
        session_id: Uuid::new_v4(),
        joined_at: now,
        muted: true,
        screen_sharing: false,
        hand_raised: false,
    };

    call_manager
        .add_participant(call.call_id, participant.clone())
        .await;
    debug!(
        call_id = %call.call_id,
        user_id = %auth.user_id,
        session_id = %participant.session_id,
        "calls.join_call participant added to call state"
    );

    // Get or create SFU for this call
    let sfu = state
        .sfu_manager
        .get_or_create_sfu(call.call_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get or create SFU: {}", e)))?;

    // Add participant to the SFU
    let (_, signaling_rx) = sfu
        .add_participant(auth.user_id, participant.session_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to add participant to SFU: {}", e)))?;
    spawn_signaling_forwarder(
        &state,
        channel_uuid,
        auth.user_id,
        participant.session_id,
        signaling_rx,
    );
    debug!(
        call_id = %call.call_id,
        user_id = %auth.user_id,
        session_id = %participant.session_id,
        "calls.join_call signaling forwarder spawned"
    );

    // Broadcast user_joined event
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_joined",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "user_id": encode_mm_id(auth.user_id),
            "session_id": participant.session_id.to_string(),
            "muted": true,
            "raised_hand": false,
        }),
        None,
    )
    .await;
    broadcast_call_state_event(&state, channel_uuid, None).await;

    info!(
        call_id = %call.call_id,
        channel_id = %channel_uuid,
        user_id = %auth.user_id,
        session_id = %participant.session_id,
        "calls.join_call completed"
    );

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/leave
/// Leave a call
pub(crate) async fn leave_call(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    info!(
        user_id = %auth.user_id,
        channel_id = %channel_uuid,
        "calls.leave_call requested"
    );

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = match call_manager.get_call_by_channel(&channel_uuid).await {
        Some(c) => c,
        None => {
            debug!(
                channel_id = %channel_uuid,
                "calls.leave_call: no active call found, returning success"
            );
            return Ok(Json(StatusResponse {
                status: "OK".to_string(),
            }));
        }
    };

    // Get participant info before removing (for session_id)
    let participant = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await;

    // Remove participant from call manager
    call_manager
        .remove_participant(call.call_id, auth.user_id)
        .await;

    // Remove participant from SFU if exists
    if let Some(participant) = participant {
        if let Some(sfu) = state.sfu_manager.get_sfu(call.call_id).await {
            let _ = sfu.remove_participant(participant.session_id).await;
            debug!(
                call_id = %call.call_id,
                user_id = %auth.user_id,
                session_id = %participant.session_id,
                "calls.leave_call participant removed from SFU"
            );
        }
    }

    // Broadcast user_left event
    broadcast_call_event(
        &state,
        "custom_com.mattermost.calls_user_left",
        &channel_uuid,
        serde_json::json!({
            "channel_id": channel_id,
            "user_id": encode_mm_id(auth.user_id),
        }),
        None,
    )
    .await;
    let remaining =
        reconcile_after_participant_left(&state, call.call_id, channel_uuid, auth.user_id).await;
    if remaining <= 1 {
        schedule_empty_call_timeout(&state, call.call_id, channel_uuid);
        info!(
            call_id = %call.call_id,
            channel_id = %channel_uuid,
            remaining_participants = remaining,
            timeout_secs = EMPTY_CALL_TIMEOUT_SECS,
            "calls.leave_call scheduled no-remote-participant timeout"
        );
    } else {
        info!(
            call_id = %call.call_id,
            channel_id = %channel_uuid,
            remaining_participants = remaining,
            "calls.leave_call completed"
        );
    }

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/end
/// End a call (host only).
pub(crate) async fn end_call_endpoint(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_or_call_uuid = resolve_channel_id(&state, &channel_id).await?;
    let call_manager = state.call_state_manager.as_ref();

    let mut call = match call_manager
        .get_call_by_channel(&channel_or_call_uuid)
        .await
    {
        Some(c) => c,
        None => match call_manager.get_call(channel_or_call_uuid).await {
            Some(c) => c,
            None => {
                return Ok(Json(StatusResponse {
                    status: "OK".to_string(),
                }));
            }
        },
    };

    check_channel_permission(&state, auth.user_id, call.channel_id).await?;
    call = normalize_call_host_if_stale(&state, call).await;

    let caller_is_participant = call.participants.contains_key(&auth.user_id);
    let caller_is_only_participant = call.participants.len() <= 1 && caller_is_participant;
    let host_session_inactive = caller_is_participant && !is_host_session_active(&state, &call);
    if !can_manage_call(&auth, &call) && !caller_is_only_participant && !host_session_inactive {
        return Err(AppError::Forbidden(
            "Only the host can end this call".to_string(),
        ));
    }

    end_call(
        &state,
        call.call_id,
        call.channel_id,
        "ended_by_host",
        call.participants.len(),
    )
    .await;

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
/// GET /plugins/com.mattermost.calls/calls/{channel_id}
/// Get current call state
pub(crate) async fn get_call_state(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Response> {
    let normalized_channel_id = channel_id.trim();
    if normalized_channel_id.is_empty()
        || normalized_channel_id.eq_ignore_ascii_case("undefined")
        || normalized_channel_id.eq_ignore_ascii_case("null")
    {
        return Err(AppError::NotFound(
            "No active call in this channel".to_string(),
        ));
    }

    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    check_channel_permission(&state, auth.user_id, channel_uuid).await?;

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = match call_manager.get_call_by_channel(&channel_uuid).await {
        Some(c) => c,
        None => {
            // Try looking up by Call ID as a fallback if Channel ID lookup failed
            match call_manager.get_call(channel_uuid).await {
                Some(c) => c,
                None => {
                    // Return silent 404 to avoid noisy ERROR logs for a common client polling case
                    let body = crate::error::ErrorResponse {
                        error: crate::error::ErrorBody {
                            code: "NOT_FOUND".to_string(),
                            message: "No active call in this channel".to_string(),
                            details: None,
                        },
                    };
                    return Ok((axum::http::StatusCode::NOT_FOUND, Json(body)).into_response());
                }
            }
        }
    };
    Ok(
        Json(build_call_state_response(&state, &call, channel_id.clone(), channel_uuid).await?)
            .into_response(),
    )
}
