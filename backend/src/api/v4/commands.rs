use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::id::encode_mm_id;
use crate::mattermost_compat::models::Command;
use crate::models::{MiroTalkConfig, MiroTalkMode};
use crate::services::mirotalk::MiroTalkClient;

use super::extractors::MmAuthUser;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/commands/autocomplete", get(list_commands))
        .route("/commands/execute", post(execute_command))
}

#[derive(Deserialize)]
struct CommandsQuery {
    team_id: Option<String>,
}

async fn list_commands(
    _auth: MmAuthUser,
    Query(query): Query<CommandsQuery>,
) -> ApiResult<Json<Vec<Command>>> {
    let team_id = query
        .team_id
        .unwrap_or_else(|| encode_mm_id(Uuid::new_v4()));

    Ok(Json(get_all_commands(&team_id)))
}

fn get_all_commands(team_id: &str) -> Vec<Command> {
    let mut commands = Vec::new();

    // Helper to create a command
    let create_cmd = |trigger: &str, name: &str, desc: &str, hint: &str| -> Command {
        Command {
            id: encode_mm_id(Uuid::new_v4()),
            token: "internal_token".to_string(),
            create_at: Utc::now().timestamp_millis(),
            update_at: Utc::now().timestamp_millis(),
            delete_at: 0,
            creator_id: "system".to_string(),
            team_id: team_id.to_string(),
            trigger: trigger.to_string(),
            method: "P".to_string(),
            username: "System".to_string(),
            icon_url: "".to_string(),
            auto_complete: true,
            auto_complete_desc: desc.to_string(),
            auto_complete_hint: hint.to_string(),
            display_name: name.to_string(),
            description: desc.to_string(),
        }
    };

    // Standard commands
    commands.push(create_cmd("online", "Online", "Set status to Online", ""));
    commands.push(create_cmd("away", "Away", "Set status to Away", ""));
    commands.push(create_cmd(
        "dnd",
        "Do Not Disturb",
        "Set status to Do Not Disturb",
        "",
    ));
    commands.push(create_cmd(
        "offline",
        "Offline",
        "Set status to Offline",
        "",
    ));
    commands.push(create_cmd(
        "me",
        "Me",
        "Displays text as an action",
        "[message]",
    ));
    commands.push(create_cmd(
        "shrug",
        "Shrug",
        "Appends ¯\\_(ツ)_/¯ to your message",
        "[message]",
    ));
    commands.push(create_cmd(
        "header",
        "Header",
        "Edit the channel header",
        "[text]",
    ));
    commands.push(create_cmd(
        "purpose",
        "Purpose",
        "Edit the channel purpose",
        "[text]",
    ));
    commands.push(create_cmd(
        "join",
        "Join",
        "Join the open channel",
        "[channel]",
    ));
    commands.push(create_cmd(
        "leave",
        "Leave",
        "Leave the current channel",
        "",
    ));
    commands.push(create_cmd(
        "mute",
        "Mute",
        "Turns off desktop, email and push notifications for the current channel or the [channel] specified.",
        "[channel]",
    ));
    commands.push(create_cmd(
        "msg",
        "Message",
        "Send a Direct Message to a user",
        "[@username] [message]",
    ));
    commands.push(create_cmd("help", "Help", "Mattermost Help", ""));
    commands.push(create_cmd(
        "settings",
        "Settings",
        "Open the Account Settings dialog",
        "",
    ));
    commands.push(create_cmd("logout", "Logout", "Logout of Mattermost", ""));
    commands.push(create_cmd(
        "echo",
        "Echo",
        "Echo back the message",
        "[message]",
    ));

    // Custom commands
    commands.push(create_cmd(
        "call",
        "Call",
        "Start a Mirotalk video call",
        "[start/join]",
    ));

    commands
}

#[derive(Deserialize)]
struct ExecuteCommandRequest {
    #[allow(dead_code)]
    channel_id: String,
    command: String,
    #[allow(dead_code)]
    team_id: Option<String>,
}

#[derive(Serialize)]
struct CommandResponse {
    response_type: String, // "in_channel" or "ephemeral"
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    props: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    goto_location: Option<String>,
}

async fn execute_command(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<ExecuteCommandRequest>,
) -> ApiResult<Json<CommandResponse>> {
    let parts: Vec<&str> = input.command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "Empty command".to_string(),
            props: None,
            goto_location: None,
        }));
    }

    let trigger = parts[0].trim_start_matches('/');

    match trigger {
        "shrug" => {
            let msg = if parts.len() > 1 {
                parts[1..].join(" ") + " ¯\\_(ツ)_/¯"
            } else {
                "¯\\_(ツ)_/¯".to_string()
            };
            Ok(Json(CommandResponse {
                response_type: "in_channel".to_string(),
                text: msg,
                props: None,
                goto_location: None,
            }))
        }
        "call" => handle_call_command(state, auth, parts).await,
        "online" => handle_status_command(state, auth, "online").await,
        "away" => handle_status_command(state, auth, "away").await,
        "dnd" => handle_status_command(state, auth, "dnd").await,
        "offline" => handle_status_command(state, auth, "offline").await,
        "logout" => {
            // Client side usually handles this, but we can return a hint
            Ok(Json(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: "Please use the Logout button in the main menu.".to_string(),
                props: None,
                goto_location: None,
            }))
        }
        "join" | "leave" | "mute" | "header" | "purpose" | "me" | "echo" | "msg" | "settings"
        | "help" => {
            // These are client-side or complex to implement fully without more logic.
            // For now return not implemented message or handle simply.
            Ok(Json(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: format!("Command /{} is not yet implemented on the server.", trigger),
                props: None,
                goto_location: None,
            }))
        }
        _ => Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: format!("Command /{} not found.", trigger),
            props: None,
            goto_location: None,
        })),
    }
}

async fn handle_call_command(
    state: AppState,
    auth: MmAuthUser,
    _parts: Vec<&str>,
) -> ApiResult<Json<CommandResponse>> {
    // 1. Get MiroTalk config
    let config: MiroTalkConfig =
        sqlx::query_as("SELECT * FROM mirotalk_config WHERE is_active = true")
            .fetch_optional(&state.db)
            .await?
            .unwrap_or_else(|| MiroTalkConfig {
                is_active: true,
                mode: MiroTalkMode::Disabled,
                base_url: "".to_string(),
                api_key_secret: "".to_string(),
                default_room_prefix: None,
                join_behavior: crate::models::JoinBehavior::NewTab,
                updated_at: Utc::now(),
                updated_by: None,
            });

    if !config.is_enabled() {
        return Ok(Json(CommandResponse {
            response_type: "ephemeral".to_string(),
            text: "MiroTalk integration is disabled.".to_string(),
            props: None,
            goto_location: None,
        }));
    }

    // 2. Generate room name
    let prefix = config
        .default_room_prefix
        .clone()
        .unwrap_or_else(|| "rustchat".to_string());
    let timestamp = Utc::now().timestamp();
    // Using user ID as part of the room name for uniqueness initiated by user
    let room_name = format!("{}-{}-{}", prefix, auth.user_id, timestamp);

    // 3. Create meeting via client
    let client = MiroTalkClient::new(config.clone(), state.http_client.clone())?;
    let meeting_url = match client.create_meeting(&room_name).await {
        Ok(url) => url,
        Err(e) => {
            return Ok(Json(CommandResponse {
                response_type: "ephemeral".to_string(),
                text: format!("Failed to create meeting: {}", e),
                props: None,
                goto_location: None,
            }));
        }
    };

    // 4. Construct response
    let props = serde_json::json!({
        "attachments": [
            {
                "title": "Join Meeting",
                "title_link": meeting_url,
                "color": "#2389D7",
                "text": format!("Video call started by @{}", auth.email) // email or username if available? auth has email.
            }
        ],
        "meeting_link": meeting_url // Extra prop just in case
    });

    Ok(Json(CommandResponse {
        response_type: "in_channel".to_string(),
        text: "Video Call Started".to_string(),
        props: Some(props),
        goto_location: None,
    }))
}

async fn handle_status_command(
    state: AppState,
    auth: MmAuthUser,
    status: &str,
) -> ApiResult<Json<CommandResponse>> {
    sqlx::query("UPDATE users SET presence = $1 WHERE id = $2")
        .bind(status)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    // Broadcast status change
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

    Ok(Json(CommandResponse {
        response_type: "ephemeral".to_string(),
        text: format!("Status updated to {}", status),
        props: None,
        goto_location: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_commands() {
        let team_id = encode_mm_id(Uuid::new_v4());
        let commands = get_all_commands(&team_id);

        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.trigger == "call"));
        assert!(commands.iter().any(|c| c.trigger == "shrug"));
        assert!(commands.iter().any(|c| c.trigger == "online"));

        let call_cmd = commands.iter().find(|c| c.trigger == "call").unwrap();
        assert_eq!(call_cmd.team_id, team_id);
        assert_eq!(call_cmd.display_name, "Call");
    }
}
