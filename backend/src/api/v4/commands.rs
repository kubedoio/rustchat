use axum::{extract::{Path, Query, State}, routing::{get, post}, Json, Router};
use serde::Deserialize;

use crate::api::AppState;
use crate::api::integrations::{execute_command_internal, CommandAuth};
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::parse_mm_or_uuid;
use crate::models::{CommandResponse, ExecuteCommand};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/commands", get(list_commands))
        .route("/commands/execute", post(execute_command))
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

async fn list_commands(Query(_query): Query<CommandsQuery>) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let commands = vec![serde_json::json!({
        "id": "builtin-call",
        "trigger": "call",
        "display_name": "Call",
        "description": "Start a Mirotalk call",
        "auto_complete": true,
        "auto_complete_desc": "Start a Mirotalk call",
        "auto_complete_hint": "[end]",
    })];

    Ok(Json(commands))
}

async fn execute_command(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(payload): Json<ExecuteCommandRequest>,
) -> ApiResult<Json<CommandResponse>> {
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

async fn autocomplete_suggestions(
    Path(_team): Path<TeamPath>,
    Query(query): Query<AutocompleteQuery>,
    _auth: MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let user_input = query.user_input.trim();

    let suggestions = if user_input.starts_with("/call") {
        vec![serde_json::json!({
            "complete": "/call",
            "suggestion": "/call",
            "hint": "[end]",
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
