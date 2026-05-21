use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ensure_team_member, search_team_channels};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::channel::ChannelType;
use crate::models::{Channel, Team};
use crate::repositories::{ChannelRepository, TeamRepository};

pub async fn get_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;
    let channels: Vec<Channel> = ChannelRepository::new(&state.db)
        .list_team_channels_for_user(team_id, auth.user_id, true, None)
        .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

pub async fn get_team_channel_ids(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;
    ensure_team_member(&state, team_id, auth.user_id).await?;
    let ids = ChannelRepository::new(&state.db)
        .list_team_channel_ids(team_id, auth.user_id)
        .await?;
    Ok(Json(ids.into_iter().map(encode_mm_id).collect()))
}

pub async fn get_team_private_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;
    ensure_team_member(&state, team_id, auth.user_id).await?;
    let channels: Vec<Channel> = ChannelRepository::new(&state.db)
        .list_team_private_channels(team_id, auth.user_id)
        .await?;
    Ok(Json(channels.into_iter().map(|c| c.into()).collect()))
}

pub async fn get_team_deleted_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;
    ensure_team_member(&state, team_id, auth.user_id).await?;
    // Return public archived channels plus private archived channels the user belongs to
    let channels: Vec<Channel> = ChannelRepository::new(&state.db)
        .list_team_deleted_channels(team_id, auth.user_id)
        .await?;
    Ok(Json(channels.into_iter().map(|c| c.into()).collect()))
}

#[derive(Deserialize)]
pub struct ChannelAutocompleteQuery {
    name: Option<String>,
    term: Option<String>,
}

#[derive(Serialize)]
pub struct ChannelAutocompleteResponse {
    pub channels: Vec<mm::Channel>,
    pub users: Vec<mm::User>,
}

pub async fn autocomplete_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Query(query): Query<ChannelAutocompleteQuery>,
) -> ApiResult<Json<ChannelAutocompleteResponse>> {
    let term = query.name.or(query.term).unwrap_or_default();
    let channels = search_team_channels(&state, auth.user_id, &team_id, &term, 20).await?;
    Ok(Json(ChannelAutocompleteResponse {
        channels,
        users: vec![],
    }))
}

pub async fn search_autocomplete_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Query(query): Query<ChannelAutocompleteQuery>,
) -> ApiResult<Json<ChannelAutocompleteResponse>> {
    let term = query.name.or(query.term).unwrap_or_default();
    let channels = search_team_channels(&state, auth.user_id, &team_id, &term, 20).await?;
    Ok(Json(ChannelAutocompleteResponse {
        channels,
        users: vec![],
    }))
}

pub async fn get_team_channel_by_name(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((team_id, channel_name)): Path<(String, String)>,
) -> ApiResult<Json<mm::Channel>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;
    let channel: Channel = ChannelRepository::new(&state.db)
        .get_channel_by_name(team_id, &channel_name)
        .await?
        .ok_or_else(|| AppError::ChannelNotFound)?;

    if channel.channel_type == ChannelType::Private {
        let is_member = ChannelRepository::new(&state.db)
            .is_channel_member(channel.id, auth.user_id)
            .await?;
        if !is_member {
            return Err(AppError::Forbidden(
                "Not a member of this channel".to_string(),
            ));
        }
    }

    Ok(Json(channel.into()))
}

pub async fn get_team_channel_by_name_for_team_name(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((team_name, channel_name)): Path<(String, String)>,
) -> ApiResult<Json<mm::Channel>> {
    let team: Team = TeamRepository::new(&state.db)
        .get_team_by_name(&team_name)
        .await?
        .ok_or_else(|| AppError::TeamNotFound)?;
    get_team_channel_by_name(
        State(state),
        auth,
        Path((encode_mm_id(team.id), channel_name)),
    )
    .await
}
