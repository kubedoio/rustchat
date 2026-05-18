use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::id::encode_mm_id;

use super::broadcast::broadcast_call_event;
use super::helpers::{
    build_call_state_response, channel_calls_enabled, check_channel_permission, resolve_channel_id,
    CHANNEL_CALLS_ENABLED,
};
use super::lifecycle::CallStateResponse;
use super::state::CallState;
#[derive(Debug, Deserialize)]
pub(crate) struct ChannelEnableRequest {
    enabled: bool,
}

pub(crate) async fn get_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<CallChannelInfo>>> {
    let call_manager = state.call_state_manager.as_ref();
    let active_calls = call_manager.get_all_calls().await;
    let mut calls_by_channel: HashMap<Uuid, Option<CallState>> = active_calls
        .into_iter()
        .map(|call| (call.channel_id, Some(call)))
        .collect();

    for entry in CHANNEL_CALLS_ENABLED.iter() {
        let override_channel_id: Uuid = *entry.key();
        calls_by_channel.entry(override_channel_id).or_insert(None);
    }

    let mut channels = Vec::new();
    for (channel_id, call) in calls_by_channel {
        let is_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)",
        )
        .bind(channel_id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);

        if !is_member {
            continue;
        }

        channels.push(
            build_call_channel_info(&state, channel_id, channel_calls_enabled(channel_id), call)
                .await?,
        );
    }

    Ok(Json(channels))
}
#[derive(Debug, Serialize)]
pub(crate) struct CallChannelInfo {
    channel_id: String,
    channel_id_raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    call_id_raw: Option<String>,
    enabled: bool,
    has_call: bool,
    participant_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    call: Option<CallStateResponse>,
}
async fn build_call_channel_info(
    state: &AppState,
    channel_uuid: Uuid,
    enabled: bool,
    call: Option<CallState>,
) -> ApiResult<CallChannelInfo> {
    let (call_id, call_id_raw, call_state, participant_count) = if let Some(call) = call {
        let participant_count = call.participants.len() as i32;
        let call_state = Some(
            build_call_state_response(state, &call, encode_mm_id(channel_uuid), channel_uuid)
                .await?,
        );
        (
            Some(encode_mm_id(call.call_id)),
            Some(call.call_id.to_string()),
            call_state,
            participant_count,
        )
    } else {
        (None, None, None, 0)
    };

    Ok(CallChannelInfo {
        channel_id: encode_mm_id(channel_uuid),
        channel_id_raw: channel_uuid.to_string(),
        call_id,
        call_id_raw,
        enabled,
        has_call: call_state.is_some(),
        participant_count,
        call: call_state,
    })
}
pub(crate) async fn get_channel_state_mobile(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<CallChannelInfo>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    check_channel_permission(&state, auth.user_id, channel_uuid).await?;

    let call = state
        .call_state_manager
        .get_call_by_channel(&channel_uuid)
        .await;
    let payload = build_call_channel_info(
        &state,
        channel_uuid,
        channel_calls_enabled(channel_uuid),
        call,
    )
    .await?;
    Ok(Json(payload))
}
/// POST /plugins/com.mattermost.calls/{channel_id}
/// Enable or disable calls in a channel.
pub(crate) async fn set_channel_calls_enabled(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Json(payload): Json<ChannelEnableRequest>,
) -> ApiResult<Json<CallChannelInfo>> {
    let channel_uuid = resolve_channel_id(&state, &channel_id).await?;
    check_channel_permission(&state, auth.user_id, channel_uuid).await?;

    CHANNEL_CALLS_ENABLED.insert(channel_uuid, payload.enabled);

    broadcast_call_event(
        &state,
        if payload.enabled {
            "custom_com.mattermost.calls_channel_enable_voice"
        } else {
            "custom_com.mattermost.calls_channel_disable_voice"
        },
        &channel_uuid,
        serde_json::json!({
            "enabled": payload.enabled,
        }),
        None,
    )
    .await;

    let call = state
        .call_state_manager
        .get_call_by_channel(&channel_uuid)
        .await;
    let response = build_call_channel_info(&state, channel_uuid, payload.enabled, call).await?;
    Ok(Json(response))
}
