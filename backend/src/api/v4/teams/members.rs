use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{ensure_team_admin_or_system_manage, ensure_team_member, map_team_member};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::TeamMember;
use crate::repositories::{TeamRepository, UserRepository};

pub async fn get_team_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::TeamMember>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_member(&state, team_id, auth.user_id).await?;

    // Join with users table to get user information including presence
    let rows = TeamRepository::new(&state.db)
        .list_team_members_with_presence(team_id)
        .await?;

    let members: Vec<mm::TeamMember> = rows
        .into_iter()
        .map(|(team_id, user_id, role, presence)| {
            map_team_member(TeamMember {
                team_id,
                user_id,
                role,
                created_at: chrono::Utc::now(),
                presence: presence.unwrap_or_else(|| "offline".to_string()),
            })
        })
        .collect();

    Ok(Json(members))
}

#[derive(Deserialize)]
pub struct AddTeamMemberRequest {
    pub user_id: String,
    #[allow(dead_code)]
    pub roles: Option<String>,
}

pub async fn add_team_member(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<mm::TeamMember>> {
    use super::utils::parse_body;
    use crate::services::team_membership::apply_default_channel_membership_for_team_join;

    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;
    let input: AddTeamMemberRequest = parse_body(&headers, &body, "Invalid member body")?;
    let user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?;
    TeamRepository::new(&state.db)
        .upsert_team_member(team_id, user_id, "member")
        .await?;
    if let Err(err) = apply_default_channel_membership_for_team_join(&state, team_id, user_id).await
    {
        tracing::warn!(
            team_id = %team_id,
            user_id = %user_id,
            error = %err,
            "Default channel auto-join failed after v4 add_team_member"
        );
    }

    Ok(Json(map_team_member(TeamMember {
        team_id,
        user_id,
        role: "member".to_string(),
        created_at: chrono::Utc::now(),
        presence: "offline".to_string(),
    })))
}

pub async fn get_team_member(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((team_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<mm::TeamMember>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?;
    ensure_team_member(&state, team_id, auth.user_id).await?;
    let member = TeamRepository::new(&state.db)
        .get_team_member(team_id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Team member not found".to_string()))?;

    Ok(Json(map_team_member(member)))
}

pub async fn remove_team_member(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((team_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    use super::utils::status_ok;

    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;
    let user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?;
    TeamRepository::new(&state.db)
        .remove_member(team_id, user_id)
        .await?;
    Ok(status_ok())
}

pub async fn get_team_member_ids(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_member(&state, team_id, auth.user_id).await?;
    let ids = TeamRepository::new(&state.db)
        .list_team_member_ids(team_id)
        .await?;
    Ok(Json(ids.into_iter().map(encode_mm_id).collect()))
}

pub async fn get_team_member_me(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<mm::TeamMember>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let member = TeamRepository::new(&state.db)
        .get_team_member(team_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::Forbidden("Not a member of this team".to_string()))?;

    Ok(Json(map_team_member(member)))
}

#[derive(Deserialize)]
pub struct TeamMemberRolesRequest {
    roles: String,
}

pub async fn update_team_member_roles(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((team_id, user_id)): Path<(String, String)>,
    Json(input): Json<TeamMemberRolesRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use super::utils::status_ok;

    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;
    let user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?;
    let role = if input.roles.contains("team_admin") {
        "admin"
    } else {
        "member"
    };
    TeamRepository::new(&state.db)
        .update_team_member_role(team_id, user_id, role)
        .await?;
    Ok(status_ok())
}

#[derive(Deserialize)]
pub struct TeamMemberSchemeRolesRequest {
    scheme_admin: Option<bool>,
    #[allow(dead_code)]
    scheme_user: Option<bool>,
    scheme_guest: Option<bool>,
}

pub async fn update_team_member_scheme_roles(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((team_id, user_id)): Path<(String, String)>,
    Json(input): Json<TeamMemberSchemeRolesRequest>,
) -> ApiResult<Json<mm::TeamMember>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;

    // Verify target user exists
    let exists = UserRepository::new(&state.db).exists(user_id).await?;
    if !exists {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    // Determine role from scheme flags
    let role = if input.scheme_admin == Some(true) {
        "admin"
    } else if input.scheme_guest == Some(true) {
        "guest"
    } else {
        "member"
    };

    // Update the role; also verify they are an existing team member
    let member = TeamRepository::new(&state.db)
        .update_team_member_role_returning(team_id, user_id, role)
        .await?
        .ok_or_else(|| AppError::NotFound("Team member not found".to_string()))?;

    Ok(Json(map_team_member(member)))
}
