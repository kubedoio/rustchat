use axum::{
    body::Bytes,
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::user_channels::resolve_team_id;
use super::utils::{UsersByIdsRequest, UsersByUsernamesRequest};
use super::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::{ChannelType, User};
use crate::repositories::{ChannelRepository, TeamRepository, UserRepository};

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub in_team: Option<String>,
    pub in_channel: Option<String>,
    pub name: Option<String>,
    pub limit: Option<i64>,
}

pub async fn autocomplete_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Query(query): Query<AutocompleteQuery>,
) -> ApiResult<Json<Vec<mm::User>>> {
    let limit = query.limit.unwrap_or(25).clamp(1, 200);
    let name = query.name.unwrap_or_default();

    let mut users: Vec<User> = if let Some(channel_id) = query.in_channel {
        let channel_id = parse_mm_or_uuid(&channel_id)
            .ok_or_else(|| AppError::BadRequest("Invalid in_channel".to_string()))?;

        let is_member = ChannelRepository::new(&state.db)
            .is_channel_member(channel_id, auth.user_id)
            .await?;

        if !is_member {
            return Err(AppError::Forbidden(
                "Not a member of this channel".to_string(),
            ));
        }

        UserRepository::new(&state.db)
            .search_channel_members(channel_id, &name, limit)
            .await?
    } else if let Some(team_id) = query.in_team {
        let team_id = resolve_team_id(&state, &team_id).await?;

        let is_member = TeamRepository::new(&state.db)
            .is_team_member(team_id, auth.user_id)
            .await?;

        if !is_member {
            return Err(AppError::NotOnTeam);
        }

        UserRepository::new(&state.db)
            .search_team_members(team_id, &name, limit)
            .await?
    } else {
        UserRepository::new(&state.db)
            .search_active(&name, limit)
            .await?
    };

    users.truncate(limit as usize);
    let mm_users: Vec<mm::User> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(mm_users))
}

#[derive(Deserialize)]
pub struct UserSearchRequest {
    pub term: Option<String>,
    pub team_id: Option<String>,
    #[serde(rename = "not_in_channel_id")]
    pub _not_in_channel_id: Option<String>,
    pub in_channel_id: Option<String>,
    pub limit: Option<i64>,
}

pub async fn search_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::User>>> {
    let input: UserSearchRequest = super::parse_body(&headers, &body, "Invalid search body")?;
    let term = input.term.unwrap_or_default();
    let limit = input.limit.unwrap_or(100).clamp(1, 200) as i64;

    let users: Vec<User> = if let Some(channel_id) = input.in_channel_id {
        let channel_id = parse_mm_or_uuid(&channel_id)
            .ok_or_else(|| AppError::BadRequest("Invalid in_channel_id".to_string()))?;

        let is_member = ChannelRepository::new(&state.db)
            .is_channel_member(channel_id, auth.user_id)
            .await?;

        if !is_member {
            return Err(AppError::Forbidden(
                "Not a member of this channel".to_string(),
            ));
        }

        UserRepository::new(&state.db)
            .search_channel_members(channel_id, &term, limit)
            .await?
    } else if let Some(team_id) = input.team_id {
        let team_id = resolve_team_id(&state, &team_id).await?;

        let is_member = TeamRepository::new(&state.db)
            .is_team_member(team_id, auth.user_id)
            .await?;

        if !is_member {
            return Err(AppError::NotOnTeam);
        }

        UserRepository::new(&state.db)
            .search_team_members(team_id, &term, limit)
            .await?
    } else {
        UserRepository::new(&state.db)
            .search_active(&term, limit)
            .await?
    };

    let mm_users: Vec<mm::User> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(mm_users))
}

pub async fn get_users_by_ids(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    headers: HeaderMap,
    Query(_query): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::User>>> {
    let ids = super::parse_body::<UsersByIdsRequest>(&headers, &body, "Invalid users/ids body")
        .map(|parsed| match parsed {
            UsersByIdsRequest::Ids(ids) => ids,
            UsersByIdsRequest::Wrapped { user_ids } => user_ids,
        })?;

    let uuids: Vec<Uuid> = ids.iter().filter_map(|id| parse_mm_or_uuid(id)).collect();

    if uuids.is_empty() {
        return Ok(Json(vec![]));
    }

    let users = UserRepository::new(&state.db)
        .get_active_by_ids(&uuids)
        .await?;

    let _ = super::status::clear_expired_custom_statuses_for_users(&state, &uuids).await?;

    let mm_users: Vec<mm::User> = users
        .into_iter()
        .map(|mut u| {
            u.clear_custom_status_if_expired();
            u.into()
        })
        .collect();
    Ok(Json(mm_users))
}

pub async fn get_users_by_usernames(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::User>>> {
    // Mattermost clients send a raw JSON array for this endpoint:
    // ["user1","user2"] (not an object wrapper). We also accept
    // {"usernames":[...]} for compatibility with custom clients.
    let usernames =
        super::parse_body::<UsersByUsernamesRequest>(&headers, &body, "Invalid usernames body")
            .map(|parsed| match parsed {
                UsersByUsernamesRequest::Usernames(usernames) => usernames,
                UsersByUsernamesRequest::Wrapped { usernames } => usernames,
            })?;
    if usernames.is_empty() {
        return Ok(Json(vec![]));
    }

    let users = UserRepository::new(&state.db)
        .get_by_usernames(&usernames)
        .await?;

    Ok(Json(users.into_iter().map(|u| u.into()).collect()))
}

pub async fn get_user_by_email(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(email): axum::extract::Path<String>,
) -> ApiResult<Json<mm::User>> {
    use crate::auth::policy::permissions;
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Missing permission to lookup users by email".to_string(),
        ));
    }
    let user = UserRepository::new(&state.db)
        .get_by_email(&email)
        .await?
        .ok_or_else(|| AppError::UserNotFound)?;

    Ok(Json(user.into()))
}

#[derive(Deserialize)]
pub struct UsersQuery {
    pub in_channel: Option<String>,
    pub in_team: Option<String>,
    pub not_in_channel: Option<String>,
    pub not_in_team: Option<String>,
    pub without_team: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Query(query): Query<UsersQuery>,
) -> ApiResult<Json<Vec<mm::User>>> {
    let page = query.page.unwrap_or(0).max(0);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = page * per_page;

    let users = if let Some(in_channel) = query.in_channel.as_deref() {
        let channel_id = parse_mm_or_uuid(in_channel)
            .ok_or_else(|| AppError::BadRequest("Invalid in_channel".to_string()))?;

        let is_member = ChannelRepository::new(&state.db)
            .is_channel_member(channel_id, auth.user_id)
            .await?;

        if !is_member {
            return Err(AppError::Forbidden(
                "Not a member of this channel".to_string(),
            ));
        }

        UserRepository::new(&state.db)
            .list_channel_members_paginated(channel_id, per_page, offset)
            .await?
    } else if let Some(in_team) = query.in_team.as_deref() {
        let team_id = resolve_team_id(&state, in_team).await?;

        let is_member = TeamRepository::new(&state.db)
            .is_team_member(team_id, auth.user_id)
            .await?;

        if !is_member {
            return Err(AppError::NotOnTeam);
        }

        if let Some(not_in_channel) = query.not_in_channel.as_deref() {
            let channel_id = parse_mm_or_uuid(not_in_channel)
                .ok_or_else(|| AppError::BadRequest("Invalid not_in_channel".to_string()))?;

            let channel_repo = ChannelRepository::new(&state.db);
            let channel = channel_repo
                .get_by_id_optional(channel_id)
                .await?
                .ok_or_else(|| AppError::ChannelNotFound)?;

            if channel.team_id != team_id {
                return Err(AppError::BadRequest(
                    "not_in_channel must belong to in_team".to_string(),
                ));
            }

            let is_channel_member = channel_repo
                .is_channel_member(channel_id, auth.user_id)
                .await?;

            if channel.channel_type != ChannelType::Public
                && !is_channel_member
                && !auth.has_permission(&permissions::SYSTEM_MANAGE)
                && !auth.has_permission(&permissions::ADMIN_FULL)
            {
                return Err(AppError::Forbidden(
                    "Not a member of this channel".to_string(),
                ));
            }

            UserRepository::new(&state.db)
                .list_team_members_not_in_channel_paginated(team_id, channel_id, per_page, offset)
                .await?
        } else {
            UserRepository::new(&state.db)
                .list_team_members_paginated(team_id, per_page, offset)
                .await?
        }
    } else if let Some(not_in_team) = query.not_in_team.as_deref() {
        let team_id = resolve_team_id(&state, not_in_team).await?;

        let is_member = TeamRepository::new(&state.db)
            .is_team_member(team_id, auth.user_id)
            .await?;

        if !is_member
            && !auth.has_permission(&permissions::SYSTEM_MANAGE)
            && !auth.has_permission(&permissions::ADMIN_FULL)
        {
            return Err(AppError::NotOnTeam);
        }

        UserRepository::new(&state.db)
            .list_users_not_in_team_paginated(auth.org_id, team_id, per_page, offset)
            .await?
    } else if query.without_team.unwrap_or(false) {
        UserRepository::new(&state.db)
            .list_users_without_team_paginated(auth.org_id, per_page, offset)
            .await?
    } else {
        UserRepository::new(&state.db)
            .list_users(auth.org_id, None, per_page, offset)
            .await?
    };

    let mm_users: Vec<mm::User> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(mm_users))
}

pub async fn get_known_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<String>>> {
    let user_ids = UserRepository::new(&state.db)
        .get_known_user_ids(auth.user_id)
        .await?;

    let ids = user_ids.into_iter().map(encode_mm_id).collect();
    Ok(Json(ids))
}
