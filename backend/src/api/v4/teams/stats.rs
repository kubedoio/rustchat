use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use super::ensure_team_member;
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use uuid::Uuid;

pub async fn get_team_stats(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;
    ensure_team_member(&state, team_id, auth.user_id).await?;
    let total_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM team_members WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(&state.db)
            .await?;
    let active_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM team_members tm
        JOIN users u ON u.id = tm.user_id
        WHERE tm.team_id = $1 AND u.is_active = true
        "#,
    )
    .bind(team_id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(serde_json::json!({
        "team_id": encode_mm_id(team_id),
        "total_member_count": total_count,
        "active_member_count": active_count,
    })))
}

#[derive(Deserialize)]
pub struct MembersMinusGroupQuery {
    group_id: Option<String>,
    channel_id: Option<String>,
}

pub async fn get_team_members_minus_group_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Query(query): Query<MembersMinusGroupQuery>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    ensure_team_member(&state, team_id, auth.user_id).await?;

    #[derive(sqlx::FromRow)]
    struct MemberRow {
        user_id: Uuid,
        username: String,
        email: String,
        team_role: String,
    }

    let rows: Vec<MemberRow> = if let Some(ref group_id_str) = query.group_id {
        let group_id = parse_mm_or_uuid(group_id_str).ok_or_else(|| AppError::InvalidGroupId)?;
        sqlx::query_as(
            r#"
            SELECT u.id AS user_id, u.username, u.email, tm.role AS team_role
            FROM team_members tm
            JOIN users u ON tm.user_id = u.id
            WHERE tm.team_id = $1
              AND NOT EXISTS (
                  SELECT 1 FROM group_members gm
                  WHERE gm.group_id = $2 AND gm.user_id = u.id
              )
            "#,
        )
        .bind(team_id)
        .bind(group_id)
        .fetch_all(&state.db)
        .await?
    } else if let Some(ref channel_id_str) = query.channel_id {
        let channel_id =
            parse_mm_or_uuid(channel_id_str).ok_or_else(|| AppError::InvalidChannelId)?;
        sqlx::query_as(
            r#"
            SELECT u.id AS user_id, u.username, u.email, tm.role AS team_role
            FROM team_members tm
            JOIN users u ON tm.user_id = u.id
            WHERE tm.team_id = $1
              AND NOT EXISTS (
                  SELECT 1 FROM channel_members cm
                  WHERE cm.channel_id = $2 AND cm.user_id = u.id
              )
            "#,
        )
        .bind(team_id)
        .bind(channel_id)
        .fetch_all(&state.db)
        .await?
    } else {
        vec![]
    };

    let result = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "user_id": encode_mm_id(r.user_id),
                "username": r.username,
                "email": r.email,
                "role": r.team_role,
            })
        })
        .collect();

    Ok(Json(result))
}
