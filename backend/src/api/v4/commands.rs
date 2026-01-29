use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use chrono::Utc;
use uuid::Uuid;
use sha2::{Sha256, Digest};

use crate::api::AppState;
use crate::api::integrations::{execute_command_internal, CommandAuth};
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use crate::models::{CommandResponse, ExecuteCommand};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/commands", get(list_commands))
        .route("/commands/execute", post(execute_command))
        .route("/commands/autocomplete", get(autocomplete_commands))
        .route(
            "/teams/{team_id}/commands/autocomplete_suggestions",
            get(autocomplete_suggestions),
        )
}

#[derive(Deserialize)]
struct CommandsQuery {
    team_id: Option<String>,
}

#[derive(Deserialize)]
struct ExecuteCommandRequest {
    command: String,
    channel_id: String,
    team_id: Option<String>,
}

#[derive(Deserialize)]
struct AutocompleteQuery {
    user_input: String,
    channel_id: Option<String>,
    root_id: Option<String>,
}

#[derive(Deserialize)]
struct TeamPath {
    team_id: String,
}

#[derive(Deserialize)]
struct AutocompleteParams {
    team_id: String,
}

/// Generate a deterministic 26-char MM-style ID from a seed string
fn generate_mm_id(seed: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let uuid = Uuid::from_slice(&result[..16]).unwrap_or_else(|_| Uuid::new_v4());
    encode_mm_id(uuid)
}

/// Standard built-in Mattermost slash commands
fn get_builtin_commands(team_id: &str) -> Vec<serde_json::Value> {
    let now = Utc::now().timestamp_millis();
    
    vec![
        // Status commands
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:online", team_id)),
            "token": "builtin_online",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "online",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Set your status to Online",
            "auto_complete_hint": "",
            "display_name": "Online",
            "description": "Set your status to Online"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:away", team_id)),
            "token": "builtin_away",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "away",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Set your status to Away",
            "auto_complete_hint": "",
            "display_name": "Away",
            "description": "Set your status to Away"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:dnd", team_id)),
            "token": "builtin_dnd",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "dnd",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Set your status to Do Not Disturb",
            "auto_complete_hint": "",
            "display_name": "Do Not Disturb",
            "description": "Set your status to Do Not Disturb"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:offline", team_id)),
            "token": "builtin_offline",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "offline",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Set your status to Offline",
            "auto_complete_hint": "",
            "display_name": "Offline",
            "description": "Set your status to Offline"
        }),
        // Channel commands
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:join", team_id)),
            "token": "builtin_join",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "join",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Join a channel",
            "auto_complete_hint": "[channel-name]",
            "display_name": "Join",
            "description": "Join a channel"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:leave", team_id)),
            "token": "builtin_leave",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "leave",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Leave the current channel",
            "auto_complete_hint": "",
            "display_name": "Leave",
            "description": "Leave the current channel"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:mute", team_id)),
            "token": "builtin_mute",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "mute",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Mute a channel",
            "auto_complete_hint": "[channel-name]",
            "display_name": "Mute",
            "description": "Mute a channel"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:header", team_id)),
            "token": "builtin_header",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "header",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Set the channel header",
            "auto_complete_hint": "[text]",
            "display_name": "Header",
            "description": "Set the channel header"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:purpose", team_id)),
            "token": "builtin_purpose",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "purpose",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Set the channel purpose",
            "auto_complete_hint": "[text]",
            "display_name": "Purpose",
            "description": "Set the channel purpose"
        }),
        // Message formatting commands
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:me", team_id)),
            "token": "builtin_me",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "me",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Display action text",
            "auto_complete_hint": "[text]",
            "display_name": "Me",
            "description": "Display action text"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:shrug", team_id)),
            "token": "builtin_shrug",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "shrug",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Add a shrug emoji",
            "auto_complete_hint": "[text]",
            "display_name": "Shrug",
            "description": "Add a shrug emoji to your message"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:echo", team_id)),
            "token": "builtin_echo",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "echo",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Echo back your message",
            "auto_complete_hint": "[text]",
            "display_name": "Echo",
            "description": "Echo back your message"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:msg", team_id)),
            "token": "builtin_msg",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "msg",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Send a direct message",
            "auto_complete_hint": "[@username] [message]",
            "display_name": "Message",
            "description": "Send a direct message to a user"
        }),
        // Account commands
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:settings", team_id)),
            "token": "builtin_settings",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "settings",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Open settings menu",
            "auto_complete_hint": "",
            "display_name": "Settings",
            "description": "Open account settings"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:logout", team_id)),
            "token": "builtin_logout",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "logout",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Logout from the application",
            "auto_complete_hint": "",
            "display_name": "Logout",
            "description": "Logout from the application"
        }),
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:help", team_id)),
            "token": "builtin_help",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "help",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Show available commands",
            "auto_complete_hint": "",
            "display_name": "Help",
            "description": "Show available slash commands"
        }),
        // Call commands (Mirotalk integration)
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:call", team_id)),
            "token": "builtin_call",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "call",
            "method": "P",
            "username": "CallBot",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Start or join a Mirotalk video call",
            "auto_complete_hint": "[start/join/end]",
            "display_name": "Call",
            "description": "Video conferencing via Mirotalk"
        }),
        // Invite command
        serde_json::json!({
            "id": generate_mm_id(&format!("{}:invite", team_id)),
            "token": "builtin_invite",
            "create_at": now,
            "update_at": now,
            "delete_at": 0,
            "creator_id": "system",
            "team_id": team_id,
            "trigger": "invite",
            "method": "P",
            "username": "System",
            "icon_url": "",
            "auto_complete": true,
            "auto_complete_desc": "Invite user to channel",
            "auto_complete_hint": "[@username]",
            "display_name": "Invite",
            "description": "Invite a user to the current channel"
        }),
    ]
}

async fn list_commands(Query(query): Query<CommandsQuery>) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Use provided team_id or generate a placeholder for system-level commands
    let team_id = query.team_id.as_deref().unwrap_or("system");
    let team_id_mm = if team_id == "system" {
        generate_mm_id("system")
    } else {
        parse_mm_or_uuid(team_id)
            .map(encode_mm_id)
            .unwrap_or_else(|| generate_mm_id(team_id))
    };
    
    let commands = get_builtin_commands(&team_id_mm);
    Ok(Json(commands))
}

async fn autocomplete_commands(
    Query(params): Query<AutocompleteParams>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Parse team_id to MM format
    let team_id_mm = parse_mm_or_uuid(&params.team_id)
        .map(encode_mm_id)
        .unwrap_or_else(|| generate_mm_id(&params.team_id));
    
    let commands = get_builtin_commands(&team_id_mm);
    Ok(Json(commands))
}

async fn execute_command(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<CommandResponse>> {
    let payload: ExecuteCommandRequest = parse_body(&headers, &body, "Invalid command body")?;
    let channel_id = parse_mm_or_uuid(&payload.channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel_id".to_string()))?;

    let team_id = if let Some(team_id_str) = payload.team_id.as_deref() {
        Some(
            parse_mm_or_uuid(team_id_str)
                .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?,
        )
    } else {
        None
    };

    // Handle built-in status commands directly
    let parts: Vec<&str> = payload.command.split_whitespace().collect();
    if !parts.is_empty() {
        let trigger = parts[0].trim_start_matches('/');
        let args = if parts.len() > 1 {
            parts[1..].join(" ")
        } else {
            String::new()
        };
        
        match trigger {
            "online" | "away" | "dnd" | "offline" => {
                return handle_status_command(state, auth, trigger, channel_id).await;
            }
            "me" => {
                return handle_me_command(state, auth, &args, channel_id).await;
            }
            "msg" => {
                return handle_msg_command(state, auth, &args).await;
            }
            "join" => {
                return handle_join_command(state, auth, &args, team_id).await;
            }
            "leave" => {
                return handle_leave_command(state, auth, channel_id).await;
            }
            "mute" => {
                return handle_mute_command(state, auth, &args, channel_id).await;
            }
            "header" => {
                return handle_header_command(state, auth, &args, channel_id).await;
            }
            "purpose" => {
                return handle_purpose_command(state, auth, &args, channel_id).await;
            }
            "settings" => {
                return Ok(Json(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: "Opening settings...".to_string(),
                    username: None,
                    icon_url: None,
                    goto_location: Some("/settings".to_string()),
                    attachments: None,
                }));
            }
            "logout" => {
                return Ok(Json(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: "Logging out...".to_string(),
                    username: None,
                    icon_url: None,
                    goto_location: Some("/logout".to_string()),
                    attachments: None,
                }));
            }
            "help" => {
                let help_text = r#"**Available Commands:**

**Status:**
`/online` - Set status to Online
`/away` - Set status to Away
`/dnd` - Set status to Do Not Disturb
`/offline` - Set status to Offline

**Channels:**
`/join [channel]` - Join a channel
`/leave` - Leave current channel
`/mute [channel]` - Mute a channel
`/header [text]` - Set channel header
`/purpose [text]` - Set channel purpose

**Messages:**
`/me [text]` - Display action text
`/shrug [text]` - Add shrug emoji
`/echo [text]` - Echo back text
`/msg [@user] [text]` - Send direct message

**Calls:**
`/call start` - Start a Mirotalk video call
`/call end` - End active call

**Other:**
`/invite [@user]` - Invite user to channel
`/settings` - Open settings
`/logout` - Logout
`/help` - Show this help"#;
                
                return Ok(Json(CommandResponse {
                    response_type: "ephemeral".to_string(),
                    text: help_text.to_string(),
                    username: None,
                    icon_url: None,
                    goto_location: None,
                    attachments: None,
                }));
            }
            _ => {}
        }
    }

    // Fall through to internal command handler
    let response = execute_command_internal(
        &state,
        CommandAuth {
            user_id: auth.user_id,
            email: auth.email,
            role: auth.role,
        },
        ExecuteCommand {
            command: payload.command,
            channel_id,
            team_id,
        },
    )
    .await?;

    Ok(Json(response))
}

// Status command handler
async fn handle_status_command(
    state: AppState,
    auth: MmAuthUser,
    status: &str,
    _channel_id: uuid::Uuid,
) -> ApiResult<Json<CommandResponse>> {
    // Update user presence in database
    sqlx::query("UPDATE users SET presence = $1 WHERE id = $2")
        .bind(status)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    // Broadcast status change via WebSocket
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UserUpdated,
        serde_json::json!({
            "user_id": auth.user_id,
            "status": status,
            "manual": true,
            "last_activity_at": Utc::now().timestamp_millis()
        }),
        None,
    );
    state.ws_hub.broadcast(broadcast).await;

    let status_text = match status {
        "online" => "You are now **Online**",
        "away" => "You are now **Away**",
        "dnd" => "You are now in **Do Not Disturb** mode",
        "offline" => "You are now **Offline**",
        _ => "Status updated",
    };

    Ok(Json(CommandResponse {
        response_type: "ephemeral".to_string(),
        text: status_text.to_string(),
        username: None,
        icon_url: None,
        goto_location: None,
        attachments: None,
    }))
}

// /me command handler - posts action text
async fn handle_me_command(
    state: AppState,
    auth: MmAuthUser,
    args: &str,
    channel_id: uuid::Uuid,
) -> ApiResult<Json<CommandResponse>> {
    if args.trim().is_empty() {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "Usage: /me [action text]".to_string(),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }));
    }

    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    let message = format!("*{} {}", user.username, args);
    
    let create_post_input = crate::models::CreatePost {
        message,
        file_ids: vec![],
        props: None,
        root_post_id: None,
    };

    crate::services::posts::create_post(
        &state,
        auth.user_id,
        channel_id,
        create_post_input,
        None,
    )
    .await?;

    Ok(Json(CommandResponse {
        response_type: "ephemeral".to_string(),
        text: "".to_string(),
        username: None,
        icon_url: None,
        goto_location: None,
        attachments: None,
    }))
}

// /msg command handler - send direct message
async fn handle_msg_command(
    _state: AppState,
    _auth: MmAuthUser,
    args: &str,
) -> ApiResult<Json<CommandResponse>> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "Usage: /msg @username [message]".to_string(),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }));
    }

    let username = parts[0].trim_start_matches('@');
    let message = parts[1];

    // For now, return ephemeral message indicating DM should be opened
    // In a full implementation, this would create/find a DM channel and post there
    Ok(Json(CommandResponse {
        response_type: "ephemeral".to_string(),
        text: format!("Direct message to @{}: {}", username, message),
        username: None,
        icon_url: None,
        goto_location: None,
        attachments: None,
    }))
}

// /join command handler - join a channel
async fn handle_join_command(
    state: AppState,
    auth: MmAuthUser,
    args: &str,
    team_id: Option<uuid::Uuid>,
) -> ApiResult<Json<CommandResponse>> {
    let channel_name = args.trim();
    if channel_name.is_empty() {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "Usage: /join [channel-name]".to_string(),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }));
    }

    // Find the channel
    let team_filter = if let Some(tid) = team_id {
        sqlx::query_as::<_, crate::models::Channel>(
            "SELECT * FROM channels WHERE name = $1 AND team_id = $2"
        )
        .bind(channel_name)
        .bind(tid)
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, crate::models::Channel>(
            "SELECT * FROM channels WHERE name = $1"
        )
        .bind(channel_name)
        .fetch_optional(&state.db)
        .await?
    };

    if let Some(channel) = team_filter {
        // Check if already a member
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM channel_members WHERE channel_id = $1 AND user_id = $2"
        )
        .bind(channel.id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if existing > 0 {
            return Ok(Json(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: format!("You are already a member of ~{}", channel_name),
                username: None,
                icon_url: None,
                goto_location: None,
                attachments: None,
            }));
        }

        // Add member
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')"
        )
        .bind(channel.id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

        Ok(Json(CommandResponse {
            response_type: "in_channel".to_string(),
            text: format!("@{} joined the channel.", auth.username),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }))
    } else {
        Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: format!("Channel ~{} not found", channel_name),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }))
    }
}

// /leave command handler - leave current channel
async fn handle_leave_command(
    state: AppState,
    auth: MmAuthUser,
    channel_id: uuid::Uuid,
) -> ApiResult<Json<CommandResponse>> {
    // Remove from channel
    let result = sqlx::query(
        "DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "You are not a member of this channel".to_string(),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }));
    }

    Ok(Json(CommandResponse {
        response_type: "in_channel".to_string(),
        text: format!("@{} left the channel.", auth.username),
        username: None,
        icon_url: None,
        goto_location: None,
        attachments: None,
    }))
}

// /mute command handler - mute a channel
async fn handle_mute_command(
    state: AppState,
    auth: MmAuthUser,
    args: &str,
    _channel_id: uuid::Uuid,
) -> ApiResult<Json<CommandResponse>> {
    let channel_name = args.trim();
    if channel_name.is_empty() {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "Usage: /mute [channel-name]".to_string(),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }));
    }

    // Find the channel
    let channel: Option<crate::models::Channel> = sqlx::query_as(
        "SELECT * FROM channels WHERE name = $1"
    )
    .bind(channel_name)
    .fetch_optional(&state.db)
    .await?;

    if let Some(channel) = channel {
        // Update notification settings to mute
        sqlx::query(
            "UPDATE channel_members SET notify_props = jsonb_set(notify_props, '{mark_unread}', '"mute"') WHERE channel_id = $1 AND user_id = $2"
        )
        .bind(channel.id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

        Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: format!("Channel ~{} has been muted", channel_name),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }))
    } else {
        Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: format!("Channel ~{} not found", channel_name),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }))
    }
}

// /header command handler - set channel header
async fn handle_header_command(
    state: AppState,
    auth: MmAuthUser,
    args: &str,
    channel_id: uuid::Uuid,
) -> ApiResult<Json<CommandResponse>> {
    let header = args.trim();
    if header.is_empty() {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "Usage: /header [text]".to_string(),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }));
    }

    // Update channel header
    sqlx::query("UPDATE channels SET description = $1 WHERE id = $2")
        .bind(header)
        .bind(channel_id)
        .execute(&state.db)
        .await?;

    Ok(Json(CommandResponse {
        response_type: "in_channel".to_string(),
        text: format!("@{} updated the channel header.", auth.username),
        username: None,
        icon_url: None,
        goto_location: None,
        attachments: None,
    }))
}

// /purpose command handler - set channel purpose
async fn handle_purpose_command(
    state: AppState,
    auth: MmAuthUser,
    args: &str,
    channel_id: uuid::Uuid,
) -> ApiResult<Json<CommandResponse>> {
    let purpose = args.trim();
    if purpose.is_empty() {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "Usage: /purpose [text]".to_string(),
            username: None,
            icon_url: None,
            goto_location: None,
            attachments: None,
        }));
    }

    // For now, store purpose in channel props
    sqlx::query(
        "UPDATE channels SET props = jsonb_set(props, '{purpose}', $1) WHERE id = $2"
    )
    .bind(serde_json::json!(purpose))
    .bind(channel_id)
    .execute(&state.db)
    .await?;

    Ok(Json(CommandResponse {
        response_type: "in_channel".to_string(),
        text: format!("@{} updated the channel purpose.", auth.username),
        username: None,
        icon_url: None,
        goto_location: None,
        attachments: None,
    }))
}

fn parse_body<T: serde::de::DeserializeOwned>(
    headers: &axum::http::HeaderMap,
    body: &Bytes,
    message: &str,
) -> ApiResult<T> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else {
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest(message.to_string()))
    }
}

async fn autocomplete_suggestions(
    Path(team): Path<TeamPath>,
    Query(query): Query<AutocompleteQuery>,
    _auth: MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let user_input = query.user_input.trim();

    let suggestions = if user_input.starts_with("/call") {
        vec![serde_json::json!({
            "complete": "/call",
            "suggestion": "/call",
            "hint": "[start/join/end]",
            "description": "Start a Mirotalk call",
        })]
    } else {
        vec![]
    };

    Ok(Json(serde_json::json!({
        "suggestions": suggestions,
        "did_succeed": true
    })))
}
