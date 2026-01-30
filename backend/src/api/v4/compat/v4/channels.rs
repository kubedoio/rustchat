use axum::extract::{State, Json};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChannelSearchRequest {
    pub term: String,
    pub team_id: Option<String>,
}

pub async fn create_direct_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(ids): Json<Vec<String>>,
) -> ApiResult<Json<mm::Channel>> {
    if ids.len() != 2 {
        return Err(AppError::BadRequest("Direct channel requires exactly 2 user IDs".to_string()));
    }

    let other_id_str = ids.iter().find(|id| !id.ends_with(&crate::mattermost_compat::id::encode_mm_id(auth.user_id)))
        .or_else(|| ids.get(1))
        .unwrap();

    let other_id = parse_mm_or_uuid(other_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid other user ID".to_string()))?;

    // Call internal channel creation service
    // For now, let's use the core API logic
    let channel = crate::api::v4::channels::create_direct_channel_internal(&state, auth.user_id, other_id).await?;
    
    Ok(Json(channel.into()))
}

pub async fn create_group_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(ids): Json<Vec<String>>,
) -> ApiResult<Json<mm::Channel>> {
    let mut uuids = Vec::new();
    for id in ids {
        let uuid = parse_mm_or_uuid(&id)
            .ok_or_else(|| AppError::BadRequest(format!("Invalid user ID: {}", id)))?;
        uuids.push(uuid);
    }

    let channel = crate::api::v4::channels::create_group_channel_internal(&state, auth.user_id, uuids).await?;
    
    Ok(Json(channel.into()))
}

pub async fn search_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<ChannelSearchRequest>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = input.team_id.and_then(|id| parse_mm_or_uuid(&id));
    
    let term = format!("%{}%", input.term.to_lowercase());

    let channels: Vec<crate::models::channel::Channel> = if let Some(tid) = team_id {
        sqlx::query_as(
            r#"
            SELECT c.* FROM channels c
            JOIN channel_members cm ON c.id = cm.channel_id
            WHERE c.team_id = $1 AND cm.user_id = $2
              AND (LOWER(c.name) LIKE $3 OR LOWER(c.display_name) LIKE $3)
            ORDER BY c.display_name ASC
            "#,
        )
        .bind(tid)
        .bind(auth.user_id)
        .bind(&term)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT c.* FROM channels c
            JOIN channel_members cm ON c.id = cm.channel_id
            WHERE cm.user_id = $1
              AND (LOWER(c.name) LIKE $2 OR LOWER(c.display_name) LIKE $2)
            ORDER BY c.display_name ASC
            "#,
        )
        .bind(auth.user_id)
        .bind(&term)
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(channels.into_iter().map(|c| c.into()).collect()))
}
