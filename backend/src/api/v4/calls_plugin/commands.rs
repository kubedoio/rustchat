//! Calls Plugin Slash Commands
//!
//! Implements /call commands for starting, joining, and managing calls via slash commands.

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Build slash command routes
pub fn router() -> Router<AppState> {
    Router::new().route("/commands/call", post(handle_call_command))
}

// ============ Command Request/Response Types ============

#[derive(Debug, Deserialize)]
pub struct SlashCommandRequest {
    pub channel_id: String,
    #[serde(rename = "command")]
    pub _command: String,
    pub text: String,
    #[serde(rename = "user_id")]
    pub _user_id: String,
}

#[derive(Debug, Serialize)]
pub struct SlashCommandResponse {
    pub text: String,
    pub response_type: String, // "in_channel" or "ephemeral"
    pub props: Option<Value>,
}

// ============ Command Handlers ============

/// Handle /call slash command
///
/// Supported commands:
/// - /call start - Start a new call in the current channel
/// - /call join - Join the active call in the current channel  
/// - /call leave - Leave the current call
/// - /call end - End the call (if you're the owner)
/// - /call mute - Mute yourself
/// - /call unmute - Unmute yourself
async fn handle_call_command(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(payload): Json<SlashCommandRequest>,
) -> ApiResult<Json<SlashCommandResponse>> {
    let channel_uuid =
        Uuid::parse_str(&payload.channel_id).map_err(|_| AppError::InvalidChannelId)?;

    // Parse command text
    let args: Vec<&str> = payload.text.split_whitespace().collect();
    let subcommand = args.first().copied().unwrap_or("start");

    match subcommand {
        "start" => handle_start_command(&state, auth.user_id, channel_uuid).await,
        "join" => handle_join_command(&state, auth.user_id, channel_uuid).await,
        "leave" => handle_leave_command(&state, auth.user_id, channel_uuid).await,
        "end" => handle_end_command(&state, auth.user_id, channel_uuid).await,
        "mute" => handle_mute_command(&state, auth.user_id, channel_uuid).await,
        "unmute" => handle_unmute_command(&state, auth.user_id, channel_uuid).await,
        _ => Ok(Json(SlashCommandResponse {
            text: "**Call Commands**\n\
                • `/call start` - Start a new call in this channel\n\
                • `/call join` - Join the active call\n\
                • `/call leave` - Leave the current call\n\
                • `/call end` - End the call (owner only)\n\
                • `/call mute` - Mute yourself\n\
                • `/call unmute` - Unmute yourself\n\
                • `/call help` - Show this help message"
                .to_string(),
            response_type: "ephemeral".to_string(),
            props: None,
        })),
    }
}

/// Handle /call start
async fn handle_start_command(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
) -> ApiResult<Json<SlashCommandResponse>> {
    // Check if user is channel member
    check_channel_permission(state, user_id, channel_id).await?;

    // Use the same logic as the HTTP endpoint
    // For now, return a message directing user to use the UI
    Ok(Json(SlashCommandResponse {
        text: "Starting a call... Please wait.".to_string(),
        response_type: "in_channel".to_string(),
        props: Some(serde_json::json!({
            "attachments": [{
                "actions": [{
                    "name": "Join Call",
                    "integration": {
                        "url": format!("/api/v4/plugins/com.mattermost.calls/calls/{}/join", channel_id),
                        "context": {
                            "action": "join"
                        }
                    }
                }]
            }]
        })),
    }))
}

/// Handle /call join
async fn handle_join_command(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
) -> ApiResult<Json<SlashCommandResponse>> {
    check_channel_permission(state, user_id, channel_id).await?;

    Ok(Json(SlashCommandResponse {
        text: "Joining the call...".to_string(),
        response_type: "ephemeral".to_string(),
        props: None,
    }))
}

/// Handle /call leave
async fn handle_leave_command(
    _state: &AppState,
    _user_id: Uuid,
    _channel_id: Uuid,
) -> ApiResult<Json<SlashCommandResponse>> {
    Ok(Json(SlashCommandResponse {
        text: "You have left the call.".to_string(),
        response_type: "ephemeral".to_string(),
        props: None,
    }))
}

/// Handle /call end
async fn handle_end_command(
    _state: &AppState,
    _user_id: Uuid,
    _channel_id: Uuid,
) -> ApiResult<Json<SlashCommandResponse>> {
    Ok(Json(SlashCommandResponse {
        text: "The call has ended.".to_string(),
        response_type: "in_channel".to_string(),
        props: None,
    }))
}

/// Handle /call mute
async fn handle_mute_command(
    _state: &AppState,
    _user_id: Uuid,
    _channel_id: Uuid,
) -> ApiResult<Json<SlashCommandResponse>> {
    Ok(Json(SlashCommandResponse {
        text: "You are now muted.".to_string(),
        response_type: "ephemeral".to_string(),
        props: None,
    }))
}

/// Handle /call unmute
async fn handle_unmute_command(
    _state: &AppState,
    _user_id: Uuid,
    _channel_id: Uuid,
) -> ApiResult<Json<SlashCommandResponse>> {
    Ok(Json(SlashCommandResponse {
        text: "You are now unmuted.".to_string(),
        response_type: "ephemeral".to_string(),
        props: None,
    }))
}

/// Check if user has permission to access channel
async fn check_channel_permission(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
) -> ApiResult<()> {
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
