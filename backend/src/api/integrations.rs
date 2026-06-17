//! Integrations API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::auth::policy::permissions;
use crate::auth::AuthUser;
use crate::calls::state::{CallState, Participant};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::encode_mm_id;
use crate::middleware::reliability::{send_reqwest_with_retry, RetryCondition, RetryConfig};
use crate::models::{
    Bot, BotToken, CommandResponse, CreateBot, CreateIncomingWebhook, CreateOutgoingWebhook,
    CreateSlashCommand, ExecuteCommand, IncomingWebhook, OutgoingWebhook, OutgoingWebhookPayload,
    SlashCommand, WebhookPayload,
};
use crate::repositories::{ChannelRepository, IntegrationRepository, UserRepository};
use crate::services::webhooks::{callback_http_client, is_valid_callback_url};
use chrono::Utc;
use std::time::Duration;

/// Generate a secure random token
fn generate_token() -> String {
    use rand::distributions::{Alphanumeric, DistString};
    Alphanumeric.sample_string(&mut rand::thread_rng(), 32)
}

/// Build integrations routes
pub fn router() -> Router<AppState> {
    Router::new()
        // Incoming webhooks
        .route(
            "/hooks/incoming",
            get(list_incoming_webhooks).post(create_incoming_webhook),
        )
        .route(
            "/hooks/incoming/{id}",
            get(get_incoming_webhook).delete(delete_incoming_webhook),
        )
        .route("/hooks/{token}", post(execute_incoming_webhook))
        // Outgoing webhooks
        .route(
            "/hooks/outgoing",
            get(list_outgoing_webhooks).post(create_outgoing_webhook),
        )
        .route(
            "/hooks/outgoing/{id}",
            get(get_outgoing_webhook).delete(delete_outgoing_webhook),
        )
        // Slash commands
        .route(
            "/commands",
            get(list_slash_commands).post(create_slash_command),
        )
        .route(
            "/commands/{id}",
            get(get_slash_command).delete(delete_slash_command),
        )
        .route("/commands/execute", post(execute_command))
        // Bots
        .route("/bots", get(list_bots).post(create_bot))
        .route("/bots/{id}", get(get_bot).delete(delete_bot))
        .route(
            "/bots/{id}/tokens",
            get(list_bot_tokens).post(create_bot_token),
        )
        .route("/bots/{bot_id}/tokens/{token_id}", delete(revoke_bot_token))
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandAuth {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamQuery {
    pub team_id: Uuid,
}

/// Verify the user is a member of the specified team.
async fn ensure_team_member(state: &AppState, team_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    if !ChannelRepository::new(&state.db)
        .is_team_member(team_id, user_id)
        .await?
    {
        return Err(AppError::NotOnTeam);
    }
    Ok(())
}

// ============ Incoming Webhooks ============

async fn list_incoming_webhooks(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<IncomingWebhook>>> {
    ensure_team_member(&state, query.team_id, auth.user_id).await?;

    let webhooks = IntegrationRepository::new(&state.db)
        .list_incoming_webhooks_by_team(query.team_id)
        .await?;

    Ok(Json(webhooks))
}

async fn create_incoming_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
    Json(input): Json<CreateIncomingWebhook>,
) -> ApiResult<Json<IncomingWebhook>> {
    let token = generate_token();

    let webhook = IntegrationRepository::new(&state.db)
        .create_incoming_webhook(
            query.team_id,
            input.channel_id,
            auth.user_id,
            input.display_name.as_deref(),
            input.description.as_deref(),
            &token,
            true,
        )
        .await?;

    Ok(Json(webhook))
}

async fn get_incoming_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<IncomingWebhook>> {
    let webhook = IntegrationRepository::new(&state.db)
        .get_incoming_webhook_by_id(id)
        .await?
        .ok_or_else(|| AppError::WebhookNotFound)?;

    if !auth.can_access_owned(webhook.creator_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Cannot access this webhook".to_string(),
        ));
    }

    Ok(Json(webhook))
}

async fn delete_incoming_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let webhook = IntegrationRepository::new(&state.db)
        .get_incoming_webhook_by_id(id)
        .await?
        .ok_or_else(|| AppError::WebhookNotFound)?;

    if !auth.can_access_owned(webhook.creator_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Cannot delete this webhook".to_string(),
        ));
    }

    IntegrationRepository::new(&state.db)
        .delete_incoming_webhook(id)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// Execute an incoming webhook (external service posts here)
async fn execute_incoming_webhook(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<WebhookPayload>,
) -> ApiResult<Json<serde_json::Value>> {
    let webhook = IntegrationRepository::new(&state.db)
        .get_incoming_webhook_by_token(&token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid webhook token".to_string()))?;

    // Create a post in the channel
    IntegrationRepository::new(&state.db)
        .create_post_from_webhook(
            webhook.channel_id,
            webhook.creator_id,
            &payload.text,
            &payload.props,
        )
        .await?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

// ============ Outgoing Webhooks ============

async fn list_outgoing_webhooks(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<OutgoingWebhook>>> {
    ensure_team_member(&state, query.team_id, auth.user_id).await?;

    let webhooks = IntegrationRepository::new(&state.db)
        .list_outgoing_webhooks_by_team(query.team_id)
        .await?;

    Ok(Json(webhooks))
}

async fn create_outgoing_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
    Json(input): Json<CreateOutgoingWebhook>,
) -> ApiResult<Json<OutgoingWebhook>> {
    if input.callback_urls.is_empty() {
        return Err(AppError::Validation(
            "At least one callback URL required".to_string(),
        ));
    }

    for url in &input.callback_urls {
        if !is_valid_callback_url(url) {
            return Err(AppError::Validation(format!(
                "Invalid callback URL: {}",
                url
            )));
        }
    }

    let token = generate_token();

    let webhook = IntegrationRepository::new(&state.db)
        .create_outgoing_webhook(
            query.team_id,
            input.channel_id,
            auth.user_id,
            input.display_name.as_deref(),
            input.description.as_deref(),
            &input.trigger_words,
            &input.trigger_when,
            &input.callback_urls,
            None,
            &token,
            true,
        )
        .await?;

    Ok(Json(webhook))
}

async fn get_outgoing_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<OutgoingWebhook>> {
    let webhook = IntegrationRepository::new(&state.db)
        .get_outgoing_webhook_by_id(id)
        .await?
        .ok_or_else(|| AppError::WebhookNotFound)?;

    if !auth.can_access_owned(webhook.creator_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Cannot access this webhook".to_string(),
        ));
    }

    Ok(Json(webhook))
}

async fn delete_outgoing_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let webhook = IntegrationRepository::new(&state.db)
        .get_outgoing_webhook_by_id(id)
        .await?
        .ok_or_else(|| AppError::WebhookNotFound)?;

    if !auth.can_access_owned(webhook.creator_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Cannot delete this webhook".to_string(),
        ));
    }

    IntegrationRepository::new(&state.db)
        .delete_outgoing_webhook(id)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Slash Commands ============

async fn list_slash_commands(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<SlashCommand>>> {
    ensure_team_member(&state, query.team_id, auth.user_id).await?;

    let commands = IntegrationRepository::new(&state.db)
        .list_slash_commands_by_team(query.team_id)
        .await?;

    Ok(Json(commands))
}

async fn create_slash_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
    Json(input): Json<CreateSlashCommand>,
) -> ApiResult<Json<SlashCommand>> {
    if !input.trigger.starts_with('/') && input.trigger.len() < 2 {
        return Err(AppError::Validation("Invalid trigger format".to_string()));
    }

    if !is_valid_callback_url(&input.url) {
        return Err(AppError::Validation(
            "Invalid command callback URL: must use http(s) and must not point to internal, loopback, or reserved addresses".to_string(),
        ));
    }

    let token = generate_token();
    let trigger = input.trigger.trim_start_matches('/');

    let command = IntegrationRepository::new(&state.db)
        .create_slash_command(
            query.team_id,
            auth.user_id,
            trigger,
            &input.url,
            &input.method,
            input.display_name.as_deref(),
            input.description.as_deref(),
            input.hint.as_deref(),
            &token,
        )
        .await?;

    Ok(Json(command))
}

async fn get_slash_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SlashCommand>> {
    let command = IntegrationRepository::new(&state.db)
        .get_slash_command_by_id(id)
        .await?
        .ok_or_else(|| AppError::CommandNotFound)?;

    if !auth.can_access_owned(command.creator_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Cannot access this command".to_string(),
        ));
    }

    Ok(Json(command))
}

async fn delete_slash_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let command = IntegrationRepository::new(&state.db)
        .get_slash_command_by_id(id)
        .await?
        .ok_or_else(|| AppError::CommandNotFound)?;

    if !auth.can_access_owned(command.creator_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Cannot delete this command".to_string(),
        ));
    }

    IntegrationRepository::new(&state.db)
        .delete_slash_command(id)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn execute_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<ExecuteCommand>,
) -> ApiResult<Json<CommandResponse>> {
    let response = execute_command_internal(
        &state,
        CommandAuth {
            user_id: auth.user_id,
            email: auth.email,
            role: auth.role,
        },
        payload,
    )
    .await?;

    Ok(Json(response))
}

const HELP_TEXT: &str = r#"**Available Commands:**
• `/call [end]` - Start or end a video call
• `/join ~channel` - Join a channel
• `/leave` - Leave current channel
• `/me [action]` - Post an action message
• `/shrug [message]` - Add ¯\_(ツ)_/¯ to your message
• `/echo [text]` - Echo text back to you"#;

fn build_command_response(
    response_type: impl Into<String>,
    text: impl Into<String>,
    goto_location: Option<String>,
    attachments: Option<serde_json::Value>,
) -> CommandResponse {
    CommandResponse {
        response_type: response_type.into(),
        text: text.into(),
        username: None,
        icon_url: None,
        goto_location,
        attachments,
    }
}

fn parse_command_input(command: &str) -> ApiResult<(&str, String)> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(AppError::BadRequest("Empty command".to_string()));
    }
    let trigger = parts[0].trim_start_matches('/');
    let args = if parts.len() > 1 {
        parts[1..].join(" ")
    } else {
        String::new()
    };
    Ok((trigger, args))
}

async fn resolve_team_id(state: &AppState, payload: &ExecuteCommand) -> ApiResult<Uuid> {
    if let Some(tid) = payload.team_id {
        Ok(tid)
    } else {
        ChannelRepository::new(&state.db)
            .get_team_id(payload.channel_id)
            .await?
            .ok_or_else(|| AppError::ChannelNotFound)
    }
}

async fn execute_call_command(
    state: &AppState,
    auth: &CommandAuth,
    payload: &ExecuteCommand,
    args: &str,
) -> ApiResult<CommandResponse> {
    let db_value: Option<String> = sqlx::query_scalar(
        "SELECT plugins->'calls'->>'enabled' FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await?;

    tracing::info!(
        "Calls enabled - DB value: {:?}, Env value: {}",
        db_value,
        state.config.calls.enabled
    );

    let calls_enabled = db_value
        .as_ref()
        .map(|v| v.parse::<bool>().unwrap_or(false))
        .unwrap_or(state.config.calls.enabled);

    tracing::info!("Calls enabled - Final result: {}", calls_enabled);

    if !calls_enabled {
        let db_val_clone = db_value.clone();
        return Ok(build_command_response(
            "ephemeral",
            format!(
                "Calls are not enabled (db: {:?}, env: {})",
                db_val_clone, state.config.calls.enabled
            ),
            None,
            None,
        ));
    }

    let user = UserRepository::new(&state.db)
        .get_by_id_unchecked(auth.user_id)
        .await?
        .ok_or_else(|| AppError::UserNotFound)?;

    let call_manager = state.call_state_manager.as_ref();

    if args == "end" || args == "stop" {
        if let Some(call) = call_manager.get_call_by_channel(&payload.channel_id).await {
            let participants = call_manager.get_participants(call.call_id).await;
            for participant in participants {
                call_manager
                    .remove_participant(call.call_id, participant.user_id)
                    .await;

                let event = crate::realtime::WsEnvelope {
                    msg_type: "event".to_string(),
                    event: "custom_com.mattermost.calls_user_left".to_string(),
                    seq: None,
                    channel_id: Some(payload.channel_id),
                    data: serde_json::json!({
                        "channel_id": encode_mm_id(payload.channel_id),
                        "user_id": encode_mm_id(participant.user_id),
                    }),
                    broadcast: Some(crate::realtime::WsBroadcast {
                        channel_id: Some(payload.channel_id),
                        team_id: None,
                        user_id: None,
                        exclude_user_id: None,
                    }),
                };
                state.ws_hub.broadcast(event).await;
            }

            call_manager.remove_call(call.call_id).await;
            state.sfu_manager.remove_sfu(call.call_id).await;

            let event = crate::realtime::WsEnvelope {
                msg_type: "event".to_string(),
                event: "custom_com.mattermost.calls_call_end".to_string(),
                seq: None,
                channel_id: Some(payload.channel_id),
                data: serde_json::json!({
                    "channel_id": encode_mm_id(payload.channel_id),
                    "call_id": encode_mm_id(call.call_id),
                }),
                broadcast: Some(crate::realtime::WsBroadcast {
                    channel_id: Some(payload.channel_id),
                    team_id: None,
                    user_id: None,
                    exclude_user_id: None,
                }),
            };
            state.ws_hub.broadcast(event).await;

            return Ok(build_command_response(
                "ephemeral",
                "Call ended",
                None,
                None,
            ));
        }

        return Ok(build_command_response(
            "ephemeral",
            "No active call found in this channel",
            None,
            None,
        ));
    }

    let now = Utc::now().timestamp_millis();
    let channel_id = payload.channel_id;

    if let Some(existing_call) = call_manager.get_call_by_channel(&channel_id).await {
        if call_manager
            .get_participant(existing_call.call_id, auth.user_id)
            .await
            .is_none()
        {
            let participant = Participant {
                user_id: auth.user_id,
                session_id: uuid::Uuid::new_v4(),
                joined_at: now,
                muted: true,
                screen_sharing: false,
                hand_raised: false,
            };

            call_manager
                .add_participant(existing_call.call_id, participant.clone())
                .await;

            if let Ok(sfu) = state
                .sfu_manager
                .get_or_create_sfu(existing_call.call_id)
                .await
            {
                let _ = sfu
                    .add_participant(auth.user_id, participant.session_id)
                    .await;
            }

            let event = crate::realtime::WsEnvelope {
                msg_type: "event".to_string(),
                event: "custom_com.mattermost.calls_user_joined".to_string(),
                seq: None,
                channel_id: Some(channel_id),
                data: serde_json::json!({
                    "channel_id": encode_mm_id(channel_id),
                    "user_id": encode_mm_id(auth.user_id),
                    "session_id": encode_mm_id(participant.session_id),
                    "muted": true,
                    "raised_hand": false,
                }),
                broadcast: Some(crate::realtime::WsBroadcast {
                    channel_id: Some(channel_id),
                    team_id: None,
                    user_id: None,
                    exclude_user_id: None,
                }),
            };
            state.ws_hub.broadcast(event).await;
        }

        let attachments = serde_json::json!([
            {
                "color": "#166de0",
                "title": "RustChat Call",
                "text": "A call is in progress. Click to join.",
                "actions": [
                    {
                        "id": "join_call",
                        "name": "Join Call",
                        "type": "button",
                        "style": "primary",
                        "integration": {
                            "url": format!("/plugins/com.mattermost.calls/calls/{}/join", encode_mm_id(channel_id)),
                            "context": { "action": "join_call" }
                        }
                    }
                ]
            }
        ]);

        return Ok(build_command_response(
            "in_channel",
            format!("@{} joined the call", user.username),
            None,
            Some(attachments),
        ));
    }

    let call_id = uuid::Uuid::new_v4();
    let call = CallState {
        call_id,
        channel_id,
        owner_id: auth.user_id,
        host_id: auth.user_id,
        started_at: now,
        participants: std::collections::HashMap::new(),
        screen_sharer: None,
        thread_id: None,
        dismissed_users: std::collections::HashSet::new(),
    };

    call_manager.add_call(call).await;

    let participant = Participant {
        user_id: auth.user_id,
        session_id: uuid::Uuid::new_v4(),
        joined_at: now,
        muted: true,
        screen_sharing: false,
        hand_raised: false,
    };

    call_manager
        .add_participant(call_id, participant.clone())
        .await;

    if let Ok(sfu) = state.sfu_manager.get_or_create_sfu(call_id).await {
        let _ = sfu
            .add_participant(auth.user_id, participant.session_id)
            .await;
    }

    let event = crate::realtime::WsEnvelope {
        msg_type: "event".to_string(),
        event: "custom_com.mattermost.calls_call_start".to_string(),
        seq: None,
        channel_id: Some(channel_id),
        data: serde_json::json!({
            "channel_id": encode_mm_id(channel_id),
            "user_id": encode_mm_id(auth.user_id),
            "call_id": encode_mm_id(call_id),
            "start_at": now.to_string(),
            "owner_id": encode_mm_id(auth.user_id),
        }),
        broadcast: Some(crate::realtime::WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: Some(auth.user_id),
        }),
    };
    state.ws_hub.broadcast(event).await;

    let event = crate::realtime::WsEnvelope {
        msg_type: "event".to_string(),
        event: "custom_com.mattermost.calls_user_joined".to_string(),
        seq: None,
        channel_id: Some(channel_id),
        data: serde_json::json!({
            "channel_id": encode_mm_id(channel_id),
            "user_id": encode_mm_id(auth.user_id),
            "session_id": encode_mm_id(participant.session_id),
            "muted": true,
            "raised_hand": false,
        }),
        broadcast: Some(crate::realtime::WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        }),
    };
    state.ws_hub.broadcast(event).await;

    let attachments = serde_json::json!([
        {
            "color": "#166de0",
            "title": "RustChat Call",
            "text": "A call has started. Click to join.",
            "actions": [
                {
                    "id": "join_call",
                    "name": "Join Call",
                    "type": "button",
                    "style": "primary",
                    "integration": {
                        "url": format!("/plugins/com.mattermost.calls/calls/{}/join", encode_mm_id(channel_id)),
                        "context": { "action": "join_call" }
                    }
                }
            ]
        }
    ]);

    let props = serde_json::json!({
        "type": "custom_calls",
        "attachments": attachments,
        "call": {
            "call_id": encode_mm_id(call_id),
            "channel_id": encode_mm_id(channel_id),
        }
    });

    let create_post_input = crate::models::CreatePost {
        message: format!("Video call started by @ {}", user.username),
        file_ids: vec![],
        props: Some(props),
        root_post_id: None,
        client_msg_id: None,
    };

    let _ = crate::services::posts::create_post(
        state,
        auth.user_id,
        channel_id,
        create_post_input,
        None,
    )
    .await?;

    Ok(build_command_response(
        "ephemeral",
        "Call started",
        None,
        None,
    ))
}

async fn execute_join_command(
    state: &AppState,
    auth: &CommandAuth,
    payload: &ExecuteCommand,
    args: &str,
) -> ApiResult<CommandResponse> {
    if args.is_empty() {
        return Ok(build_command_response(
            "ephemeral",
            "Usage: /join ~channel-name",
            None,
            None,
        ));
    }

    let channel_name = args.trim().trim_start_matches('~');

    let current_team_id = ChannelRepository::new(&state.db)
        .get_team_id(payload.channel_id)
        .await?
        .ok_or_else(|| AppError::ChannelNotFound)?;

    let target_channel = ChannelRepository::new(&state.db)
        .find_by_team_and_name(current_team_id, channel_name)
        .await?;

    if let Some(ch) = target_channel {
        ChannelRepository::new(&state.db)
            .add_member(ch.id, auth.user_id, "member")
            .await?;

        Ok(build_command_response(
            "ephemeral",
            format!("You have joined ~{}", ch.name),
            Some(format!("/channels/{}", ch.id)),
            None,
        ))
    } else {
        Ok(build_command_response(
            "ephemeral",
            format!("Channel ~{} not found", channel_name),
            None,
            None,
        ))
    }
}

async fn execute_leave_command(
    state: &AppState,
    auth: &CommandAuth,
    payload: &ExecuteCommand,
) -> ApiResult<CommandResponse> {
    let channel = ChannelRepository::new(&state.db)
        .get_by_id_optional(payload.channel_id)
        .await?;

    if let Some(ch) = channel {
        if ch.channel_type == crate::models::ChannelType::Direct {
            return Ok(build_command_response(
                "ephemeral",
                "You cannot leave a direct message channel",
                None,
                None,
            ));
        }

        ChannelRepository::new(&state.db)
            .remove_member(payload.channel_id, auth.user_id)
            .await?;

        // Revoke WebSocket subscription
        state
            .ws_hub
            .unsubscribe_channel(auth.user_id, payload.channel_id)
            .await;

        let event = crate::realtime::WsEnvelope::event(
            crate::realtime::EventType::MemberRemoved,
            serde_json::json!({
                "channel_id": payload.channel_id,
                "user_id": auth.user_id
            }),
            Some(payload.channel_id),
        )
        .with_broadcast(crate::realtime::WsBroadcast {
            channel_id: Some(payload.channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(event).await;

        Ok(build_command_response(
            "ephemeral",
            format!("You have left ~{}", ch.name),
            Some("/".to_string()),
            None,
        ))
    } else {
        Ok(build_command_response(
            "ephemeral",
            "Channel not found",
            None,
            None,
        ))
    }
}

async fn execute_me_command(
    state: &AppState,
    auth: &CommandAuth,
    payload: &ExecuteCommand,
    args: &str,
) -> ApiResult<CommandResponse> {
    let user_name = UserRepository::new(&state.db)
        .get_username(auth.user_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "someone".to_string());

    let message = format!("*{} {}*", user_name, args);

    let create_post_input = crate::models::CreatePost {
        message,
        file_ids: vec![],
        props: Some(serde_json::json!({"from_command": "/me"})),
        root_post_id: None,
        client_msg_id: None,
    };

    let _ = crate::services::posts::create_post(
        state,
        auth.user_id,
        payload.channel_id,
        create_post_input,
        None,
    )
    .await?;

    Ok(build_command_response("ephemeral", "", None, None))
}

async fn try_execute_builtin_command(
    state: &AppState,
    auth: &CommandAuth,
    payload: &ExecuteCommand,
    trigger: &str,
    args: &str,
) -> Option<ApiResult<CommandResponse>> {
    match trigger {
        "call" => Some(execute_call_command(state, auth, payload, args).await),
        "echo" => Some(Ok(build_command_response(
            "ephemeral",
            format!("Echo: {}", args),
            None,
            None,
        ))),
        "shrug" => Some(Ok(build_command_response(
            "in_channel",
            format!("{} ¯\\_(ツ)_/¯", args),
            None,
            None,
        ))),
        "invite" => Some(Ok(build_command_response(
            "ephemeral",
            format!("Invitation sent to {}", args),
            None,
            None,
        ))),
        "join" => Some(execute_join_command(state, auth, payload, args).await),
        "leave" => Some(execute_leave_command(state, auth, payload).await),
        "me" => Some(execute_me_command(state, auth, payload, args).await),
        "help" => Some(Ok(build_command_response(
            "ephemeral",
            HELP_TEXT,
            None,
            None,
        ))),
        _ => None,
    }
}

async fn execute_custom_slash_command(
    state: &AppState,
    auth: &CommandAuth,
    payload: &ExecuteCommand,
    trigger: &str,
    args: &str,
) -> ApiResult<CommandResponse> {
    let team_id = resolve_team_id(state, payload).await?;
    let command = IntegrationRepository::new(&state.db)
        .get_slash_command_by_team_and_trigger(team_id, trigger)
        .await?;

    if let Some(cmd) = command {
        let user_name = UserRepository::new(&state.db)
            .get_username(auth.user_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());

        let channel_name = ChannelRepository::new(&state.db)
            .get_name(payload.channel_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());

        let Some((client, parsed_url)) = callback_http_client(&cmd.url).await else {
            return Ok(build_command_response(
                "ephemeral",
                "Command URL is not valid or points to an internal address",
                None,
                None,
            ));
        };

        let payload_out = OutgoingWebhookPayload {
            token: cmd.token.clone(),
            team_id: cmd.team_id,
            channel_id: payload.channel_id,
            channel_name,
            user_id: auth.user_id,
            user_name,
            text: args.to_string(),
            trigger_word: trigger.to_string(),
        };

        // The HTTP client above enforces a 10-second per-request timeout. Use a
        // single attempt so the total wall-clock time for slash-command execution
        // stays at approximately 10 seconds.
        let retry_config = RetryConfig {
            max_attempts: 1,
            initial_delay: Duration::from_millis(150),
            max_delay: Duration::from_secs(2),
            backoff_multiplier: 2.0,
            retry_if: RetryCondition::Default,
        };

        // `client` is bound to the validated IP addresses for `cmd.url` by
        // `callback_http_client`, so this request cannot be DNS-rebound to an
        // internal address even though the URL still contains the original hostname.
        let res = send_reqwest_with_retry(
            client.post(parsed_url.as_str()).json(&payload_out),
            &retry_config,
            |e| AppError::Internal(format!("Command execution failed: {}", e)),
            || {
                AppError::Internal(
                    "Command execution failed: request could not be cloned for retry".to_string(),
                )
            },
        )
        .await?;

        if res.status().is_success() {
            let resp_body =
                res.json::<CommandResponse>()
                    .await
                    .unwrap_or_else(|_| CommandResponse {
                        response_type: "ephemeral".to_string(),
                        text: "Command executed successfully (no response body)".to_string(),
                        username: None,
                        icon_url: None,
                        goto_location: None,
                        attachments: None,
                    });
            Ok(resp_body)
        } else {
            Ok(build_command_response(
                "ephemeral",
                format!("Command failed with status: {}", res.status()),
                None,
                None,
            ))
        }
    } else {
        Ok(build_command_response(
            "ephemeral",
            format!("Command /{} not found", trigger),
            None,
            None,
        ))
    }
}

pub async fn execute_slash_command(
    state: &AppState,
    auth: CommandAuth,
    payload: ExecuteCommand,
) -> ApiResult<CommandResponse> {
    let (trigger, args) = parse_command_input(&payload.command)?;
    if let Some(result) = try_execute_builtin_command(state, &auth, &payload, trigger, &args).await
    {
        return result;
    }
    execute_custom_slash_command(state, &auth, &payload, trigger, &args).await
}

pub async fn execute_command_internal(
    state: &AppState,
    auth: CommandAuth,
    payload: ExecuteCommand,
) -> ApiResult<CommandResponse> {
    execute_slash_command(state, auth, payload).await
}

// ============ Bots ============

async fn list_bots(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<Vec<Bot>>> {
    let bots = if auth.has_permission(&permissions::ADMIN_FULL) {
        IntegrationRepository::new(&state.db).list_bots().await?
    } else {
        IntegrationRepository::new(&state.db)
            .list_bots_by_owner(auth.user_id)
            .await?
    };

    Ok(Json(bots))
}

async fn create_bot(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateBot>,
) -> ApiResult<Json<Bot>> {
    // Create a user account for the bot
    let bot_username = format!(
        "bot_{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );
    let bot_email = format!("{}@bot.rustchat.local", bot_username);

    let bot_user_id = IntegrationRepository::new(&state.db)
        .create_bot_user(&bot_username, &bot_email)
        .await?;

    let bot = IntegrationRepository::new(&state.db)
        .create_bot(
            bot_user_id,
            auth.user_id,
            &input.display_name,
            input.description.as_deref(),
        )
        .await?;

    Ok(Json(bot))
}

async fn get_bot(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Bot>> {
    let bot = IntegrationRepository::new(&state.db)
        .get_bot_by_id(id)
        .await?
        .ok_or_else(|| AppError::BotNotFound)?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::CannotAccessBot);
    }

    Ok(Json(bot))
}

async fn delete_bot(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let bot = IntegrationRepository::new(&state.db)
        .get_bot_by_id(id)
        .await?
        .ok_or_else(|| AppError::BotNotFound)?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot delete this bot".to_string()));
    }

    IntegrationRepository::new(&state.db).delete_bot(id).await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn list_bot_tokens(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<BotToken>>> {
    let bot = IntegrationRepository::new(&state.db)
        .get_bot_by_id(id)
        .await?
        .ok_or_else(|| AppError::BotNotFound)?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::CannotAccessBot);
    }

    let tokens = IntegrationRepository::new(&state.db)
        .list_bot_tokens(id)
        .await?;

    Ok(Json(tokens))
}

#[derive(Debug, Deserialize)]
pub struct CreateBotTokenRequest {
    pub description: Option<String>,
}

async fn create_bot_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateBotTokenRequest>,
) -> ApiResult<Json<BotToken>> {
    let bot = IntegrationRepository::new(&state.db)
        .get_bot_by_id(id)
        .await?
        .ok_or_else(|| AppError::BotNotFound)?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::CannotAccessBot);
    }

    let token = generate_token();

    let bot_token = IntegrationRepository::new(&state.db)
        .create_bot_token(id, &token, input.description.as_deref())
        .await?;

    Ok(Json(bot_token))
}

async fn revoke_bot_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((bot_id, token_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let bot = IntegrationRepository::new(&state.db)
        .get_bot_by_id(bot_id)
        .await?
        .ok_or_else(|| AppError::BotNotFound)?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::CannotAccessBot);
    }

    IntegrationRepository::new(&state.db)
        .delete_bot_token(token_id, bot_id)
        .await?;

    Ok(Json(serde_json::json!({"status": "revoked"})))
}
