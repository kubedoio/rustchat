use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::{create_token, verify_password};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::models::{channel::Channel, channel::ChannelMember, Team, TeamMember, User};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/login", post(login))
        .route("/users/me", get(me))
        .route("/users/me/teams", get(my_teams))
        .route("/users/me/teams/members", get(my_team_members))
        .route("/users/me/teams/{team_id}/channels", get(my_team_channels))
        .route("/users/me/channels", get(my_channels))
        .route(
            "/users/me/teams/{team_id}/channels/not_members",
            get(my_team_channels_not_members),
        )
        .route("/users", get(list_users))
        .route("/users/{user_id}", get(get_user_by_id))
        .route("/users/username/{username}", get(get_user_by_username))
        .route(
            "/users/me/teams/{team_id}/channels/members",
            get(my_team_channel_members),
        )
        .route("/users/me/teams/unread", get(my_teams_unread))
        .route(
            "/users/sessions/device",
            post(attach_device).put(attach_device).delete(detach_device),
        )
        .route(
            "/users/me/preferences",
            get(get_preferences).put(update_preferences),
        )
        .route(
            "/users/{user_id}/preferences",
            put(update_preferences_for_user),
        )
        .route("/users/status/ids", post(get_statuses_by_ids))
        .route("/users/ids", post(get_users_by_ids))
        .route("/users/{user_id}/status", get(get_status).put(update_status))
        .route("/users/me/status", get(get_my_status).put(update_status))
        .route("/users/{user_id}/channels/{channel_id}/typing", post(user_typing))
        .route("/users/me/patch", put(patch_me))
        .route("/users/{user_id}/image", get(get_user_image).post(upload_user_image))
        .route("/roles/names", post(get_roles_by_names))
        .route("/users/notifications", get(get_notifications).put(update_notifications))
        .route("/users/me/sessions", get(get_sessions))
        .route("/users/logout", get(logout).post(logout))
        .route("/users/autocomplete", get(autocomplete_users))
        .route("/users/search", post(search_users))
        .route("/custom_profile_attributes/fields", get(get_custom_profile_attributes))
        .route("/users/{user_id}/custom_profile_attributes", get(get_user_custom_profile_attributes))
        .route(
            "/users/{user_id}/sidebar/categories",
            get(get_categories).post(create_category).put(update_categories),
        )
        .route(
            "/users/{user_id}/sidebar/categories/order",
            put(update_category_order),
        )
}

#[derive(Deserialize)]
struct CategoriesPath {
    user_id: String,
}

async fn get_categories(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<mm::SidebarCategories>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id_str = query.get("team_id").ok_or_else(|| AppError::BadRequest("Missing team_id".to_string()))?;
    let team_id = parse_mm_or_uuid(team_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    get_categories_internal(state, user_id, team_id).await
}

async fn create_category(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    Json(input): Json<CreateCategoryRequest>,
) -> ApiResult<Json<mm::SidebarCategory>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id_str = query.get("team_id").ok_or_else(|| AppError::BadRequest("Missing team_id".to_string()))?;
    let team_id = parse_mm_or_uuid(team_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    create_category_internal(state, user_id, team_id, input).await
}

async fn update_categories(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    Json(input): Json<UpdateCategoriesRequest>,
) -> ApiResult<Json<Vec<mm::SidebarCategory>>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id_str = query.get("team_id").ok_or_else(|| AppError::BadRequest("Missing team_id".to_string()))?;
    let team_id = parse_mm_or_uuid(team_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    update_categories_internal(state, user_id, team_id, input).await
}

async fn update_category_order(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    Json(order): Json<Vec<String>>,
) -> ApiResult<Json<Vec<String>>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id_str = query.get("team_id").ok_or_else(|| AppError::BadRequest("Missing team_id".to_string()))?;
    let team_id = parse_mm_or_uuid(team_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    update_category_order_internal(state, user_id, team_id, order).await
}

pub(crate) fn resolve_user_id(user_id_str: &str, auth: &MmAuthUser) -> ApiResult<Uuid> {
    if user_id_str == "me" {
        return Ok(auth.user_id);
    }

    let user_id = parse_mm_or_uuid(user_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;

    if user_id != auth.user_id && auth.role != "system_admin" && auth.role != "org_admin" {
        return Err(AppError::Forbidden("Cannot access another user's categories".to_string()));
    }

    Ok(user_id)
}

pub(crate) async fn get_categories_internal(
    state: AppState,
    user_id: Uuid,
    team_id: Uuid,
) -> ApiResult<Json<mm::SidebarCategories>> {
    ensure_team_exists(&state, team_id).await?;
    ensure_team_member(&state, user_id, team_id).await?;

    // Fetch categories
    let categories_rows: Vec<CategoryRow> = sqlx::query_as(
        "SELECT * FROM channel_categories WHERE user_id = $1 AND team_id = $2 AND delete_at = 0"
    )
    .bind(user_id)
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    if categories_rows.is_empty() {
        return Ok(Json(get_default_categories(&state, user_id, team_id).await?));
    }

    let mut categories = Vec::new();
    let mut order = Vec::new();
    let mut sorted_rows = categories_rows;
    sort_category_rows(&mut sorted_rows);

    for row in sorted_rows {
        let channel_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT channel_id FROM channel_category_channels WHERE category_id = $1 ORDER BY sort_order ASC"
        )
        .bind(row.id)
        .fetch_all(&state.db)
        .await?;

        let channel_ids = channel_ids.into_iter().map(encode_mm_id).collect();

        order.push(encode_mm_id(row.id));
        categories.push(mm::SidebarCategory {
            id: encode_mm_id(row.id),
            team_id: encode_mm_id(row.team_id),
            user_id: encode_mm_id(row.user_id),
            category_type: row.type_field,
            display_name: row.display_name,
            sorting: row.sorting,
            muted: row.muted,
            collapsed: row.collapsed,
            channel_ids,
            create_at: row.create_at,
            update_at: row.update_at,
            delete_at: row.delete_at,
        });
    }

    Ok(Json(mm::SidebarCategories { categories, order }))
}

#[derive(sqlx::FromRow, Clone)]
struct CategoryRow {
    id: Uuid,
    team_id: Uuid,
    user_id: Uuid,
    #[sqlx(rename = "type")]
    type_field: String,
    display_name: String,
    sorting: String,
    muted: bool,
    collapsed: bool,
    #[allow(dead_code)]
    sort_order: i32,
    create_at: i64,
    update_at: i64,
    delete_at: i64,
}

fn sort_category_rows(rows: &mut [CategoryRow]) {
    let has_custom_order = rows.iter().any(|row| row.sort_order != 0);

    if has_custom_order {
        rows.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.display_name.to_ascii_lowercase().cmp(&b.display_name.to_ascii_lowercase()))
        });
    } else {
        rows.sort_by(|a, b| a.display_name.to_ascii_lowercase().cmp(&b.display_name.to_ascii_lowercase()));
    }
}

async fn ensure_team_exists(state: &AppState, team_id: Uuid) -> ApiResult<()> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = $1)")
        .bind(team_id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Team not found".to_string()));
    }

    Ok(())
}

async fn ensure_team_member(state: &AppState, user_id: Uuid, team_id: Uuid) -> ApiResult<()> {
    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE user_id = $1 AND team_id = $2)",
    )
    .bind(user_id)
    .bind(team_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(AppError::Forbidden("User is not a member of the team".to_string()));
    }

    Ok(())
}

fn build_default_categories(
    user_id: Uuid,
    team_id: Uuid,
    channel_ids: Vec<String>,
    now: i64,
) -> mm::SidebarCategories {
    let category = mm::SidebarCategory {
        id: encode_mm_id(Uuid::new_v4()),
        team_id: encode_mm_id(team_id),
        user_id: encode_mm_id(user_id),
        category_type: "custom".to_string(),
        display_name: "Channels".to_string(),
        sorting: "alpha".to_string(),
        muted: false,
        collapsed: false,
        channel_ids,
        create_at: now,
        update_at: now,
        delete_at: 0,
    };

    mm::SidebarCategories {
        order: vec![category.id.clone()],
        categories: vec![category],
    }
}

async fn get_default_categories(state: &AppState, user_id: Uuid, team_id: Uuid) -> ApiResult<mm::SidebarCategories> {
    let channels: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT c.id FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE cm.user_id = $1 AND c.team_id = $2
        ORDER BY COALESCE(c.display_name, c.name) ASC
        "#
    )
    .bind(user_id)
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    let now = Utc::now().timestamp_millis();
    let channel_ids = channels.into_iter().map(encode_mm_id).collect();
    Ok(build_default_categories(user_id, team_id, channel_ids, now))
}

#[derive(Deserialize)]
pub(crate) struct CreateCategoryRequest {
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
    display_name: String,
    #[serde(rename = "type")]
    category_type: Option<String>,
    #[serde(default)]
    sorting: Option<String>,
}

pub(crate) async fn create_category_internal(
    state: AppState,
    user_id: Uuid,
    team_id: Uuid,
    input: CreateCategoryRequest,
) -> ApiResult<Json<mm::SidebarCategory>> {
    ensure_team_exists(&state, team_id).await?;
    ensure_team_member(&state, user_id, team_id).await?;

    if let Some(input_user_id) = input.user_id.as_deref() {
        let parsed = parse_mm_or_uuid(input_user_id)
            .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?;
        if parsed != user_id {
            return Err(AppError::BadRequest("user_id does not match path".to_string()));
        }
    }

    if let Some(input_team_id) = input.team_id.as_deref() {
        let parsed = parse_mm_or_uuid(input_team_id)
            .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
        if parsed != team_id {
            return Err(AppError::BadRequest("team_id does not match path".to_string()));
        }
    }

    let now = Utc::now().timestamp_millis();
    let id = Uuid::new_v4();
    let category_type = input.category_type.unwrap_or_else(|| "custom".to_string());
    let sorting = input.sorting.unwrap_or_else(|| "alpha".to_string());

    let next_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM channel_categories WHERE user_id = $1 AND team_id = $2",
    )
    .bind(user_id)
    .bind(team_id)
    .fetch_one(&state.db)
    .await?;

    sqlx::query(
        "INSERT INTO channel_categories (id, team_id, user_id, type, display_name, sorting, sort_order, create_at, update_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(id)
    .bind(team_id)
    .bind(user_id)
    .bind(&category_type)
    .bind(&input.display_name)
    .bind(&sorting)
    .bind(next_order as i32)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;

    Ok(Json(mm::SidebarCategory {
        id: encode_mm_id(id),
        team_id: encode_mm_id(team_id),
        user_id: encode_mm_id(user_id),
        category_type,
        display_name: input.display_name,
        sorting,
        muted: false,
        collapsed: false,
        channel_ids: vec![],
        create_at: now,
        update_at: now,
        delete_at: 0,
    }))
}

#[derive(Deserialize)]
pub(crate) struct UpdateCategoriesRequest {
    categories: Vec<mm::SidebarCategory>,
}

pub(crate) async fn update_categories_internal(
    state: AppState,
    user_id: Uuid,
    team_id: Uuid,
    input: UpdateCategoriesRequest,
) -> ApiResult<Json<Vec<mm::SidebarCategory>>> {
    ensure_team_exists(&state, team_id).await?;
    ensure_team_member(&state, user_id, team_id).await?;

    let now = Utc::now().timestamp_millis();
    let mut updated_categories = Vec::new();

    let mut tx = state.db.begin().await?;

    for cat in input.categories {
        let cat_uuid = parse_mm_or_uuid(&cat.id)
            .ok_or_else(|| AppError::BadRequest("Invalid category ID".to_string()))?;

        let cat_user_id = parse_mm_or_uuid(&cat.user_id)
            .ok_or_else(|| AppError::BadRequest("Invalid category user_id".to_string()))?;
        if cat_user_id != user_id {
            return Err(AppError::BadRequest("category user_id does not match path".to_string()));
        }

        let cat_team_id = parse_mm_or_uuid(&cat.team_id)
            .ok_or_else(|| AppError::BadRequest("Invalid category team_id".to_string()))?;
        if cat_team_id != team_id {
            return Err(AppError::BadRequest("category team_id does not match path".to_string()));
        }

        sqlx::query(
            "UPDATE channel_categories SET display_name = $1, sorting = $2, muted = $3, collapsed = $4, update_at = $5 WHERE id = $6 AND user_id = $7 AND team_id = $8"
        )
        .bind(&cat.display_name)
        .bind(&cat.sorting)
        .bind(cat.muted)
        .bind(cat.collapsed)
        .bind(now)
        .bind(cat_uuid)
        .bind(user_id)
        .bind(team_id)
        .execute(&mut *tx)
        .await?;

        // Update channels
        sqlx::query("DELETE FROM channel_category_channels WHERE category_id = $1")
            .bind(cat_uuid)
            .execute(&mut *tx)
            .await?;

        let mut parsed_channel_ids = Vec::new();
        for (i, channel_id_str) in cat.channel_ids.iter().enumerate() {
            let channel_uuid = parse_mm_or_uuid(channel_id_str)
                .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;
            sqlx::query("INSERT INTO channel_category_channels (category_id, channel_id, sort_order) VALUES ($1, $2, $3)")
                .bind(cat_uuid)
                .bind(channel_uuid)
                .bind(i as i32)
                .execute(&mut *tx)
                .await?;
            parsed_channel_ids.push(channel_uuid);
        }

        let mut cat_out = cat;
        cat_out.id = encode_mm_id(cat_uuid);
        cat_out.user_id = encode_mm_id(user_id);
        cat_out.team_id = encode_mm_id(team_id);
        cat_out.channel_ids = parsed_channel_ids.into_iter().map(encode_mm_id).collect();
        updated_categories.push(cat_out);
    }

    tx.commit().await?;

    Ok(Json(updated_categories))
}

pub(crate) async fn update_category_order_internal(
    state: AppState,
    user_id: Uuid,
    team_id: Uuid,
    order: Vec<String>,
) -> ApiResult<Json<Vec<String>>> {
    ensure_team_exists(&state, team_id).await?;
    ensure_team_member(&state, user_id, team_id).await?;

    let mut tx = state.db.begin().await?;

    for (i, cat_id_str) in order.iter().enumerate() {
        let cat_uuid = parse_mm_or_uuid(cat_id_str)
            .ok_or_else(|| AppError::BadRequest("Invalid category ID".to_string()))?;
        sqlx::query(
            "UPDATE channel_categories SET sort_order = $1 WHERE id = $2 AND user_id = $3 AND team_id = $4"
        )
        .bind(i as i32)
        .bind(cat_uuid)
        .bind(user_id)
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(order))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_millis_timestamp(value: i64) -> bool {
        value >= 1_000_000_000_000 && value <= 9_999_999_999_999
    }

    fn row(display_name: &str, sort_order: i32) -> CategoryRow {
        CategoryRow {
            id: Uuid::new_v4(),
            team_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            type_field: "custom".to_string(),
            display_name: display_name.to_string(),
            sorting: "alpha".to_string(),
            muted: false,
            collapsed: false,
            sort_order,
            create_at: 0,
            update_at: 0,
            delete_at: 0,
        }
    }

    #[test]
    fn default_category_generation() {
        let user_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();
        let channel_ids = vec!["chan-a".to_string(), "chan-b".to_string()];
        let now = 1_700_000_000_123i64;

        let result = build_default_categories(user_id, team_id, channel_ids.clone(), now);
        assert_eq!(result.categories.len(), 1);
        assert_eq!(result.order.len(), 1);

        let category = &result.categories[0];
        assert_eq!(category.display_name, "Channels");
        assert_eq!(category.channel_ids, channel_ids);
        assert_eq!(category.create_at, now);
        assert_eq!(category.update_at, now);
        assert_eq!(result.order[0], category.id);
    }

    #[test]
    fn timestamps_are_millis() {
        let user_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();
        let now = 1_700_000_000_000i64;

        let result = build_default_categories(user_id, team_id, Vec::new(), now);
        let category = &result.categories[0];
        assert!(is_millis_timestamp(category.create_at));
        assert!(is_millis_timestamp(category.update_at));
    }

    #[test]
    fn ordering_logic_prefers_sort_order() {
        let mut rows = vec![row("Gamma", 2), row("Alpha", 1)];
        sort_category_rows(&mut rows);
        assert_eq!(rows[0].display_name, "Alpha");
        assert_eq!(rows[1].display_name, "Gamma");
    }

    #[test]
    fn ordering_logic_falls_back_to_display_name() {
        let mut rows = vec![row("Bravo", 0), row("alpha", 0), row("Charlie", 0)];
        sort_category_rows(&mut rows);
        assert_eq!(rows[0].display_name, "alpha");
        assert_eq!(rows[1].display_name, "Bravo");
        assert_eq!(rows[2].display_name, "Charlie");
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    login_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    password: String,
    #[allow(dead_code)]
    device_id: Option<String>,
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<impl IntoResponse> {
    let input = parse_login_request(&headers, &body)?;
    let login_id = input
        .login_id
        .or(input.email)
        .ok_or_else(|| AppError::BadRequest("Missing login_id".to_string()))?;

    let user: Option<User> = sqlx::query_as(
        "SELECT * FROM users WHERE (email = $1 OR username = $1) AND is_active = true",
    )
    .bind(&login_id)
    .fetch_optional(&state.db)
    .await?;

    let user =
        user.ok_or_else(|| AppError::Unauthorized("Invalid login credentials".to_string()))?;

    if !verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized(
            "Invalid login credentials".to_string(),
        ));
    }

    // Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    // Generate token
    let token = create_token(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_expiry_hours,
    )?;

    let mm_user: mm::User = user.into();

    let mut headers = HeaderMap::new();
    headers.insert("Token", HeaderValue::from_str(&token).unwrap());

    Ok((headers, Json(mm_user)))
}

fn parse_login_request(headers: &HeaderMap, body: &Bytes) -> ApiResult<LoginRequest> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body)
            .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes(body)
            .map_err(|_| AppError::BadRequest("Invalid form body".to_string()))
    } else {
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest("Unsupported login body".to_string()))
    }
}

async fn me(State(state): State<AppState>, auth: MmAuthUser) -> ApiResult<Json<mm::User>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(user.into()))
}

/// GET /users/{user_id} - Get user by ID
async fn get_user_by_id(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<mm::User>> {
    // Handle "me" as a special case
    let user_uuid = if user_id == "me" {
        return Err(AppError::BadRequest("Use /users/me endpoint".to_string()));
    } else {
        parse_mm_or_uuid(&user_id)
            .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?
    };

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

/// GET /users/username/{username} - Get user by username
async fn get_user_by_username(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(username): Path<String>,
) -> ApiResult<Json<mm::User>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}


async fn my_teams(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Team>>> {
    let teams: Vec<Team> = sqlx::query_as(
        r#"
        SELECT t.* FROM teams t
        JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    if teams.is_empty() {
        return Ok(Json(vec![default_team()]));
    }

    let mm_teams: Vec<mm::Team> = teams.into_iter().map(|t| t.into()).collect();
    Ok(Json(mm_teams))
}

fn default_team() -> mm::Team {
    let id = Uuid::new_v4();
    mm::Team {
        id: encode_mm_id(id),
        create_at: 0,
        update_at: 0,
        delete_at: 0,
        display_name: "RustChat".to_string(),
        name: "rustchat".to_string(),
        description: "".to_string(),
        email: "".to_string(),
        team_type: "O".to_string(),
        company_name: "".to_string(),
        allowed_domains: "".to_string(),
        invite_id: "".to_string(),
        allow_open_invite: false,
    }
}

async fn my_team_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::TeamMember>>> {
    let members: Vec<TeamMember> = sqlx::query_as("SELECT * FROM team_members WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await?;

    let mm_members = members
        .into_iter()
        .map(|m| mm::TeamMember {
            team_id: encode_mm_id(m.team_id),
            user_id: encode_mm_id(m.user_id),
            roles: "team_user".to_string(),
            delete_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: false,
        })
        .collect();

    Ok(Json(mm_members))
}

async fn my_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.* FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

async fn my_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.* FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE cm.user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

async fn my_team_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let members: Vec<ChannelMember> = sqlx::query_as(
        r#"
        SELECT cm.*, c.name as username, c.display_name, NULL as avatar_url, NULL as presence
        FROM channel_members cm
        JOIN channels c ON cm.channel_id = c.id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_members = members
        .into_iter()
        .map(|m| mm::ChannelMember {
            channel_id: encode_mm_id(m.channel_id),
            user_id: encode_mm_id(m.user_id),
            roles: "channel_user".to_string(),
            last_viewed_at: m.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            msg_count: 0,
            mention_count: 0,
            notify_props: normalize_notify_props(m.notify_props),
            last_update_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: false,
        })
        .collect();

    Ok(Json(mm_members))
}

#[derive(Deserialize)]
struct NotMembersQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn my_team_channels_not_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Query(query): Query<NotMembersQuery>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;

    let page = query.page.unwrap_or(0).max(0);
    let per_page = query.per_page.unwrap_or(60).clamp(1, 200);
    let offset = page * per_page;

    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.*
        FROM channels c
        WHERE c.team_id = $1
          AND c.is_archived = false
          AND c.type IN ('public', 'private')
          AND NOT EXISTS (
              SELECT 1 FROM channel_members cm
              WHERE cm.channel_id = c.id AND cm.user_id = $2
          )
        ORDER BY COALESCE(c.display_name, c.name) ASC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

async fn my_teams_unread(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

fn normalize_notify_props(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        return serde_json::json!({"desktop": "default", "mark_unread": "all"});
    }

    if let Some(obj) = value.as_object() {
        if obj.is_empty() {
            return serde_json::json!({"desktop": "default", "mark_unread": "all"});
        }
    }

    value
}

#[derive(Deserialize)]
struct AttachDeviceRequest {
    device_id: String,
    #[allow(dead_code)]
    token: String,
    #[allow(dead_code)]
    platform: Option<String>,
}

async fn attach_device(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<impl IntoResponse> {
    let input: AttachDeviceRequest = parse_body(&headers, &body, "Invalid device body")?;
    sqlx::query(
        r#"
        INSERT INTO user_devices (user_id, device_id, token, platform)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, device_id)
        DO UPDATE SET token = $3, platform = $4, last_seen_at = NOW()
        "#,
    )
    .bind(auth.user_id)
    .bind(input.device_id)
    .bind(input.token)
    .bind(input.platform.unwrap_or_else(|| "unknown".to_string()))
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

#[derive(Deserialize)]
struct DetachDeviceRequest {
    device_id: String,
}

async fn detach_device(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<DetachDeviceRequest>,
) -> ApiResult<impl IntoResponse> {
    sqlx::query("DELETE FROM user_devices WHERE user_id = $1 AND device_id = $2")
        .bind(auth.user_id)
        .bind(input.device_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_preferences(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Preference>>> {
    let rows = sqlx::query("SELECT user_id, category, name, value FROM mattermost_preferences WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut prefs = Vec::new();
    for row in rows {
        use sqlx::Row;
        let uid: Uuid = row.try_get("user_id").unwrap_or_default();
        prefs.push(mm::Preference {
            user_id: encode_mm_id(uid),
            category: row.try_get("category").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            value: row.try_get("value").unwrap_or_default(),
        });
    }

    Ok(Json(prefs))
}

async fn update_preferences(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(prefs): Json<Vec<mm::Preference>>,
) -> ApiResult<impl IntoResponse> {
    update_preferences_internal(&state, auth.user_id, prefs).await
}

async fn update_preferences_for_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    Json(prefs): Json<Vec<mm::Preference>>,
) -> ApiResult<impl IntoResponse> {
    let user_id = resolve_user_id(&user_id, &auth)?;
    update_preferences_internal(&state, user_id, prefs).await
}

async fn update_preferences_internal(
    state: &AppState,
    user_id: Uuid,
    prefs: Vec<mm::Preference>,
) -> ApiResult<impl IntoResponse> {
    let mut tx = state.db.begin().await?;

    for p in prefs {
        sqlx::query(
            r#"
            INSERT INTO mattermost_preferences (user_id, category, name, value)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, category, name)
            DO UPDATE SET value = $4
            "#,
        )
        .bind(user_id)
        .bind(p.category)
        .bind(p.name)
        .bind(p.value)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_notifications() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "email": "true",
        "push": "mention",
        "desktop": "all",
        "desktop_sound": "Bing",
        "mention_keys": "",
        "channel": "true",
        "first_name": "false",
        "push_status": "online",
        "comments": "never",
        "milestones": "none",
        "auto_responder_active": "false",
        "auto_responder_message": ""
    })))
}

async fn update_notifications(
    Json(_input): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_sessions() -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

async fn logout() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

#[derive(Deserialize)]
struct AutocompleteQuery {
    in_team: Option<String>,
    in_channel: Option<String>,
    name: Option<String>,
    limit: Option<i64>,
}

async fn autocomplete_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Query(query): Query<AutocompleteQuery>,
) -> ApiResult<Json<Vec<mm::User>>> {
    let limit = query.limit.unwrap_or(25).clamp(1, 200) as i64;
    let name = query.name.unwrap_or_default();
    let name_like = format!("%{}%", name);

    let mut users: Vec<User> = if let Some(channel_id) = query.in_channel {
        let channel_id = parse_mm_or_uuid(&channel_id)
            .ok_or_else(|| AppError::BadRequest("Invalid in_channel".to_string()))?;

        let is_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)",
        )
        .bind(channel_id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if !is_member {
            return Err(AppError::Forbidden("Not a member of this channel".to_string()));
        }

        sqlx::query_as(
            r#"
            SELECT u.*
            FROM users u
            JOIN channel_members cm ON u.id = cm.user_id
            WHERE cm.channel_id = $1
              AND (u.username ILIKE $2 OR u.email ILIKE $2)
              AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(&name_like)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else if let Some(team_id) = query.in_team {
        let team_id = parse_mm_or_uuid(&team_id)
            .ok_or_else(|| AppError::BadRequest("Invalid in_team".to_string()))?;

        let is_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
        )
        .bind(team_id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if !is_member {
            return Err(AppError::Forbidden("Not a member of this team".to_string()));
        }

        sqlx::query_as(
            r#"
            SELECT u.*
            FROM users u
            JOIN team_members tm ON u.id = tm.user_id
            WHERE tm.team_id = $1
              AND (u.username ILIKE $2 OR u.email ILIKE $2)
              AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3
            "#,
        )
        .bind(team_id)
        .bind(&name_like)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM users WHERE (username ILIKE $1 OR email ILIKE $1) AND is_active = true ORDER BY username ASC LIMIT $2",
        )
        .bind(&name_like)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    };

    users.truncate(limit as usize);
    let mm_users: Vec<mm::User> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(mm_users))
}

#[derive(Deserialize)]
struct UserSearchRequest {
    term: Option<String>,
    team_id: Option<String>,
    not_in_channel_id: Option<String>,
    in_channel_id: Option<String>,
    limit: Option<i64>,
}

async fn search_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::User>>> {
    let input: UserSearchRequest = parse_body(&headers, &body, "Invalid search body")?;
    let term = input.term.unwrap_or_default();
    let like = format!("%{}%", term);
    let limit = input.limit.unwrap_or(100).clamp(1, 200) as i64;

    let users: Vec<User> = if let Some(channel_id) = input.in_channel_id {
        let channel_id = parse_mm_or_uuid(&channel_id)
            .ok_or_else(|| AppError::BadRequest("Invalid in_channel_id".to_string()))?;

        let is_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)",
        )
        .bind(channel_id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if !is_member {
            return Err(AppError::Forbidden("Not a member of this channel".to_string()));
        }

        sqlx::query_as(
            r#"
            SELECT u.*
            FROM users u
            JOIN channel_members cm ON u.id = cm.user_id
            WHERE cm.channel_id = $1
              AND (u.username ILIKE $2 OR u.email ILIKE $2)
              AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(&like)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else if let Some(team_id) = input.team_id {
        let team_id = parse_mm_or_uuid(&team_id)
            .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;

        let is_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
        )
        .bind(team_id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if !is_member {
            return Err(AppError::Forbidden("Not a member of this team".to_string()));
        }

        sqlx::query_as(
            r#"
            SELECT u.*
            FROM users u
            JOIN team_members tm ON u.id = tm.user_id
            WHERE tm.team_id = $1
              AND (u.username ILIKE $2 OR u.email ILIKE $2)
              AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3
            "#,
        )
        .bind(team_id)
        .bind(&like)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM users WHERE (username ILIKE $1 OR email ILIKE $1) AND is_active = true ORDER BY username ASC LIMIT $2",
        )
        .bind(&like)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    };

    let mm_users: Vec<mm::User> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(mm_users))
}

async fn get_statuses_by_ids(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::Status>>> {
    let ids: Vec<String> = parse_body(&headers, &body, "Invalid status ids body")?;
    let uuids: Vec<Uuid> = ids.iter().filter_map(|id| parse_mm_or_uuid(id)).collect();

    if uuids.is_empty() {
        return Ok(Json(vec![]));
    }

    let users: Vec<(Uuid, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, presence, last_login_at FROM users WHERE id = ANY($1)",
    )
    .bind(&uuids)
    .fetch_all(&state.db)
    .await?;

    let statuses = users.into_iter().map(|(id, presence, last_login)| {
        mm::Status {
            user_id: encode_mm_id(id),
            status: if presence.is_empty() { "offline".to_string() } else { presence },
            manual: false,
            last_activity_at: last_login.map(|t| t.timestamp_millis()).unwrap_or(0),
        }
    }).collect();

    Ok(Json(statuses))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum UsersByIdsRequest {
    Ids(Vec<String>),
    Wrapped { user_ids: Vec<String> },
}

async fn get_users_by_ids(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::User>>> {
    let ids = parse_body::<UsersByIdsRequest>(&headers, &body, "Invalid users/ids body")
        .map(|parsed| match parsed {
            UsersByIdsRequest::Ids(ids) => ids,
            UsersByIdsRequest::Wrapped { user_ids } => user_ids,
        })?;

    let uuids: Vec<Uuid> = ids.iter().filter_map(|id| parse_mm_or_uuid(id)).collect();

    if uuids.is_empty() {
        return Ok(Json(vec![]));
    }

    let users: Vec<User> = sqlx::query_as("SELECT * FROM users WHERE id = ANY($1) AND is_active = true")
        .bind(&uuids)
        .fetch_all(&state.db)
        .await?;

    let mm_users: Vec<mm::User> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(mm_users))
}

fn parse_body<T: DeserializeOwned>(
    headers: &HeaderMap,
    body: &Bytes,
    message: &str,
) -> ApiResult<T> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes(body)
            .map_err(|_| AppError::BadRequest(message.to_string()))
    } else {
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest(message.to_string()))
    }
}

async fn get_status(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<mm::Status>> {
    let user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;
    let (presence, last_login): (String, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT presence, last_login_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(mm::Status {
        user_id: encode_mm_id(user_id),
        status: if presence.is_empty() { "offline".to_string() } else { presence },
        manual: false,
        last_activity_at: last_login.map(|t| t.timestamp_millis()).unwrap_or(0),
    }))
}

async fn get_my_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<mm::Status>> {
    let (presence, last_login): (String, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT presence, last_login_at FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(mm::Status {
        user_id: encode_mm_id(auth.user_id),
        status: if presence.is_empty() { "offline".to_string() } else { presence },
        manual: false,
        last_activity_at: last_login.map(|t| t.timestamp_millis()).unwrap_or(0),
    }))
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    user_id: String,
    status: String,
}

#[derive(Deserialize)]
struct PatchMeRequest {
    #[allow(dead_code)]
    nickname: Option<String>,
    #[allow(dead_code)]
    first_name: Option<String>,
    #[allow(dead_code)]
    last_name: Option<String>,
    #[allow(dead_code)]
    position: Option<String>,
}

async fn update_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Status>> {
    let input: UpdateStatusRequest = parse_body(&headers, &body, "Invalid status update request")?;
    
    let input_user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;
    if input_user_id != auth.user_id {
        return Err(AppError::Forbidden("Cannot update other user's status".to_string()));
    }

    sqlx::query("UPDATE users SET presence = $1 WHERE id = $2")
        .bind(&input.status)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let status = mm::Status {
        user_id: encode_mm_id(auth.user_id),
        status: input.status.clone(),
        manual: true,
        last_activity_at: Utc::now().timestamp_millis(),
    };

    // Broadcast status change
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UserUpdated, // Mapping to status_change in WS handler
        serde_json::json!({
             "user_id": auth.user_id,
             "status": input.status,
             "manual": true,
             "last_activity_at": status.last_activity_at
        }),
        None,
    );
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(status))
}

async fn patch_me(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::User>> {
    let _input: PatchMeRequest = parse_body(&headers, &body, "Invalid patch body")?;
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(user.into()))
}

async fn get_roles_by_names(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::Role>>> {
    let names: Vec<String> = parse_body(&headers, &body, "Invalid roles body")?;
    let permissions = fetch_role_permissions(&state, &names).await.unwrap_or_default();

    let roles = names
        .into_iter()
        .map(|name| {
            let mapped = permissions
                .get(&name)
                .cloned()
                .unwrap_or_else(|| vec!["create_post".to_string(), "read_channel".to_string()]);

            mm::Role {
                id: encode_mm_id(Uuid::new_v4()),
                display_name: name.clone(),
                description: "".to_string(),
                permissions: mapped,
                scheme_managed: false,
                name,
            }
        })
        .collect();

    Ok(Json(roles))
}

async fn fetch_role_permissions(
    state: &AppState,
    roles: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT role, permission_id FROM role_permissions WHERE role = ANY($1)",
    )
    .bind(roles)
    .fetch_all(&state.db)
    .await?;

    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (role, permission_id) in rows {
        map.entry(role)
            .or_default()
            .push(map_permission_id(&permission_id));
    }

    for permissions in map.values_mut() {
        if !permissions.iter().any(|p| p == "read_channel") {
            permissions.push("read_channel".to_string());
        }
        if !permissions.iter().any(|p| p == "read_post") {
            permissions.push("read_post".to_string());
        }
    }

    Ok(map)
}

fn map_permission_id(permission_id: &str) -> String {
    match permission_id {
        "post.create" => "create_post",
        "post.edit_own" => "edit_post",
        "post.delete_own" => "delete_post",
        "channel.create" => "create_channel",
        _ => permission_id,
    }
    .to_string()
}

#[derive(Deserialize)]
struct UsersQuery {
    in_channel: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn list_users(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Query(query): Query<UsersQuery>,
) -> ApiResult<Json<Vec<mm::User>>> {
    let channel_id = match query.in_channel.as_deref() {
        Some(id) => parse_mm_or_uuid(id)
            .ok_or_else(|| AppError::BadRequest("Invalid in_channel".to_string()))?,
        None => return Ok(Json(vec![])),
    };

    let page = query.page.unwrap_or(0).max(0);
    let per_page = query.per_page.unwrap_or(60).clamp(1, 200);
    let offset = page * per_page;

    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)",
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(AppError::Forbidden("Not a member of this channel".to_string()));
    }

    let users: Vec<User> = sqlx::query_as(
        r#"
        SELECT u.*
        FROM users u
        JOIN channel_members cm ON u.id = cm.user_id
        WHERE cm.channel_id = $1 AND u.is_active = true
        ORDER BY u.username ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(channel_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let mm_users: Vec<mm::User> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(mm_users))
}

async fn get_user_image(
    State(_state): State<AppState>,
    Path(user_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let _user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;

    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0,
        0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120,
        156, 99, 0, 1, 0, 0, 5, 0, 1, 13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68,
        174, 66, 96, 130,
    ];

    Ok((
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        PNG_1X1,
    ))
}

/// POST /users/{user_id}/image - Upload user profile image
async fn upload_user_image(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let user_uuid = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;
    
    if user_uuid != auth.user_id {
        return Err(AppError::Forbidden("Cannot update other user's image".to_string()));
    }

    // Process multipart upload
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "image" {
            let content_type = field
                .content_type()
                .unwrap_or("image/png")
                .to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?
                .to_vec();

            // Upload to S3
            let key = format!("avatars/{}.png", user_uuid);
            state.s3_client.upload(&key, data, &content_type).await?;

            // Update user avatar_url
            let avatar_url = format!("/api/v4/users/{}/image", encode_mm_id(user_uuid));
            sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
                .bind(&avatar_url)
                .bind(user_uuid)
                .execute(&state.db)
                .await?;

            return Ok(Json(serde_json::json!({"status": "OK"})));
        }
    }

    Err(AppError::BadRequest("No image field found".to_string()))
}

async fn user_typing(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((user_id, channel_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;
    if user_id != auth.user_id {
         return Err(AppError::Forbidden("Mismatch user_id".to_string()));
    }

    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UserTyping,
        crate::realtime::TypingEvent {
            user_id: auth.user_id,
            display_name: "".to_string(), // Fetched by client usually
            thread_root_id: None,
        },
        Some(channel_id),
    ).with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    });

    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /custom_profile_attributes/fields - Custom profile attributes (stub)
async fn get_custom_profile_attributes() -> ApiResult<Json<Vec<serde_json::Value>>> {
    // MM Enterprise feature - return empty array for compatibility
    Ok(Json(vec![]))
}

/// GET /users/{user_id}/custom_profile_attributes - Per-user custom profile attributes (stub)
async fn get_user_custom_profile_attributes(
    Path(_user_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // MM Enterprise feature - return empty array for compatibility
    Ok(Json(vec![]))
}


