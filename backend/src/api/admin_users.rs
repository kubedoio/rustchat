//! Admin user management endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use uuid::Uuid;

use crate::api::admin::{insert_admin_audit_log, require_admin, require_global_admin};
use crate::api::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::repositories::AdminRepository;
use crate::services::membership_policies::apply_auto_membership_for_new_user;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(list_users).post(create_admin_user))
        .route(
            "/admin/users/{id}",
            patch(update_admin_user).delete(delete_admin_user),
        )
        .route(
            "/admin/users/{id}/deactivate",
            axum::routing::post(deactivate_user),
        )
        .route(
            "/admin/users/{id}/reactivate",
            axum::routing::post(reactivate_user),
        )
        .route("/admin/users/{id}/wipe", axum::routing::post(wipe_user))
}

#[derive(Debug, serde::Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub role: Option<String>,
    pub search: Option<String>,
    pub include_deleted: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
pub struct UsersListResponse {
    pub users: Vec<crate::models::User>,
    pub total: i64,
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListUsersQuery>,
) -> ApiResult<Json<UsersListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    let include_deleted = query.include_deleted.unwrap_or(false);

    let status = match query.status.as_deref() {
        Some("active") => Some(true),
        Some("inactive") => Some(false),
        _ => None,
    };

    let users = AdminRepository::new(&state.db)
        .list_users(
            status,
            query.role.as_deref(),
            query.search.as_deref(),
            include_deleted,
            per_page,
            offset,
        )
        .await?;

    let total = AdminRepository::new(&state.db)
        .count_users(
            status,
            query.role.as_deref(),
            query.search.as_deref(),
            include_deleted,
        )
        .await?;

    Ok(Json(UsersListResponse { users, total }))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
    pub display_name: Option<String>,
}

pub async fn create_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateUserInput>,
) -> ApiResult<Json<crate::models::User>> {
    require_admin(&auth)?;

    let password_hash = crate::auth::hash_password(&input.password)?;
    let role = input.role.unwrap_or_else(|| "member".to_string());

    let user = AdminRepository::new(&state.db)
        .insert_user(
            &input.username,
            &input.email,
            &password_hash,
            &role,
            input.display_name.as_deref(),
        )
        .await?;

    // Apply auto-membership policies for the new user
    match apply_auto_membership_for_new_user(&state, user.id).await {
        Ok(audit_entries) => {
            let success_count = audit_entries
                .iter()
                .filter(|e| e.status == "success" && e.action == "add")
                .count();
            if success_count > 0 {
                tracing::info!("Applied auto-membership policies for admin-created user {}: {} memberships added", user.id, success_count);
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to apply auto-membership policies for admin-created user {}: {}",
                user.id,
                e
            );
        }
    }

    Ok(Json(user))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateUserInput {
    pub role: Option<String>,
    pub display_name: Option<String>,
}

pub async fn update_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUserInput>,
) -> ApiResult<Json<crate::models::User>> {
    require_admin(&auth)?;

    let user = AdminRepository::new(&state.db)
        .update_user(id, input.role.as_deref(), input.display_name.as_deref())
        .await?;

    Ok(Json(user))
}

pub async fn deactivate_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    AdminRepository::new(&state.db).deactivate_user(id).await?;

    Ok(Json(serde_json::json!({"status": "deactivated"})))
}

pub async fn reactivate_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    AdminRepository::new(&state.db).reactivate_user(id).await?;

    Ok(Json(serde_json::json!({"status": "reactivated"})))
}

#[derive(Debug, serde::Deserialize)]
pub struct DeleteAdminUserInput {
    pub confirm: String,
    pub reason: Option<String>,
}

pub async fn delete_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<DeleteAdminUserInput>,
) -> ApiResult<Json<serde_json::Value>> {
    require_global_admin(&auth)?;

    if auth.user_id == id {
        return Err(AppError::Conflict(
            "You cannot delete your own account while logged in".to_string(),
        ));
    }

    let target = AdminRepository::new(&state.db)
        .get_user_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if target.deleted_at.is_some() {
        return Err(AppError::Conflict("User is already deleted".to_string()));
    }

    let confirm = input.confirm.trim();
    if confirm != target.username && confirm != target.email {
        return Err(AppError::BadRequest(
            "Confirmation text must exactly match the user's username or email".to_string(),
        ));
    }

    if target.role == "system_admin" {
        let admin_count = AdminRepository::new(&state.db)
            .count_system_admins()
            .await?;

        if admin_count <= 1 {
            return Err(AppError::Conflict(
                "Cannot delete the last remaining global admin".to_string(),
            ));
        }
    }

    let reason = input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    let deleted_user = AdminRepository::new(&state.db)
        .soft_delete_user(id, auth.user_id, reason.as_deref())
        .await?;

    insert_admin_audit_log(
        &state.db,
        auth.user_id,
        "user.soft_delete",
        "user",
        Some(id),
        serde_json::json!({
            "username": target.username,
            "email": target.email,
            "reason": reason,
        }),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "user_id": deleted_user.id,
        "deleted_at": deleted_user.deleted_at,
    })))
}

/// Permanently wipe a soft-deleted user from the database.
/// Only allowed if the user has no posts/messages.
pub async fn wipe_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_global_admin(&auth)?;

    if auth.user_id == id {
        return Err(AppError::Conflict(
            "You cannot wipe your own account".to_string(),
        ));
    }

    // Get the user and verify they are soft-deleted
    let target = AdminRepository::new(&state.db)
        .get_user_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if target.deleted_at.is_none() {
        return Err(AppError::Conflict(
            "User must be soft-deleted before wiping. Use DELETE endpoint first.".to_string(),
        ));
    }

    // Check if user has any posts
    let post_count = AdminRepository::new(&state.db)
        .count_posts_by_user(id)
        .await?;

    if post_count > 0 {
        return Err(AppError::Conflict(format!(
            "Cannot wipe user with {} post(s). User has messages in channels.",
            post_count
        )));
    }

    AdminRepository::new(&state.db).wipe_user(id).await?;

    // Log the wipe action
    insert_admin_audit_log(
        &state.db,
        auth.user_id,
        "user.wipe",
        "user",
        Some(id),
        serde_json::json!({
            "username": target.username,
            "email": target.email,
            "deleted_at": target.deleted_at,
        }),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "status": "wiped",
        "user_id": id,
        "message": "User permanently deleted from database",
    })))
}
