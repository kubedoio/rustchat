//! Integrations API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::calls::state::{CallState, Participant};
use crate::auth::policy::permissions;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::encode_mm_id;
use crate::middleware::reliability::{send_reqwest_with_retry, RetryCondition, RetryConfig};
use crate::models::{
    Bot, BotToken, CommandResponse, CreateBot, CreateIncomingWebhook, CreateOutgoingWebhook,
    CreateSlashCommand, ExecuteCommand, IncomingWebhook, OutgoingWebhook, OutgoingWebhookPayload,
    SlashCommand, WebhookPayload,
};
use crate::repositories::{ChannelRepository, IntegrationRepository, UserRepository};
use crate::services::webhooks::is_valid_callback_url;
use chrono::Utc;
use std::time::Duration;

/// Generate a secure random token
fn generate_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut token = String::with_capacity(32);
    let hasher_builder = RandomState::new();
    for _ in 0..4 {
        let mut hasher = hasher_builder.build_hasher();
        hasher.write_u128(uuid::Uuid::new_v4().as_u128());
        token.push_str(&format!("{:016x}", hasher.finish()));
    }
    token.truncate(32);
    token
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
        return Err(AppError::Forbidden("Not a member of this team".to_string()));
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
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

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
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

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
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

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
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

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
        .ok_or_else(|| AppError::NotFound("Command not found".to_string()))?;

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
        .ok_or_else(|| AppError::NotFound("Command not found".to_string()))?;

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

pub async fn execute_command_internal(
    state: &AppState,
    auth: CommandAuth,
    payload: ExecuteCommand,
) -> ApiResult<CommandResponse> {
    // 1. Parse trigger
    let parts: Vec<&str> = payload.command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(AppError::BadRequest("Empty command".to_string()));
    }

    let trigger = parts[0].trim_start_matches('/');
    let args = if parts.len() > 1 {
        parts[1..].join(" ")
    } else {
        String::new()
    };

    // 2. Handle built-in commands
    match trigger {
        "call" => {
            // Check if Calls Plugin is enabled (from database or env)
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
                return Ok(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: format!(
                        "Calls are not enabled (db: {:?}, env: {})",
                        db_val_clone, state.config.calls.enabled
                    ),
                    username: None,
                    icon_url: None,
                    goto_location: None,
                    attachments: None,
                });
            }

            let user = UserRepository::new(&state.db)
                .get_by_id_unchecked(auth.user_id)
                .await?
                .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

            // Get call manager
            let call_manager = state.call_state_manager.as_ref();

            // Handle end/stop command
            if args == "end" || args == "stop" {
                // Find active call in channel
                if let Some(call) = call_manager.get_call_by_channel(&payload.channel_id).await {
                    // Remove all participants and end the call
                    let participants = call_manager.get_participants(call.call_id).await;

                    for participant in participants {
                        call_manager
                            .remove_participant(call.call_id, participant.user_id)
                            .await;

                        // Broadcast user_left event
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

                    // Remove the call
                    call_manager.remove_call(call.call_id).await;

                    // Remove SFU if exists
                    state.sfu_manager.remove_sfu(call.call_id).await;

                    // Broadcast call_end event
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

                    return Ok(CommandResponse {
                        response_type: "ephemeral".to_string(),
                        text: "Call ended".to_string(),
                        username: None,
                        icon_url: None,
                        goto_location: None,
                        attachments: None,
                    });
                }

                return Ok(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: "No active call found in this channel".to_string(),
                    username: None,
                    icon_url: None,
                    goto_location: None,
                    attachments: None,
                });
            }

            // Handle start/join command (default is start)
            let now = Utc::now().timestamp_millis();
            let channel_id = payload.channel_id;

            // Check if there's already an active call in this channel
            if let Some(existing_call) = call_manager.get_call_by_channel(&channel_id).await {
                // Join existing call
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

                    // Get or create SFU
                    if let Ok(sfu) = state
                        .sfu_manager
                        .get_or_create_sfu(existing_call.call_id)
                        .await
                    {
                        let _ = sfu
                            .add_participant(auth.user_id, participant.session_id)
                            .await;
                    }

                    // Broadcast user_joined event
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

                return Ok(CommandResponse {
                    response_type: "in_channel".to_string(),
                    text: format!("@{} joined the call", user.username),
                    username: None,
                    icon_url: None,
                    goto_location: None,
                    attachments: Some(attachments),
                });
            }

            // Create new call
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

            // Add owner as first participant
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

            // Get or create SFU
            if let Ok(sfu) = state.sfu_manager.get_or_create_sfu(call_id).await {
                let _ = sfu
                    .add_participant(auth.user_id, participant.session_id)
                    .await;
            }

            // Broadcast call_start event
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

            // Broadcast user_joined event
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

            // Create post in channel
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

            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: "Call started".to_string(),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
        "echo" => {
            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: format!("Echo: {}", args),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
        "shrug" => {
            return Ok(CommandResponse {
                response_type: "in_channel".to_string(),
                text: format!("{} ¯\\_(ツ)_/¯", args),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
        "invite" => {
            // Mock invite
            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: format!("Invitation sent to {}", args),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
        "join" => {
            // Join a channel by name
            if args.is_empty() {
                return Ok(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: "Usage: /join ~channel-name".to_string(),
                    username: None,
                    icon_url: None,
                    goto_location: None,
                    attachments: None,
                });
            }

            let channel_name = args.trim().trim_start_matches('~');

            // Get team_id from current channel
            let current_team_id: Uuid = ChannelRepository::new(&state.db)
                .get_team_id(payload.channel_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Channel not found".to_string()))?;

            // Find channel
            let target_channel = ChannelRepository::new(&state.db)
                .find_by_team_and_name(current_team_id, channel_name)
                .await?;

            if let Some(ch) = target_channel {
                // Add user to channel
                ChannelRepository::new(&state.db)
                    .add_member(ch.id, auth.user_id, "member")
                    .await?;

                return Ok(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: format!("You have joined ~{}", ch.name),
                    username: None,
                    icon_url: None,
                    goto_location: Some(format!("/channels/{}", ch.id)),
                    attachments: None,
                });
            } else {
                return Ok(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: format!("Channel ~{} not found", channel_name),
                    username: None,
                    icon_url: None,
                    goto_location: None,
                    attachments: None,
                });
            }
        }
        "leave" => {
            // Leave current channel
            let channel = ChannelRepository::new(&state.db)
                .get_by_id_optional(payload.channel_id)
                .await?;

            if let Some(ch) = channel {
                if ch.channel_type == crate::models::ChannelType::Direct {
                    return Ok(CommandResponse {
                        response_type: "ephemeral".to_string(),
                        text: "You cannot leave a direct message channel".to_string(),
                        username: None,
                        icon_url: None,
                        goto_location: None,
                        attachments: None,
                    });
                }

                ChannelRepository::new(&state.db)
                    .remove_member(payload.channel_id, auth.user_id)
                    .await?;

                // Broadcast member left
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

                return Ok(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: format!("You have left ~{}", ch.name),
                    username: None,
                    icon_url: None,
                    goto_location: Some("/".to_string()),
                    attachments: None,
                });
            }

            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: "Channel not found".to_string(),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
        "me" => {
            // /me action - creates an italic-style action message
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

            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: String::new(),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
        "help" => {
            let help_text = r#"**Available Commands:**
• `/call [end]` - Start or end a video call
• `/join ~channel` - Join a channel
• `/leave` - Leave current channel
• `/me [action]` - Post an action message
• `/shrug [message]` - Add ¯\_(ツ)_/¯ to your message
• `/echo [text]` - Echo text back to you"#;

            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: help_text.to_string(),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
        _ => {}
    }

    // 3. Look up custom slash commands
    // We need team_id. If not provided in payload (it's optional), try to get from channel.
    let team_id = if let Some(tid) = payload.team_id {
        tid
    } else {
        ChannelRepository::new(&state.db)
            .get_team_id(payload.channel_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Channel not found".to_string()))?
    };

    let command = IntegrationRepository::new(&state.db)
        .get_slash_command_by_team_and_trigger(team_id, trigger)
        .await?;

    if let Some(cmd) = command {
        // Fetch username
        let user_name = UserRepository::new(&state.db)
            .get_username(auth.user_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());

        // Fetch channel name
        let channel_name = ChannelRepository::new(&state.db)
            .get_name(payload.channel_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());

        // Validate slash command URL to prevent SSRF
        if !is_valid_callback_url(&cmd.url) {
            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: "Command URL is not valid or points to an internal address".to_string(),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }

        // Execute external command (HTTP POST)
        let client = reqwest::Client::new();

        let payload_out = OutgoingWebhookPayload {
            token: cmd.token.clone(),
            team_id: cmd.team_id,
            channel_id: payload.channel_id,
            channel_name,
            user_id: auth.user_id,
            user_name,
            text: args,
            trigger_word: trigger.to_string(),
        };

        let retry_config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(150),
            max_delay: Duration::from_secs(2),
            backoff_multiplier: 2.0,
            retry_if: RetryCondition::Default,
        };

        let res = send_reqwest_with_retry(
            client.post(&cmd.url).json(&payload_out),
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
            let resp_body: CommandResponse =
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
            return Ok(resp_body);
        } else {
            return Ok(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: format!("Command failed with status: {}", res.status()),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            });
        }
    }

    Ok(CommandResponse {
        response_type: "ephemeral".to_string(),
        text: format!("Command /{} not found", trigger),
        username: None,
        icon_url: None,
        goto_location: None,
        attachments: None,
    })
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
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot access this bot".to_string()));
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
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

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
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot access this bot".to_string()));
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
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot access this bot".to_string()));
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
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if !auth.can_access_owned(bot.owner_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot access this bot".to_string()));
    }

    IntegrationRepository::new(&state.db)
        .delete_bot_token(token_id, bot_id)
        .await?;

    Ok(Json(serde_json::json!({"status": "revoked"})))
}
