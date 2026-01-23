//! Users API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::auth::{AuthUser, hash_password};
use crate::error::{ApiResult, AppError};
use crate::models::{UpdateUser, User, UserResponse, ChangePassword};

/// Build users routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/{id}", get(get_user).put(update_user))
        .route("/{id}/password", axum::routing::post(change_password))
}

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub _org_id: Option<Uuid>,
}

/// List users (requires admin or same org)
async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListUsersQuery>,
) -> ApiResult<Json<Vec<UserResponse>>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let users: Vec<User> = if auth.role == "system_admin" {
        // System admin can see all users
        sqlx::query_as(
            "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else if let Some(org_id) = auth.org_id {
        // Regular users see their org members
        sqlx::query_as(
            "SELECT * FROM users WHERE org_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(org_id)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        return Err(AppError::Forbidden("No organization access".to_string()));
    };

    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

/// Get a specific user
async fn get_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UserResponse>> {
    let user: User = sqlx::query_as(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Check access: same user, same org, or system admin
    let can_view = auth.user_id == user.id
        || auth.role == "system_admin"
        || (auth.org_id.is_some() && auth.org_id == user.org_id);

    if !can_view {
        return Err(AppError::Forbidden("Cannot view this user".to_string()));
    }

    Ok(Json(UserResponse::from(user)))
}

/// Update a user profile
async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUser>,
) -> ApiResult<Json<UserResponse>> {
    // Only the user themselves or an admin can update
    if auth.user_id != id && auth.role != "system_admin" && auth.role != "org_admin" {
        return Err(AppError::Forbidden("Cannot update this user".to_string()));
    }

    // Build dynamic update query
    let mut updates = Vec::new();
    let mut param_count = 1;

    if input.username.is_some() {
        updates.push(format!("username = ${}", param_count));
        param_count += 1;
    }
    if input.display_name.is_some() {
        updates.push(format!("display_name = ${}", param_count));
        param_count += 1;
    }
    if input.avatar_url.is_some() {
        updates.push(format!("avatar_url = ${}", param_count));
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    // For simplicity, we'll do separate optional updates
    if let Some(ref username) = input.username {
        sqlx::query("UPDATE users SET username = $1 WHERE id = $2")
            .bind(username)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref display_name) = input.display_name {
        sqlx::query("UPDATE users SET display_name = $1 WHERE id = $2")
            .bind(display_name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref avatar_url) = input.avatar_url {
        sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
            .bind(avatar_url)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    // Fetch updated user
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    // Broadcast update
    let user_response = UserResponse::from(user.clone());
    let event = crate::realtime::events::WsEnvelope::event(
        crate::realtime::events::EventType::UserUpdated,
        user_response.clone(),
        None,
    ).with_broadcast(crate::realtime::events::WsBroadcast {
        team_id: None,
        channel_id: None,
        user_id: None, // Broadcast to everyone
        exclude_user_id: Some(auth.user_id),
    });
    state.ws_hub.broadcast(event).await;

    Ok(Json(user_response))
}

/// Change user password
async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ChangePassword>,
) -> ApiResult<Json<serde_json::Value>> {
    // Only the user themselves can change their password
    if auth.user_id != id {
        return Err(AppError::Forbidden("Cannot change password for another user".to_string()));
    }

    // Fetch user with current password hash (still needed to ensure user exists and for other potential logic)
    let _user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    // Validate new password complexity
    let config = crate::services::auth_config::get_password_rules(&state.db).await?;
    crate::services::auth_config::validate_password(&input.new_password, &config)?;

    // Hash and update
    let new_hash = hash_password(&input.new_password)?;
    
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_hash)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "status": "success", "message": "Password updated successfully" })))
}



