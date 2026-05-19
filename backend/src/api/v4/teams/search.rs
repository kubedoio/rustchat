use axum::{
    body::Bytes,
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;

use super::utils::parse_body;
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use crate::models::{Channel, Team};

/// POST /teams/{team_id}/channels/search - Search channels in a team
#[derive(Deserialize)]
pub struct SearchChannelsRequest {
    pub term: String,
}

pub async fn search_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;

    let input: SearchChannelsRequest = parse_body(&headers, &body, "Invalid search request")?;
    let search_term = format!("%{}%", input.term.to_lowercase());

    // Search public channels and private channels the user is a member of
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT DISTINCT c.* FROM channels c
        LEFT JOIN channel_members cm ON c.id = cm.channel_id AND cm.user_id = $2
        WHERE c.team_id = $1
          AND c.deleted_at IS NULL
          AND (LOWER(c.name) LIKE $3 OR LOWER(c.display_name) LIKE $3)
          AND (c.type = 'public' OR cm.user_id IS NOT NULL)
        ORDER BY c.display_name ASC
        LIMIT 50
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .bind(&search_term)
    .fetch_all(&state.db)
    .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

pub async fn search_teams(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<HashMap<String, String>>,
) -> ApiResult<Json<Vec<mm::Team>>> {
    let term = input.get("term").map(|s| s.as_str()).unwrap_or_default();

    let teams: Vec<Team> = sqlx::query_as(
        r#"
        SELECT t.* FROM teams t
        JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = $1
          AND (t.name ILIKE $2 OR t.display_name ILIKE $2)
        "#,
    )
    .bind(auth.user_id)
    .bind(format!("%{}%", term))
    .fetch_all(&state.db)
    .await?;

    Ok(Json(teams.into_iter().map(|t| t.into()).collect()))
}

