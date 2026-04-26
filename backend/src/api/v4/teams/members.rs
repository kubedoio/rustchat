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

pub async fn get_team_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::TeamMember>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    ensure_team_member(&state, team_id, auth.user_id).await?;

    // Join with users table to get user information including presence
    #[allow(clippy::type_complexity)]
    let rows: Vec<(Uuid, Uuid, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        r#"
        SELECT
            tm.team_id,
            tm.user_id,
            tm.role,
            u.username,
            u.display_name,
            u.presence
        FROM team_members tm
        JOIN users u ON tm.user_id = u.id
        WHERE tm.team_id = $1
        ORDER BY u.username
        "#,
    )
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    let members: Vec<mm::TeamMember> = rows
        .into_iter()
        .map(
            |(team_id, user_id, role, _username, _display_name, presence)| {
                map_team_member(TeamMember {
                    team_id,
                    user_id,
                    role,
                    created_at: chrono::Utc::now(),
                    presence: presence.unwrap_or_else(|| "offline".to_string()),
                })
            },
        )
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
    sqlx::query(
        r#"
        INSERT INTO team_members (team_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (team_id, user_id)
        DO UPDATE SET role = EXCLUDED.role
        "#,
    )
    .bind(team_id)
    .bind(user_id)
    .bind("member")
    .execute(&state.db)
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
    let member: TeamMember =
        sqlx::query_as("SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .fetch_optional(&state.db)
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
    sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(team_id)
        .bind(user_id)
        .execute(&state.db)
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
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM team_members WHERE team_id = $1")
        .bind(team_id)
        .fetch_all(&state.db)
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
    let member: TeamMember =
        sqlx::query_as("SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
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
    sqlx::query("UPDATE team_members SET role = $1 WHERE team_id = $2 AND user_id = $3")
        .bind(role)
        .bind(team_id)
        .bind(user_id)
        .execute(&state.db)
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
    let _exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    if !_exists {
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
    let member: TeamMember = sqlx::query_as(
        "UPDATE team_members SET role = $1 WHERE team_id = $2 AND user_id = $3 RETURNING *",
    )
    .bind(role)
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Team member not found".to_string()))?;

    Ok(Json(map_team_member(member)))
}
