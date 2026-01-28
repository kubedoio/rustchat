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
    Ok(Json(vec![]))
}
