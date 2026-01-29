use axum::{extract::Query, Json, Router, routing::get};

use crate::api::AppState;
use crate::error::ApiResult;

pub fn router() -> Router<AppState> {
    Router::new().route("/commands", get(list_commands))
}

#[derive(serde::Deserialize)]
struct CommandsQuery {
    team_id: Option<String>,
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
