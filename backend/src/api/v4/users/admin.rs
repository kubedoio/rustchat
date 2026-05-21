use axum::{body::Bytes, extract::State, http::HeaderMap, Json};
use serde::Deserialize;

use super::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

pub async fn update_user_auth(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let _value: serde_json::Value = super::parse_body(&headers, &body, "Invalid auth body")?;
    Ok(super::status_ok())
}

pub async fn accept_terms_of_service(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let _value: serde_json::Value = super::parse_body(&headers, &body, "Invalid terms body")?;
    Ok(super::status_ok())
}

pub async fn migrate_auth_ldap(
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let _value: serde_json::Value = super::parse_body(&headers, &body, "Invalid migrate body")?;
    Ok(super::status_ok())
}

pub async fn migrate_auth_saml(
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let _value: serde_json::Value = super::parse_body(&headers, &body, "Invalid migrate body")?;
    Ok(super::status_ok())
}

pub async fn reset_failed_attempts(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    Ok(super::status_ok())
}

/// GET /api/v4/users/{user_id}/oauth/apps/authorized
pub async fn get_authorized_oauth_apps(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    axum::extract::Path(_user_id): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

#[derive(Debug, Deserialize, Default)]
pub struct GroupAssociationQuery {
    pub q: Option<String>,
    pub filter_allow_reference: Option<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct UserGroupRow {
    id: uuid::Uuid,
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
}

pub async fn get_user_groups(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<GroupAssociationQuery>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let user_uuid = if user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&user_id)
            .ok_or_else(|| AppError::InvalidUserId)?
    };

    if user_uuid != auth.user_id && !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Missing permission to view other user's groups".to_string(),
        ));
    }

    let has_system_group_read = auth.has_permission(&permissions::SYSTEM_MANAGE)
        || auth.has_permission(&permissions::ADMIN_FULL);
    let filter_allow_reference =
        query.filter_allow_reference.unwrap_or(false) || !has_system_group_read;
    let search_term = query.q.unwrap_or_default().to_ascii_lowercase();

    let rows: Vec<UserGroupRow> = sqlx::query_as(
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
            EXISTS(
                SELECT 1
                FROM group_syncables gs
                WHERE gs.group_id = g.id
                  AND gs.delete_at IS NULL
            ) AS has_syncables,
            (
                SELECT COUNT(*)
                FROM group_members gm2
                WHERE gm2.group_id = g.id
            ) AS member_count
        FROM groups g
        JOIN group_members gm ON gm.group_id = g.id
        WHERE gm.user_id = $1
          AND g.deleted_at IS NULL
          AND ($2 = FALSE OR g.allow_reference = TRUE)
          AND (
                $3 = ''
                OR LOWER(COALESCE(g.name, '')) LIKE $4
                OR LOWER(g.display_name) LIKE $4
          )
        ORDER BY g.display_name ASC
        "#,
    )
    .bind(user_uuid)
    .bind(filter_allow_reference)
    .bind(&search_term)
    .bind(format!("%{}%", search_term))
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.iter().map(user_group_json).collect()))
}

fn user_group_json(row: &UserGroupRow) -> serde_json::Value {
    serde_json::json!({
        "id": encode_mm_id(row.id),
        "name": row.name,
        "display_name": row.display_name,
        "description": row.description,
        "source": row.source,
        "remote_id": row.remote_id,
        "allow_reference": row.allow_reference,
        "create_at": row.created_at.timestamp_millis(),
        "update_at": row.updated_at.timestamp_millis(),
        "delete_at": row.deleted_at.map(|t| t.timestamp_millis()).unwrap_or(0),
        "has_syncables": row.has_syncables,
        "member_count": row.member_count,
    })
}
