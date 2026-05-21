use axum::{extract::State, Json};
use serde::Deserialize;
use uuid::Uuid;

use super::MmAuthUser;
use crate::api::AppState;
use crate::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use crate::models::channel::{Channel, ChannelType};
use crate::repositories::{ChannelRepository, TeamRepository};

/// Resolves a team identifier to a UUID.
/// First tries to parse as UUID/mm-id, then falls back to looking up by team name.
pub async fn resolve_team_id(state: &AppState, team_id_str: &str) -> ApiResult<Uuid> {
    // First try to parse as UUID or Mattermost ID
    if let Some(team_id) = parse_mm_or_uuid(team_id_str) {
        return Ok(team_id);
    }

    // Fall back to looking up by team name
    let id = TeamRepository::new(&state.db)
        .get_id_by_name(team_id_str)
        .await?
        .ok_or_else(|| AppError::TeamNotFound)?;

    Ok(id)
}

pub async fn hydrate_dm_display_names_batch(
    repo: &ChannelRepository<'_>,
    channels: &mut [Channel],
    viewer_id: Uuid,
) {
    let dm_ids: Vec<Uuid> = channels
        .iter()
        .filter(|c| c.channel_type == ChannelType::Direct)
        .map(|c| c.id)
        .collect();

    if dm_ids.is_empty() {
        return;
    }

    match repo.get_dm_display_names(&dm_ids, viewer_id).await {
        Ok(names) => {
            for channel in channels
                .iter_mut()
                .filter(|c| c.channel_type == ChannelType::Direct)
            {
                channel.display_name = names.get(&channel.id).cloned();
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to batch hydrate DM display names");
        }
    }
}

#[derive(Deserialize)]
pub struct MyTeamChannelsQuery {
    #[serde(default)]
    pub last_delete_at: i64,
    #[serde(default)]
    pub include_deleted: bool,
}

pub async fn my_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(team_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<MyTeamChannelsQuery>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = resolve_team_id(&state, &team_id).await?;

    tracing::debug!(
        user_id = %auth.user_id,
        team_id = %team_id,
        "Fetching channels for user"
    );

    // Determine the filter condition for deleted channels based on query parameters:
    // 1. If include_deleted is true: return all channels (including deleted ones)
    // 2. If last_delete_at > 0: return non-deleted channels AND channels deleted since last_delete_at
    //    This allows mobile clients to sync deleted channels for cache invalidation
    // 3. Default: only return non-deleted channels
    let last_delete_ts = if query.last_delete_at > 0 {
        Some(chrono::DateTime::from_timestamp_millis(query.last_delete_at).unwrap_or_default())
    } else {
        None
    };

    let repo = ChannelRepository::new(&state.db);
    let mut channels: Vec<Channel> = repo
        .list_team_channels_for_user(team_id, auth.user_id, query.include_deleted, last_delete_ts)
        .await?;

    tracing::debug!(
        user_id = %auth.user_id,
        team_id = %team_id,
        channel_count = channels.len(),
        "Found channels for user"
    );

    hydrate_dm_display_names_batch(&repo, &mut channels, auth.user_id).await;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

pub async fn get_team_channels_for_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path((user_id, team_id)): axum::extract::Path<(String, String)>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let user_id = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let team_id = resolve_team_id(&state, &team_id).await?;
    let repo = ChannelRepository::new(&state.db);
    let mut channels: Vec<Channel> = repo
        .list_team_channels_for_user(team_id, user_id, true, None)
        .await?;

    hydrate_dm_display_names_batch(&repo, &mut channels, user_id).await;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

#[derive(Deserialize)]
pub struct MyChannelsQuery {
    #[serde(default)]
    pub since: i64,
}

pub async fn my_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Query(query): axum::extract::Query<MyChannelsQuery>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let since = if query.since > 0 {
        Some(chrono::DateTime::from_timestamp_millis(query.since).unwrap_or_default())
    } else {
        None
    };

    let repo = ChannelRepository::new(&state.db);
    let mut channels: Vec<Channel> = repo.list_user_channels(auth.user_id, since).await?;

    hydrate_dm_display_names_batch(&repo, &mut channels, auth.user_id).await;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

pub async fn get_channels_for_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let user_id = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let repo = ChannelRepository::new(&state.db);
    let mut channels: Vec<Channel> = repo.list_user_channels(user_id, None).await?;

    hydrate_dm_display_names_batch(&repo, &mut channels, user_id).await;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

#[derive(Deserialize)]
pub struct NotMembersQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn my_team_channels_not_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(team_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<NotMembersQuery>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = resolve_team_id(&state, &team_id).await?;

    let page = query.page.unwrap_or(0).max(0);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = page * per_page;

    let channels: Vec<Channel> = ChannelRepository::new(&state.db)
        .list_not_member_channels(team_id, auth.user_id, per_page, offset)
        .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}
