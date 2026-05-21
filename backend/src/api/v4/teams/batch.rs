use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use super::utils::parse_body;
use super::{ensure_team_admin_or_system_manage, map_team_member};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use crate::models::TeamMember;
use crate::services::team_membership::apply_default_channel_membership_for_team_join;

#[derive(Deserialize)]
pub struct AddTeamMembersBatchRequest {
    user_ids: Vec<String>,
}

pub async fn add_team_members_batch(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<Vec<mm::TeamMember>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;
    let input: AddTeamMembersBatchRequest = parse_body(&headers, &body, "Invalid batch body")?;
    let mut members = Vec::new();
    for user_id in input.user_ids {
        let user_id = match parse_mm_or_uuid(&user_id) {
            Some(id) => id,
            None => continue,
        };
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
        if let Err(err) =
            apply_default_channel_membership_for_team_join(&state, team_id, user_id).await
        {
            tracing::warn!(
                team_id = %team_id,
                user_id = %user_id,
                error = %err,
                "Default channel auto-join failed after v4 add_team_members_batch"
            );
        }
        members.push(map_team_member(TeamMember {
            team_id,
            user_id,
            role: "member".to_string(),
            created_at: chrono::Utc::now(),
            presence: "offline".to_string(),
        }));
    }
    Ok(Json(members))
}
