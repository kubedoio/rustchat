use chrono::Utc;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::mattermost_compat::id::encode_mm_id;
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};

use super::broadcast::{
    broadcast_call_event, broadcast_call_state_event, broadcast_host_changed_event,
};
use super::posts::mark_call_thread_post_ended;
use super::state::{CallState, Participant};
pub(crate) const UNANSWERED_CALL_TIMEOUT_SECS: u64 = 20;
pub(crate) const EMPTY_CALL_TIMEOUT_SECS: u64 = 10;

pub(crate) fn can_manage_call(auth: &MmAuthUser, call: &CallState) -> bool {
    call.host_id == auth.user_id || auth.has_permission(&permissions::ADMIN_FULL)
}
pub(crate) fn is_host_session_active(_state: &AppState, call: &CallState) -> bool {
    // The host has an active session if they're in the participants list.
    // We don't check the connection_store here because:
    // 1. The call session_id is different from the WebSocket connection_id
    // 2. A participant in the call should be considered "active" regardless of
    //    their WebSocket connection state (they might reconnect via WS but stay in the call)
    call.participants.contains_key(&call.host_id)
}
fn select_next_host(participants: &HashMap<Uuid, Participant>) -> Option<Uuid> {
    participants
        .values()
        .min_by_key(|participant| (participant.joined_at, participant.user_id))
        .map(|participant| participant.user_id)
}
pub(crate) async fn normalize_call_host_if_stale(state: &AppState, call: CallState) -> CallState {
    if call.participants.is_empty() || call.participants.contains_key(&call.host_id) {
        return call;
    }

    let Some(new_host_id) = select_next_host(&call.participants) else {
        return call;
    };

    state
        .call_state_manager
        .set_host(call.call_id, new_host_id)
        .await;
    broadcast_host_changed_event(state, call.channel_id, new_host_id).await;
    broadcast_call_state_event(state, call.channel_id, None).await;

    state
        .call_state_manager
        .get_call(call.call_id)
        .await
        .unwrap_or(call)
}
pub(crate) async fn reconcile_after_participant_left(
    state: &AppState,
    call_id: Uuid,
    channel_id: Uuid,
    departed_user_id: Uuid,
) -> usize {
    let mut call = match state.call_state_manager.get_call(call_id).await {
        Some(call) => call,
        None => return 0,
    };

    let should_select_new_host = call.host_id == departed_user_id
        || (!call.participants.is_empty() && !call.participants.contains_key(&call.host_id));
    if should_select_new_host {
        if let Some(new_host_id) = select_next_host(&call.participants) {
            state
                .call_state_manager
                .set_host(call.call_id, new_host_id)
                .await;
            broadcast_host_changed_event(state, channel_id, new_host_id).await;
            if let Some(updated_call) = state.call_state_manager.get_call(call_id).await {
                call = updated_call;
            }
        }
    }

    broadcast_call_state_event(state, channel_id, None).await;

    call.participants.len()
}
pub(crate) fn schedule_unanswered_call_timeout(state: &AppState, call_id: Uuid, channel_id: Uuid) {
    let state = state.clone();
    tokio::spawn(async move {
        info!(
            call_id = %call_id,
            channel_id = %channel_id,
            timeout_secs = UNANSWERED_CALL_TIMEOUT_SECS,
            "calls.timeout scheduled unanswered-call timeout"
        );
        sleep(Duration::from_secs(UNANSWERED_CALL_TIMEOUT_SECS)).await;
        end_call_if_still_unanswered(&state, call_id).await;
    });
}
pub(crate) fn schedule_empty_call_timeout(state: &AppState, call_id: Uuid, channel_id: Uuid) {
    let state = state.clone();
    tokio::spawn(async move {
        info!(
            call_id = %call_id,
            channel_id = %channel_id,
            timeout_secs = EMPTY_CALL_TIMEOUT_SECS,
            "calls.timeout scheduled empty-call timeout"
        );
        sleep(Duration::from_secs(EMPTY_CALL_TIMEOUT_SECS)).await;
        end_call_if_still_empty(&state, call_id).await;
    });
}
async fn end_call_if_still_unanswered(state: &AppState, call_id: Uuid) {
    let Some(call) = state.call_state_manager.get_call(call_id).await else {
        return;
    };

    let participant_count = call.participants.len();
    if participant_count > 1 {
        debug!(
            call_id = %call_id,
            participant_count = participant_count,
            "calls.timeout unanswered-call timeout skipped"
        );
        return;
    }

    end_call(
        state,
        call.call_id,
        call.channel_id,
        "unanswered_timeout",
        participant_count,
    )
    .await;
}

async fn end_call_if_still_empty(state: &AppState, call_id: Uuid) {
    let Some(call) = state.call_state_manager.get_call(call_id).await else {
        return;
    };

    let participant_count = call.participants.len();
    if participant_count > 1 {
        debug!(
            call_id = %call_id,
            participant_count = participant_count,
            "calls.timeout no-remote-participant timeout skipped"
        );
        return;
    }

    end_call(
        state,
        call.call_id,
        call.channel_id,
        "no_remote_participant_timeout",
        participant_count,
    )
    .await;
}

pub(crate) async fn end_call(
    state: &AppState,
    call_id: Uuid,
    channel_id: Uuid,
    reason: &'static str,
    participant_count: usize,
) {
    let thread_id = state
        .call_state_manager
        .get_call(call_id)
        .await
        .and_then(|call| call.thread_id);
    let ended_at = Utc::now().timestamp_millis();

    state.call_state_manager.remove_call(call_id).await;
    state.sfu_manager.remove_sfu(call_id).await;

    if let Some(call_thread_id) = thread_id {
        match mark_call_thread_post_ended(state, call_thread_id, ended_at).await {
            Ok(Some(updated_post)) => {
                let broadcast =
                    WsEnvelope::event(EventType::MessageUpdated, updated_post, Some(channel_id))
                        .with_broadcast(WsBroadcast {
                            channel_id: Some(channel_id),
                            team_id: None,
                            user_id: None,
                            exclude_user_id: None,
                        });
                state.ws_hub.broadcast(broadcast).await;
            }
            Ok(None) => {
                warn!(
                    call_id = %call_id,
                    thread_id = %call_thread_id,
                    "calls.end_call thread post not found while marking end_at"
                );
            }
            Err(err) => {
                warn!(
                    call_id = %call_id,
                    thread_id = %call_thread_id,
                    error = %err,
                    "calls.end_call failed to persist end_at on call thread post"
                );
            }
        }
    }

    let encoded_channel_id = encode_mm_id(channel_id);
    let encoded_call_id = encode_mm_id(call_id);
    broadcast_call_event(
        state,
        "custom_com.mattermost.calls_call_end",
        &channel_id,
        serde_json::json!({
            "id": encoded_call_id,
            "channelID": encoded_channel_id,
            "call_id": encoded_call_id,
            "channel_id": encoded_channel_id,
        }),
        None,
    )
    .await;

    info!(
        call_id = %call_id,
        channel_id = %channel_id,
        reason = reason,
        participant_count = participant_count,
        "calls.call ended"
    );
}
