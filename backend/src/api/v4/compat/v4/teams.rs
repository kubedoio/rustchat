use axum::extract::{State, Json};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::models::Team;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TeamSearchRequest {
    pub term: String,
}

pub async fn search_teams(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(input): Json<TeamSearchRequest>,
) -> ApiResult<Json<Vec<mm::Team>>> {
    let term = format!("%{}%", input.term.to_lowercase());

    let teams: Vec<Team> = sqlx::query_as(
        r#"
        SELECT * FROM teams 
        WHERE (LOWER(name) LIKE $1 OR LOWER(display_name) LIKE $1)
        ORDER BY display_name ASC
        "#
    )
    .bind(&term)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(teams.into_iter().map(|t| t.into()).collect()))
}
