use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
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
            "/commands/{command_id}",
            get(get_command).put(update_command).delete(delete_command),
        )
        .route("/commands/{command_id}/move", put(move_command))
        .route("/commands/{command_id}/regen_token", post(regenerate_command_token))
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

async fn get_command(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_command_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn update_command(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_command_id): Path<String>,
    Json(_command): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn delete_command(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_command_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn move_command(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_command_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn regenerate_command_token(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_command_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"token": ""})))
}
