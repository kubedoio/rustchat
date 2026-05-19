use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

use super::lifecycle::{CallSessionResponse, CallStateResponse};
use super::posts::ensure_call_thread_id;
use super::state::CallState;

pub(crate) static CHANNEL_CALLS_ENABLED: Lazy<DashMap<Uuid, bool>> = Lazy::new(DashMap::new);

pub(crate) fn channel_calls_enabled(channel_id: Uuid) -> bool {
    CHANNEL_CALLS_ENABLED
        .get(&channel_id)
        .map(|entry| *entry)
        .unwrap_or(true)
}

/// Helper to resolve a channel ID which might be a UUID, a Mattermost encoded ID, or a DM name.
pub(crate) async fn resolve_channel_id(state: &AppState, channel_id: &str) -> ApiResult<Uuid> {
    let channel_id = channel_id.trim();
    if let Ok(uuid) = Uuid::parse_str(channel_id) {
        return Ok(uuid);
    }

    if let Some(uuid) = parse_mm_or_uuid(channel_id) {
        return Ok(uuid);
    }

    // Check if it's a DM name
    if crate::models::channel::parse_direct_channel_name(channel_id).is_some() {
        // Look up channel by name
        let channel_uuid: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM channels WHERE name = $1")
                .bind(channel_id)
                .fetch_optional(&state.db)
                .await?;

        if let Some(uuid) = channel_uuid {
            return Ok(uuid);
        }
    }

    Err(AppError::BadRequest("Invalid channel_id".to_string()))
}
pub(crate) async fn build_call_state_response(
    state: &AppState,
    call: &CallState,
    channel_id_for_response: String,
    channel_uuid: Uuid,
) -> ApiResult<CallStateResponse> {
    let thread_id = ensure_call_thread_id(state, call).await;

    let call_participants = state
        .call_state_manager
        .get_participants(call.call_id)
        .await;

    let user_ids: Vec<Uuid> = call_participants.iter().map(|p| p.user_id).collect();
    let users_info: HashMap<Uuid, (String, String)> = if !user_ids.is_empty() {
        sqlx::query("SELECT id, username, COALESCE(display_name, '') as display_name FROM users WHERE id = ANY($1)")
            .bind(&user_ids)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                let id: Uuid = row.get(0);
                let username: String = row.get(1);
                let display_name: String = row.get(2);
                (id, (username, display_name))
            })
            .collect()
    } else {
        HashMap::new()
    };

    let participants: Vec<String> = call_participants
        .iter()
        .map(|p| encode_mm_id(p.user_id))
        .collect();
    let participants_raw: Vec<String> = call_participants
        .iter()
        .map(|p| p.user_id.to_string())
        .collect();
    let sessions: HashMap<String, CallSessionResponse> = call_participants
        .iter()
        .map(|participant| {
            let raw_session_id = participant.session_id.to_string();
            let (username, display_name) = users_info
                .get(&participant.user_id)
                .cloned()
                .unwrap_or_else(|| (participant.user_id.to_string(), String::new()));

            (
                raw_session_id.clone(),
                CallSessionResponse {
                    session_id: raw_session_id,
                    session_id_raw: participant.session_id.to_string(),
                    user_id: encode_mm_id(participant.user_id),
                    user_id_raw: participant.user_id.to_string(),
                    username,
                    display_name,
                    unmuted: !participant.muted,
                    raised_hand: if participant.hand_raised { 1 } else { 0 },
                },
            )
        })
        .collect();
    let screen_sharing_session = call.screen_sharer.and_then(|screen_sharer| {
        call_participants
            .iter()
            .find(|participant| participant.user_id == screen_sharer)
    });
    let dismissed_notification: HashMap<String, bool> = call
        .dismissed_users
        .iter()
        .map(|user_id| (encode_mm_id(*user_id), true))
        .collect();

    Ok(CallStateResponse {
        id: encode_mm_id(call.call_id),
        id_raw: call.call_id.to_string(),
        channel_id: channel_id_for_response,
        channel_id_raw: channel_uuid.to_string(),
        start_at: call.started_at,
        owner_id: encode_mm_id(call.owner_id),
        owner_id_raw: call.owner_id.to_string(),
        host_id: encode_mm_id(call.host_id),
        host_id_raw: call.host_id.to_string(),
        participants,
        participants_raw,
        sessions,
        screen_sharing_id: call.screen_sharer.map(encode_mm_id),
        screen_sharing_id_raw: call.screen_sharer.map(|id| id.to_string()),
        screen_sharing_session_id: screen_sharing_session
            .map(|participant| participant.session_id.to_string()),
        screen_sharing_session_id_raw: screen_sharing_session
            .map(|participant| participant.session_id.to_string()),
        thread_id: thread_id.map(encode_mm_id),
        recording: None,
        dismissed_notification: Some(dismissed_notification),
    })
}
/// Check if user has permission to access channel
pub(crate) async fn check_channel_permission(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
) -> ApiResult<()> {
    // Check if user is channel member
    let member: Option<(Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    if member.is_none() {
        return Err(AppError::Forbidden(
            "You are not a member of this channel".to_string(),
        ));
    }

    Ok(())
}
