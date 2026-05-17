use axum::{body::Bytes, extract::State, Json};
use serde::Deserialize;
use uuid::Uuid;

use super::{mm, parse_mm_or_uuid, ApiResult, AppState, MmAuthUser};
use crate::repositories::{ChannelRepository, TeamRepository, UserRepository};

const KEYCLOAK_GROUP_SOURCE: &str = "plugin_keycloak";

#[derive(Deserialize)]
pub struct DirectChannelRequest {
    pub user_ids: Vec<String>,
}

pub async fn create_direct_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    // Mattermost sends either a plain array ["id1", "id2"] or an object {"user_ids": ["id1", "id2"]}
    // Try parsing as plain array first, then fall back to object format
    let user_ids: Vec<String> = serde_json::from_slice::<Vec<String>>(&body).or_else(|_| {
        super::utils::parse_body::<DirectChannelRequest>(&headers, &body, "Invalid user_ids")
            .map(|req| req.user_ids)
    })?;

    if user_ids.len() != 2 {
        return Err(crate::error::AppError::BadRequest(
            "Request body must contain exactly 2 user IDs".to_string(),
        ));
    }

    let ids: Vec<Uuid> = user_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();

    if ids.len() != 2 {
        return Err(crate::error::AppError::BadRequest(
            "Invalid user IDs provided".to_string(),
        ));
    }

    if !ids.contains(&auth.user_id) {
        return Err(crate::error::AppError::Forbidden(
            "Must include your user id".to_string(),
        ));
    }

    let other_id = if ids[0] == auth.user_id {
        ids[1]
    } else {
        ids[0]
    };

    let channel = create_direct_channel_internal(&state, auth.user_id, other_id).await?;
    Ok(Json(channel.into()))
}

pub async fn enforce_dm_acl_for_users(state: &AppState, user_ids: &[Uuid]) -> ApiResult<()> {
    if !state.config.messaging.dm_acl_enabled {
        return Ok(());
    }

    let mut unique_users = user_ids.to_vec();
    unique_users.sort_unstable();
    unique_users.dedup();

    if unique_users.len() < 2 {
        return Ok(());
    }

    let allowed: bool = if unique_users.len() == 2 {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM groups g
                JOIN group_dm_acl_flags gf ON gf.group_id = g.id AND gf.enabled = TRUE
                JOIN group_members gm1 ON gm1.group_id = g.id
                JOIN group_members gm2 ON gm2.group_id = g.id
                WHERE g.deleted_at IS NULL
                  AND g.source = $3
                  AND gm1.user_id = $1
                  AND gm2.user_id = $2
            )
            "#,
        )
        .bind(unique_users[0])
        .bind(unique_users[1])
        .bind(KEYCLOAK_GROUP_SOURCE)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT gm.group_id
                FROM group_members gm
                JOIN groups g ON g.id = gm.group_id
                JOIN group_dm_acl_flags gf ON gf.group_id = g.id AND gf.enabled = TRUE
                WHERE g.deleted_at IS NULL
                  AND g.source = $1
                  AND gm.user_id = ANY($2)
                GROUP BY gm.group_id
                HAVING COUNT(DISTINCT gm.user_id) = $3
            )
            "#,
        )
        .bind(KEYCLOAK_GROUP_SOURCE)
        .bind(&unique_users)
        .bind(unique_users.len() as i64)
        .fetch_one(&state.db)
        .await?
    };

    if !allowed {
        return Err(crate::error::AppError::Forbidden(
            "Direct and group messaging is restricted by group policy".to_string(),
        ));
    }

    Ok(())
}

pub async fn create_direct_channel_internal(
    state: &AppState,
    creator_id: Uuid,
    other_id: Uuid,
) -> ApiResult<crate::models::channel::Channel> {
    enforce_dm_acl_for_users(state, &[creator_id, other_id]).await?;

    let canonical_name = crate::models::canonical_direct_channel_name(creator_id, other_id);
    let legacy_name = crate::models::legacy_direct_channel_name(creator_id, other_id);
    let mut ids = vec![creator_id, other_id];
    ids.sort();

    let team_repo = TeamRepository::new(&state.db);
    let user_repo = UserRepository::new(&state.db);
    let channel_repo = ChannelRepository::new(&state.db);

    let team_id = team_repo
        .get_first_team_for_user(creator_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .ok_or_else(|| crate::error::AppError::BadRequest("User has no team".to_string()))?;

    let display_name = user_repo
        .get_display_name_or_username(other_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    if let Some(channel) = channel_repo
        .find_direct_channel(team_id, &canonical_name, &legacy_name)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    {
        for user_id in ids {
            channel_repo
                .add_member(channel.id, user_id, "member")
                .await
                .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
        }

        return Ok(channel);
    }

    let channel = channel_repo
        .create_direct_channel(team_id, &canonical_name, &display_name, creator_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    for user_id in ids {
        channel_repo
            .add_member(channel.id, user_id, "member")
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    Ok(channel)
}

/// POST /channels/group - Create group DM (3+ users)
pub async fn create_group_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    // Group DMs also use array format
    let input: DirectChannelRequest =
        super::utils::parse_body(&headers, &body, "Invalid user_ids")?;

    if input.user_ids.len() < 2 {
        return Err(crate::error::AppError::BadRequest(
            "user_ids must contain at least 2 users".to_string(),
        ));
    }

    let uuids: Vec<Uuid> = input
        .user_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();

    let channel = create_group_channel_internal(&state, auth.user_id, uuids).await?;
    Ok(Json(channel.into()))
}

pub async fn create_group_channel_internal(
    state: &AppState,
    creator_id: Uuid,
    user_ids: Vec<Uuid>,
) -> ApiResult<crate::models::channel::Channel> {
    let mut ids = user_ids;
    if !ids.contains(&creator_id) {
        ids.push(creator_id);
    }

    ids.sort();
    ids.dedup();
    enforce_dm_acl_for_users(state, &ids).await?;

    let name = format!(
        "gm_{}",
        ids.iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("_")
    );

    let team_repo = TeamRepository::new(&state.db);
    let user_repo = UserRepository::new(&state.db);
    let channel_repo = ChannelRepository::new(&state.db);

    let team_id = team_repo
        .get_first_team_for_user(creator_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .ok_or_else(|| crate::error::AppError::BadRequest("User has no team".to_string()))?;

    // Generate display name from usernames
    let usernames: Vec<String> = user_repo
        .get_by_ids(&ids)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .into_iter()
        .map(|u| u.username)
        .collect();
    let display_name = usernames.join(", ");

    let channel = channel_repo
        .create_group_channel(team_id, &name, &display_name, creator_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    for user_id in ids {
        channel_repo
            .add_member(channel.id, user_id, "member")
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    Ok(channel)
}
