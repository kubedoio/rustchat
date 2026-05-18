use axum::{body::Bytes, extract::State, http::HeaderMap, Json};
use serde::Deserialize;

use super::parse_body;
use super::status;
use super::user_sidebar_categories::resolve_user_id;
use super::utils::parse_timezone_for_update;
use super::MmAuthUser;
use crate::api::AppState;
use crate::auth::extractors::PolymorphicAuth;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use crate::models::User;
use crate::repositories::UserRepository;

/// GET /users/me - Get authenticated user (supports both JWT and API key auth)
///
/// This endpoint supports polymorphic authentication:
/// - JWT token (for human users via browser/mobile)
/// - API key (for agents, services, and CI systems)
pub async fn me(State(state): State<AppState>, auth: PolymorphicAuth) -> ApiResult<Json<mm::User>> {
    let mut user: User = UserRepository::new(&state.db)
        .get_by_id(auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let _ = status::clear_expired_custom_status_if_needed(&state, auth.user_id).await?;
    user.clear_custom_status_if_expired();

    Ok(Json(user.into()))
}

/// GET /users/{user_id} - Get user by ID
pub async fn get_user_by_id(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<mm::User>> {
    // Handle "me" as a special case
    let user_uuid = if user_id == "me" {
        return Err(AppError::BadRequest("Use /users/me endpoint".to_string()));
    } else {
        parse_mm_or_uuid(&user_id)
            .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?
    };

    let mut user: User = UserRepository::new(&state.db)
        .get_by_id_unchecked(user_uuid)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let _ = status::clear_expired_custom_status_if_needed(&state, user_uuid).await?;
    user.clear_custom_status_if_expired();

    Ok(Json(user.into()))
}

/// GET /users/username/{username} - Get user by username
pub async fn get_user_by_username(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> ApiResult<Json<mm::User>> {
    let user: User = UserRepository::new(&state.db)
        .get_by_username(&username)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

#[derive(Deserialize)]
pub struct PatchMeRequest {
    pub nickname: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub position: Option<String>,
    #[serde(default)]
    pub timezone: Option<serde_json::Value>,
    #[serde(default)]
    pub notify_props: Option<serde_json::Value>,
}

pub async fn patch_me(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::User>> {
    let input: PatchMeRequest = parse_body(&headers, &body, "Invalid patch body")?;
    let timezone = parse_timezone_for_update(input.timezone.as_ref());

    // Update any provided fields
    sqlx::query(
        r#"
        UPDATE users 
        SET first_name = COALESCE($1, first_name),
            last_name = COALESCE($2, last_name),
            nickname = COALESCE($3, nickname),
            position = COALESCE($4, position),
            notify_props = COALESCE($5, notify_props),
            timezone = COALESCE($6, timezone),
            updated_at = NOW()
        WHERE id = $7
        "#,
    )
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(&input.nickname)
    .bind(&input.position)
    .bind(input.notify_props.as_ref())
    .bind(timezone.as_deref())
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    // Fetch updated user
    let user: User = UserRepository::new(&state.db)
        .get_by_id_unchecked(auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

pub async fn patch_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::User>> {
    let input: PatchMeRequest = parse_body(&headers, &body, "Invalid patch body")?;
    let user_id = resolve_user_id(&user_id, &auth)?;
    let timezone = parse_timezone_for_update(input.timezone.as_ref());

    // Update user profile fields
    sqlx::query(
        r#"UPDATE users SET 
            nickname = COALESCE($1, nickname),
            first_name = COALESCE($2, first_name),
            last_name = COALESCE($3, last_name),
            position = COALESCE($4, position),
            notify_props = COALESCE($5, notify_props),
            timezone = COALESCE($6, timezone),
            updated_at = NOW()
        WHERE id = $7"#,
    )
    .bind(input.nickname)
    .bind(input.first_name)
    .bind(input.last_name)
    .bind(input.position)
    .bind(input.notify_props)
    .bind(timezone.as_deref())
    .bind(user_id)
    .execute(&state.db)
    .await?;

    let user: User = UserRepository::new(&state.db)
        .get_by_id_unchecked(user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    Ok(Json(user.into()))
}
