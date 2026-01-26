use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::models::Team;

pub fn router() -> Router<AppState> {
    Router::new().route("/teams/{team_id}", get(get_team))
}

async fn get_team(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<mm::Team>> {
    let team: Team = sqlx::query_as("SELECT * FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(team.into()))
}
