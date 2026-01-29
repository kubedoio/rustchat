//! Integrations API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    Bot, BotToken, CommandResponse, CreateBot, CreateIncomingWebhook, CreateOutgoingWebhook,
    CreateSlashCommand, ExecuteCommand, IncomingWebhook, OutgoingWebhook, OutgoingWebhookPayload,
    SlashCommand, WebhookPayload, MiroTalkConfig,
};
use crate::mattermost_compat::id::encode_mm_id;
use crate::services::mirotalk::MiroTalkClient;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::Utc;
use url::Url;

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
pub struct CommandAuth {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamQuery {
    pub team_id: Uuid,
}

// ============ Incoming Webhooks ============

async fn list_incoming_webhooks(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<IncomingWebhook>>> {
    let webhooks: Vec<IncomingWebhook> = sqlx::query_as(
        "SELECT * FROM incoming_webhooks WHERE team_id = $1 ORDER BY created_at DESC",
    )
    .bind(query.team_id)
    .fetch_all(&state.db)
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

    let webhook: IncomingWebhook = sqlx::query_as(
        r#"
        INSERT INTO incoming_webhooks (team_id, channel_id, creator_id, display_name, description, token)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(query.team_id)
    .bind(input.channel_id)
    .bind(auth.user_id)
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(&token)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(webhook))
}

async fn get_incoming_webhook(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<IncomingWebhook>> {
    let webhook: IncomingWebhook = sqlx::query_as("SELECT * FROM incoming_webhooks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    Ok(Json(webhook))
}

async fn delete_incoming_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let webhook: IncomingWebhook = sqlx::query_as("SELECT * FROM incoming_webhooks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    if webhook.creator_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Cannot delete this webhook".to_string(),
        ));
    }

    sqlx::query("DELETE FROM incoming_webhooks WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// Execute an incoming webhook (external service posts here)
async fn execute_incoming_webhook(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<WebhookPayload>,
) -> ApiResult<Json<serde_json::Value>> {
    let webhook: IncomingWebhook =
        sqlx::query_as("SELECT * FROM incoming_webhooks WHERE token = $1 AND is_active = true")
            .bind(&token)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid webhook token".to_string()))?;

    // Create a post in the channel
    sqlx::query(
        r#"
        INSERT INTO posts (channel_id, user_id, message, props)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(webhook.channel_id)
    .bind(webhook.creator_id) // Use webhook creator as poster
    .bind(&payload.text)
    .bind(&payload.props)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

// ============ Outgoing Webhooks ============

async fn list_outgoing_webhooks(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<OutgoingWebhook>>> {
    let webhooks: Vec<OutgoingWebhook> = sqlx::query_as(
        "SELECT * FROM outgoing_webhooks WHERE team_id = $1 ORDER BY created_at DESC",
    )
    .bind(query.team_id)
    .fetch_all(&state.db)
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

    let webhook: OutgoingWebhook = sqlx::query_as(
        r#"
        INSERT INTO outgoing_webhooks 
        (team_id, channel_id, creator_id, display_name, description, trigger_words, trigger_when, callback_urls, token)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(query.team_id)
    .bind(input.channel_id)
    .bind(auth.user_id)
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(&input.trigger_words)
    .bind(&input.trigger_when)
    .bind(&input.callback_urls)
    .bind(&token)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(webhook))
}

async fn get_outgoing_webhook(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<OutgoingWebhook>> {
    let webhook: OutgoingWebhook = sqlx::query_as("SELECT * FROM outgoing_webhooks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    Ok(Json(webhook))
}

async fn delete_outgoing_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let webhook: OutgoingWebhook = sqlx::query_as("SELECT * FROM outgoing_webhooks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    if webhook.creator_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Cannot delete this webhook".to_string(),
        ));
    }

    sqlx::query("DELETE FROM outgoing_webhooks WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Slash Commands ============

async fn list_slash_commands(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<SlashCommand>>> {
    let commands: Vec<SlashCommand> =
        sqlx::query_as("SELECT * FROM slash_commands WHERE team_id = $1 ORDER BY trigger")
            .bind(query.team_id)
            .fetch_all(&state.db)
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

    let command: SlashCommand = sqlx::query_as(
        r#"
        INSERT INTO slash_commands 
        (team_id, creator_id, trigger, url, method, display_name, description, hint, token)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(query.team_id)
    .bind(auth.user_id)
    .bind(trigger)
    .bind(&input.url)
    .bind(&input.method)
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(&input.hint)
    .bind(&token)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(command))
}

async fn get_slash_command(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SlashCommand>> {
    let command: SlashCommand = sqlx::query_as("SELECT * FROM slash_commands WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Command not found".to_string()))?;

    Ok(Json(command))
}

async fn delete_slash_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let command: SlashCommand = sqlx::query_as("SELECT * FROM slash_commands WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Command not found".to_string()))?;

    if command.creator_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Cannot delete this command".to_string(),
        ));
    }

    sqlx::query("DELETE FROM slash_commands WHERE id = $1")
        .bind(id)
        .execute(&state.db)
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
            let config: MiroTalkConfig = sqlx::query_as(
                "SELECT * FROM mirotalk_config WHERE is_active = true",
            )
            .fetch_optional(&state.db)
            .await?
            .unwrap_or_else(|| MiroTalkConfig {
                is_active: true,
                mode: crate::models::MiroTalkMode::Disabled,
                base_url: "".to_string(),
                api_key_secret: "".to_string(),
                default_room_prefix: None,
                join_behavior: crate::models::JoinBehavior::NewTab,
                updated_at: Utc::now(),
                updated_by: None,
            });

            if !config.is_enabled() {
                return Ok(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: "MiroTalk integration is not enabled".to_string(),
                    username: None,
                    icon_url: None,
                    goto_location: None,
                    attachments: None,
                });
            }

            let user = sqlx::query_as::<_, crate::models::User>(
                "SELECT * FROM users WHERE id = $1",
            )
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await?;

            let display_name = user
                .display_name
                .clone()
                .unwrap_or_else(|| user.username.clone());

            if args == "end" || args == "stop" {
                let existing: Option<crate::models::post::Post> = sqlx::query_as(
                    "SELECT * FROM posts WHERE channel_id = $1 AND props->>'type' = 'custom_calls' ORDER BY created_at DESC LIMIT 1",
                )
                .bind(payload.channel_id)
                .fetch_optional(&state.db)
                .await?;

                if let Some(post) = existing {
                    let mut props = post.props.as_object().cloned().unwrap_or_default();
                    props.insert("ended".to_string(), serde_json::Value::Bool(true));
                    props.insert("attachments".to_string(), serde_json::Value::Array(vec![]));

                    let updated: crate::models::post::PostResponse = sqlx::query_as(
                        r#"
                        WITH updated_post AS (
                            UPDATE posts SET message = $1, props = $2, edited_at = NOW()
                            WHERE id = $3
                            RETURNING *
                        )
                        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                               p.reply_count::int8 as reply_count,
                               p.last_reply_at, p.seq,
                               u.username, u.avatar_url, u.email
                        FROM updated_post p
                        LEFT JOIN users u ON p.user_id = u.id
                        "#,
                    )
                    .bind(format!("Video call ended by @{}", user.username))
                    .bind(serde_json::Value::Object(props))
                    .bind(post.id)
                    .fetch_one(&state.db)
                    .await?;

                    let broadcast = crate::realtime::WsEnvelope::event(
                        crate::realtime::EventType::MessageUpdated,
                        updated.clone(),
                        Some(post.channel_id),
                    )
                    .with_broadcast(crate::realtime::WsBroadcast {
                        channel_id: Some(post.channel_id),
                        team_id: None,
                        user_id: None,
                        exclude_user_id: None,
                    });
                    state.ws_hub.broadcast(broadcast).await;

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

            let channel_id_mm = encode_mm_id(payload.channel_id);
            let salt = if config.api_key_secret.is_empty() {
                "rustchat".to_string()
            } else {
                config.api_key_secret.clone()
            };
            let room_seed = format!("{}:{}", channel_id_mm, salt);
            let room_id = URL_SAFE_NO_PAD.encode(room_seed.as_bytes());

            let client = MiroTalkClient::new(config.clone(), state.http_client.clone())?;
            let _ = client.create_meeting(&room_id).await?;

            let mut join_url = Url::parse(&config.base_url)
                .map_err(|_| AppError::Config("Invalid MiroTalk base URL".to_string()))?;
            if let Ok(mut segments) = join_url.path_segments_mut() {
                segments.pop_if_empty();
                segments.push("join");
                segments.push(&room_id);
            }
            join_url.query_pairs_mut().append_pair("name", &display_name);

            let attachments = serde_json::json!([
                {
                    "color": "#166de0",
                    "title": "Mirotalk Conference",
                    "text": "The meeting is active. Tap to join.",
                    "actions": [
                        {
                            "id": "join_call",
                            "name": "Join Meeting",
                            "type": "button",
                            "style": "primary",
                            "integration": {
                                "url": join_url.to_string(),
                                "context": { "action": "open_url" }
                            }
                        }
                    ]
                }
            ]);

            let props = serde_json::json!({
                "type": "custom_calls",
                "attachments": attachments,
                "call": {
                    "room_id": room_id
                }
            });

            let create_post_input = crate::models::CreatePost {
                message: format!("Video call started by @{}", user.username),
                file_ids: vec![],
                props: Some(props),
                root_post_id: None,
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
        _ => {}
    }

    // 3. Look up custom slash commands
    // We need team_id. If not provided in payload (it's optional), try to get from channel.
    let team_id = if let Some(tid) = payload.team_id {
        tid
    } else {
        let channel = sqlx::query!(
            "SELECT team_id FROM channels WHERE id = $1",
            payload.channel_id
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Channel not found".to_string()))?;
        channel.team_id
    };

    let command = sqlx::query_as::<_, SlashCommand>(
        "SELECT * FROM slash_commands WHERE team_id = $1 AND trigger = $2",
    )
    .bind(team_id)
    .bind(trigger)
    .fetch_optional(&state.db)
    .await?;

    if let Some(cmd) = command {
        // Fetch username
        let user_name = sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        // Fetch channel name
        let channel_name =
            sqlx::query_scalar::<_, String>("SELECT name FROM channels WHERE id = $1")
                .bind(payload.channel_id)
                .fetch_one(&state.db)
                .await
                .unwrap_or_else(|_| "unknown".to_string());

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

        let res = client
            .post(&cmd.url)
            .json(&payload_out)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Command execution failed: {}", e)))?;

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
    let bots: Vec<Bot> = if auth.role == "system_admin" {
        sqlx::query_as("SELECT * FROM bots ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?
    } else {
        sqlx::query_as("SELECT * FROM bots WHERE owner_id = $1 ORDER BY created_at DESC")
            .bind(auth.user_id)
            .fetch_all(&state.db)
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

    let bot_user_id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO users (username, email, password_hash, is_bot, role)
        VALUES ($1, $2, 'BOT_NO_PASSWORD', true, 'member')
        RETURNING id
        "#,
    )
    .bind(&bot_username)
    .bind(&bot_email)
    .fetch_one(&state.db)
    .await?;

    let bot: Bot = sqlx::query_as(
        r#"
        INSERT INTO bots (user_id, owner_id, display_name, description)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(bot_user_id.0)
    .bind(auth.user_id)
    .bind(&input.display_name)
    .bind(&input.description)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(bot))
}

async fn get_bot(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Bot>> {
    let bot: Bot = sqlx::query_as("SELECT * FROM bots WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    Ok(Json(bot))
}

async fn delete_bot(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let bot: Bot = sqlx::query_as("SELECT * FROM bots WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if bot.owner_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Cannot delete this bot".to_string()));
    }

    sqlx::query("DELETE FROM bots WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn list_bot_tokens(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<BotToken>>> {
    let bot: Bot = sqlx::query_as("SELECT * FROM bots WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if bot.owner_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Cannot access this bot".to_string()));
    }

    let tokens: Vec<BotToken> =
        sqlx::query_as("SELECT * FROM bot_tokens WHERE bot_id = $1 ORDER BY created_at DESC")
            .bind(id)
            .fetch_all(&state.db)
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
    let bot: Bot = sqlx::query_as("SELECT * FROM bots WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if bot.owner_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Cannot access this bot".to_string()));
    }

    let token = generate_token();

    let bot_token: BotToken = sqlx::query_as(
        r#"
        INSERT INTO bot_tokens (bot_id, token, description)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&token)
    .bind(&input.description)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(bot_token))
}

async fn revoke_bot_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((bot_id, token_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let bot: Bot = sqlx::query_as("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found".to_string()))?;

    if bot.owner_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Cannot access this bot".to_string()));
    }

    sqlx::query("DELETE FROM bot_tokens WHERE id = $1 AND bot_id = $2")
        .bind(token_id)
        .bind(bot_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "revoked"})))
}
