use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::encode_mm_id;

use super::helpers::resolve_channel_id;
use super::lifecycle::StatusResponse;
use super::signaling::{send_signaling_event, spawn_signaling_forwarder};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

// WebRTC Signaling Request/Response structs
#[derive(Debug, Deserialize)]
pub struct OfferRequest {
    pub sdp: String,
}

#[derive(Debug, Serialize)]
pub struct AnswerResponse {
    pub sdp: String,
    pub type_: String,
}

#[derive(Debug, Deserialize)]
pub struct IceCandidateRequest {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/offer
/// Receives SDP offer from client, creates peer connection in SFU, returns SDP answer
pub(crate) async fn handle_offer(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<OfferRequest>,
) -> ApiResult<Json<AnswerResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    info!(
        user_id = %auth.user_id,
        channel_id = %channel_uuid,
        sdp_len = payload.sdp.len(),
        "calls.offer received"
    );

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = match call_manager.get_call_by_channel(&channel_uuid).await {
        Some(c) => c,
        None => call_manager
            .get_call(channel_uuid)
            .await
            .ok_or_else(|| AppError::NotFound("No active call in this channel".to_string()))?,
    };

    // Get participant session_id
    let participant = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
        .ok_or_else(|| AppError::Forbidden("You are not in this call".to_string()))?;

    // Get or create SFU for this call. In multi-node or resumed-state scenarios,
    // call state can exist before a local SFU is hydrated.
    let sfu = state
        .sfu_manager
        .get_or_create_sfu(call.call_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get or create SFU: {}", e)))?;

    info!(call_id = %call.call_id, "SFU retrieved/created");

    // Ensure this participant is present in the SFU before handling signaling.
    // Also ensure the signaling forwarder is running to send ICE candidates to the client.
    let signaling_rx = if !sfu.has_participant(participant.session_id).await {
        warn!(
            call_id = %call.call_id,
            user_id = %auth.user_id,
            session_id = %participant.session_id,
            "calls.offer participant missing in SFU; recovering by re-registering"
        );
        let (_, rx) = sfu
            .add_participant(auth.user_id, participant.session_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to add participant to SFU: {}", e)))?;
        Some(rx)
    } else {
        // Participant exists but we need to ensure signaling forwarder is running
        // Get the signaling receiver for the existing participant
        sfu.get_signaling_receiver(participant.session_id).await
    };

    // Spawn signaling forwarder if we have a receiver (new participant or reconnection)
    if let Some(rx) = signaling_rx {
        spawn_signaling_forwarder(
            &state,
            channel_uuid,
            auth.user_id,
            participant.session_id,
            rx,
        );
    }

    // Parse the offer SDP (keep raw SDP for potential retry)
    let sdp_raw = payload.sdp;
    let offer = RTCSessionDescription::offer(sdp_raw.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid SDP offer: {}", e)))?;

    // Handle the offer and get answer.
    // If it fails (e.g. dead PeerConnection), recreate the participant and retry once.
    let answer = match sfu.handle_offer(participant.session_id, offer).await {
        Ok(ans) => ans,
        Err(first_err) => {
            warn!(
                session_id = %participant.session_id,
                error = %first_err,
                "sfu.handle_offer failed; recreating PeerConnection and retrying"
            );

            let (_, signaling_rx) = sfu
                .recreate_participant(auth.user_id, participant.session_id)
                .await
                .map_err(|e| {
                    error!(session_id = %participant.session_id, error = %e, "recreate_participant failed");
                    AppError::Internal(format!("Failed to recreate participant: {}", e))
                })?;

            spawn_signaling_forwarder(
                &state,
                channel_uuid,
                auth.user_id,
                participant.session_id,
                signaling_rx,
            );

            let retry_offer = RTCSessionDescription::offer(sdp_raw)
                .map_err(|e| AppError::Internal(format!("Invalid SDP on retry: {}", e)))?;

            sfu.handle_offer(participant.session_id, retry_offer)
                .await
                .map_err(|e| {
                    error!(session_id = %participant.session_id, error = %e, "sfu.handle_offer retry also failed");
                    AppError::Internal(format!("Failed to handle offer after retry: {}", e))
                })?
        }
    };
    debug!(
        call_id = %call.call_id,
        user_id = %auth.user_id,
        session_id = %participant.session_id,
        answer_sdp_len = answer.sdp.len(),
        "calls.offer handled successfully"
    );

    // Extract SDP from answer
    let sdp = answer.sdp;
    send_signaling_event(
        &state,
        channel_uuid,
        auth.user_id,
        participant.session_id,
        SignalingMessage::Answer { sdp: sdp.clone() },
    )
    .await;

    Ok(Json(AnswerResponse {
        sdp,
        type_: "answer".to_string(),
    }))
}
/// POST /plugins/com.mattermost.calls/calls/{channel_id}/ice
/// Receives ICE candidate from client and adds it to the peer connection
pub(crate) async fn handle_ice_candidate(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<IceCandidateRequest>,
) -> ApiResult<Json<StatusResponse>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;

    let candidate_len = payload.candidate.len();
    debug!(
        user_id = %auth.user_id,
        channel_id = %channel_uuid,
        candidate_len = candidate_len,
        sdp_mid = ?payload.sdp_mid,
        sdp_mline_index = ?payload.sdp_mline_index,
        "calls.ice received candidate"
    );

    // Get call manager
    let call_manager = state.call_state_manager.as_ref();

    // Find call
    let call = match call_manager.get_call_by_channel(&channel_uuid).await {
        Some(c) => c,
        None => match call_manager.get_call(channel_uuid).await {
            Some(c) => c,
            None => {
                warn!(
                    user_id = %auth.user_id,
                    channel_id = %channel_uuid,
                    "Ignoring ICE candidate: no active call in this channel"
                );
                return Ok(Json(StatusResponse {
                    status: "IGNORED".to_string(),
                }));
            }
        },
    };

    // Get participant session_id
    let Some(participant) = call_manager
        .get_participant(call.call_id, auth.user_id)
        .await
    else {
        warn!(
            user_id = %auth.user_id,
            call_id = %call.call_id,
            "Ignoring ICE candidate: user is not a participant of the call"
        );
        return Ok(Json(StatusResponse {
            status: "IGNORED".to_string(),
        }));
    };

    // Get SFU for this call
    let sfu = state
        .sfu_manager
        .get_or_create_sfu(call.call_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get or create SFU: {}", e)))?;

    if !sfu.has_participant(participant.session_id).await {
        warn!(
            call_id = %call.call_id,
            user_id = %auth.user_id,
            session_id = %participant.session_id,
            "calls.ice participant missing in SFU; recovering by re-registering"
        );
        let (_, signaling_rx) = sfu
            .add_participant(auth.user_id, participant.session_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to add participant to SFU: {}", e)))?;
        spawn_signaling_forwarder(
            &state,
            call.channel_id,
            auth.user_id,
            participant.session_id,
            signaling_rx,
        );
    }

    // Handle the ICE candidate
    sfu.handle_ice_candidate(
        participant.session_id,
        payload.candidate,
        payload.sdp_mid,
        payload.sdp_mline_index,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Failed to handle ICE candidate: {}", e)))?;
    debug!(
        call_id = %call.call_id,
        user_id = %auth.user_id,
        session_id = %participant.session_id,
        candidate_len = candidate_len,
        "calls.ice handled successfully"
    );

    Ok(Json(StatusResponse {
        status: "OK".to_string(),
    }))
}
