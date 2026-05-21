use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::collections::HashMap;
use uuid::Uuid;

use super::ensure_team_member;
use super::utils::{pagination_from_group_query, GroupAssociationQuery};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

pub async fn get_team_groups(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Query(query): Query<GroupAssociationQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        ensure_team_member(&state, team_id, auth.user_id).await?;
    }

    let search_term = query.q.clone().unwrap_or_default().to_ascii_lowercase();
    let filter_allow_reference = should_filter_allow_reference(&auth, &query);
    let (paginate, offset, per_page) = pagination_from_group_query(&query);

    let rows: Vec<TeamGroupRow> = sqlx::query_as(
        r#"
        SELECT
            g.id,
            g.name,
            g.display_name,
            g.description,
            g.source,
            g.remote_id,
            g.allow_reference,
            g.created_at,
            g.updated_at,
            g.deleted_at,
            gs.scheme_admin,
            EXISTS(
                SELECT 1
                FROM group_syncables gs2
                WHERE gs2.group_id = g.id
                  AND gs2.delete_at IS NULL
            ) AS has_syncables,
            (
                SELECT COUNT(*)
                FROM group_members gm
                WHERE gm.group_id = g.id
            ) AS member_count
        FROM groups g
        JOIN group_syncables gs
          ON gs.group_id = g.id
         AND gs.syncable_type = 'team'
         AND gs.syncable_id = $1
         AND gs.delete_at IS NULL
        WHERE g.deleted_at IS NULL
          AND ($2 = FALSE OR g.allow_reference = TRUE)
          AND (
                $3 = ''
                OR LOWER(COALESCE(g.name, '')) LIKE $4
                OR LOWER(g.display_name) LIKE $4
          )
        ORDER BY g.display_name ASC
        "#,
    )
    .bind(team_id)
    .bind(filter_allow_reference)
    .bind(&search_term)
    .bind(format!("%{}%", search_term))
    .fetch_all(&state.db)
    .await?;

    let total_group_count = rows.len();
    let paged_rows = if paginate {
        rows.into_iter()
            .skip(offset)
            .take(per_page)
            .collect::<Vec<_>>()
    } else {
        rows
    };

    Ok(Json(serde_json::json!({
        "groups": paged_rows.iter().map(team_group_json).collect::<Vec<_>>(),
        "total_group_count": total_group_count
    })))
}

pub async fn get_team_groups_by_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Query(query): Query<GroupAssociationQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        ensure_team_member(&state, team_id, auth.user_id).await?;
    }

    let search_term = query.q.clone().unwrap_or_default().to_ascii_lowercase();
    let filter_allow_reference = should_filter_allow_reference(&auth, &query);
    let (paginate, offset, per_page) = pagination_from_group_query(&query);

    let rows: Vec<ChannelGroupByTeamRow> = sqlx::query_as(
        r#"
        SELECT
            c.id AS channel_id,
            g.id AS group_id,
            g.name,
            g.display_name,
            g.description,
            g.source,
            g.remote_id,
            g.allow_reference,
            g.created_at,
            g.updated_at,
            g.deleted_at,
            gs.scheme_admin,
            EXISTS(
                SELECT 1
                FROM group_syncables gs2
                WHERE gs2.group_id = g.id
                  AND gs2.delete_at IS NULL
            ) AS has_syncables,
            (
                SELECT COUNT(*)
                FROM group_members gm
                WHERE gm.group_id = g.id
            ) AS member_count
        FROM channels c
        JOIN group_syncables gs
          ON gs.syncable_type = 'channel'
         AND gs.syncable_id = c.id
         AND gs.delete_at IS NULL
        JOIN groups g ON g.id = gs.group_id
        WHERE c.team_id = $1
          AND g.deleted_at IS NULL
          AND ($2 = FALSE OR g.allow_reference = TRUE)
          AND (
                $3 = ''
                OR LOWER(COALESCE(g.name, '')) LIKE $4
                OR LOWER(g.display_name) LIKE $4
          )
        ORDER BY c.id, g.display_name ASC
        "#,
    )
    .bind(team_id)
    .bind(filter_allow_reference)
    .bind(&search_term)
    .bind(format!("%{}%", search_term))
    .fetch_all(&state.db)
    .await?;

    let mut grouped: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for row in rows {
        grouped
            .entry(encode_mm_id(row.channel_id))
            .or_default()
            .push(team_group_json_from_parts(
                row.group_id,
                row.name,
                row.display_name,
                row.description,
                row.source,
                row.remote_id,
                row.allow_reference,
                row.created_at,
                row.updated_at,
                row.deleted_at,
                row.has_syncables,
                row.member_count,
                row.scheme_admin,
            ));
    }

    if paginate {
        for groups in grouped.values_mut() {
            let paged = groups
                .iter()
                .skip(offset)
                .take(per_page)
                .cloned()
                .collect::<Vec<_>>();
            *groups = paged;
        }
    }

    Ok(Json(serde_json::json!({
        "groups": grouped
    })))
}

fn should_filter_allow_reference(auth: &MmAuthUser, query: &GroupAssociationQuery) -> bool {
    let has_system_group_read = auth.has_permission(&permissions::SYSTEM_MANAGE)
        || auth.has_permission(&permissions::ADMIN_FULL);

    query.filter_allow_reference.unwrap_or(false) || !has_system_group_read
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TeamGroupRow {
    id: Uuid,
    name: Option<String>,
    display_name: String,
    description: String,
    source: String,
    remote_id: Option<String>,
    allow_reference: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    scheme_admin: bool,
    has_syncables: bool,
    member_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ChannelGroupByTeamRow {
    channel_id: Uuid,
    group_id: Uuid,
    name: Option<String>,
    display_name: String,
    description: String,
    source: String,
    remote_id: Option<String>,
    allow_reference: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    scheme_admin: bool,
    has_syncables: bool,
    member_count: i64,
}

pub(crate) fn team_group_json(row: &TeamGroupRow) -> serde_json::Value {
    team_group_json_from_parts(
        row.id,
        row.name.clone(),
        row.display_name.clone(),
        row.description.clone(),
        row.source.clone(),
        row.remote_id.clone(),
        row.allow_reference,
        row.created_at,
        row.updated_at,
        row.deleted_at,
        row.has_syncables,
        row.member_count,
        row.scheme_admin,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn team_group_json_from_parts(
    id: Uuid,
    name: Option<String>,
    display_name: String,
    description: String,
    source: String,
    remote_id: Option<String>,
    allow_reference: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    has_syncables: bool,
    member_count: i64,
    scheme_admin: bool,
) -> serde_json::Value {
    serde_json::json!({
        "id": encode_mm_id(id),
        "name": name,
        "display_name": display_name,
        "description": description,
        "source": source,
        "remote_id": remote_id,
        "allow_reference": allow_reference,
        "create_at": created_at.timestamp_millis(),
        "update_at": updated_at.timestamp_millis(),
        "delete_at": deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
        "has_syncables": has_syncables,
        "member_count": member_count,
        "scheme_admin": scheme_admin,
    })
}
