use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;

use super::extractors::MmAuthUser;
use super::users::{
    create_category_internal, get_categories_internal, resolve_user_id, update_categories_internal,
    update_category_order_internal, CreateCategoryRequest, UpdateCategoriesRequest,
};
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/users/{user_id}/teams/{team_id}/channels/categories",
            get(get_categories)
                .post(create_category)
                .put(update_categories),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/channels/categories/order",
            put(update_category_order),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/channels/categories/{category_id}",
            get(get_category).put(update_category).delete(delete_category),
        )
}

#[derive(Deserialize)]
struct CategoriesPath {
    user_id: String,
    team_id: String,
}

async fn get_categories(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
) -> ApiResult<Json<mm::SidebarCategories>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&params.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    get_categories_internal(state, user_id, team_id).await
}

async fn create_category(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Json(input): Json<CreateCategoryRequest>,
) -> ApiResult<Json<mm::SidebarCategory>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&params.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    create_category_internal(state, user_id, team_id, input).await
}

async fn update_categories(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Json(input): Json<UpdateCategoriesRequest>,
) -> ApiResult<Json<Vec<mm::SidebarCategory>>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&params.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    update_categories_internal(state, user_id, team_id, input).await
}

async fn update_category_order(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Json(order): Json<Vec<String>>,
) -> ApiResult<Json<Vec<String>>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&params.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    update_category_order_internal(state, user_id, team_id, order).await
}

#[derive(Deserialize)]
struct SingleCategoryPath {
    user_id: String,
    team_id: String,
    category_id: String,
}

/// Row struct for fetching categories from channel_categories table
#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: uuid::Uuid,
    team_id: uuid::Uuid,
    user_id: uuid::Uuid,
    #[sqlx(rename = "type")]
    type_field: String,
    display_name: String,
    sorting: String,
    muted: bool,
    collapsed: bool,
    sort_order: i32,
    create_at: i64,
    update_at: i64,
    delete_at: i64,
}

/// GET /users/{user_id}/teams/{team_id}/channels/categories/{category_id}
async fn get_category(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<SingleCategoryPath>,
) -> ApiResult<Json<mm::SidebarCategory>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&params.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    let category_id = parse_mm_or_uuid(&params.category_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid category_id".to_string()))?;

    // Fetch the specific category
    let category: Option<CategoryRow> = sqlx::query_as(
        "SELECT * FROM channel_categories WHERE id = $1 AND user_id = $2 AND team_id = $3 AND delete_at = 0"
    )
    .bind(category_id)
    .bind(user_id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await?;

    let category = category.ok_or_else(|| 
        crate::error::AppError::NotFound("Category not found".to_string()))?;

    // Get channels for this category
    let channel_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT channel_id FROM channel_category_channels WHERE category_id = $1 ORDER BY sort_order"
    )
    .bind(category_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let channel_ids: Vec<String> = channel_ids.into_iter()
        .map(crate::mattermost_compat::id::encode_mm_id)
        .collect();

    Ok(Json(mm::SidebarCategory {
        id: crate::mattermost_compat::id::encode_mm_id(category.id),
        user_id: crate::mattermost_compat::id::encode_mm_id(category.user_id),
        team_id: crate::mattermost_compat::id::encode_mm_id(category.team_id),
        sort_order: category.sort_order,
        sorting: category.sorting,
        category_type: category.type_field,
        display_name: category.display_name,
        muted: category.muted,
        collapsed: category.collapsed,
        channel_ids,
        create_at: category.create_at,
        update_at: category.update_at,
        delete_at: category.delete_at,
    }))
}

/// PUT /users/{user_id}/teams/{team_id}/channels/categories/{category_id}
#[derive(Deserialize)]
struct UpdateCategoryRequest {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    sorting: Option<String>,
    #[serde(default)]
    muted: Option<bool>,
    #[serde(default)]
    collapsed: Option<bool>,
    #[serde(default)]
    channel_ids: Option<Vec<String>>,
}

async fn update_category(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<SingleCategoryPath>,
    Json(input): Json<UpdateCategoryRequest>,
) -> ApiResult<Json<mm::SidebarCategory>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&params.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    let category_id = parse_mm_or_uuid(&params.category_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid category_id".to_string()))?;

    let now = chrono::Utc::now().timestamp_millis();

    // Update the category
    let category: CategoryRow = sqlx::query_as(
        r#"UPDATE channel_categories SET
            display_name = COALESCE($4, display_name),
            sorting = COALESCE($5, sorting),
            muted = COALESCE($6, muted),
            collapsed = COALESCE($7, collapsed),
            update_at = $8
        WHERE id = $1 AND user_id = $2 AND team_id = $3 AND delete_at = 0
        RETURNING *"#
    )
    .bind(category_id)
    .bind(user_id)
    .bind(team_id)
    .bind(&input.display_name)
    .bind(&input.sorting)
    .bind(input.muted)
    .bind(input.collapsed)
    .bind(now)
    .fetch_one(&state.db)
    .await?;

    // Update channel assignments if provided
    if let Some(new_channel_ids) = &input.channel_ids {
        // Delete existing channel associations
        sqlx::query("DELETE FROM channel_category_channels WHERE category_id = $1")
            .bind(category_id)
            .execute(&state.db)
            .await?;

        // Insert new associations
        for (idx, ch_id_str) in new_channel_ids.iter().enumerate() {
            if let Some(ch_id) = parse_mm_or_uuid(ch_id_str) {
                sqlx::query(
                    "INSERT INTO channel_category_channels (category_id, channel_id, sort_order) VALUES ($1, $2, $3)"
                )
                .bind(category_id)
                .bind(ch_id)
                .bind(idx as i32)
                .execute(&state.db)
                .await?;
            }
        }
    }

    // Get current channel_ids
    let channel_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT channel_id FROM channel_category_channels WHERE category_id = $1 ORDER BY sort_order"
    )
    .bind(category_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let channel_ids: Vec<String> = channel_ids.into_iter()
        .map(crate::mattermost_compat::id::encode_mm_id)
        .collect();

    Ok(Json(mm::SidebarCategory {
        id: crate::mattermost_compat::id::encode_mm_id(category.id),
        user_id: crate::mattermost_compat::id::encode_mm_id(category.user_id),
        team_id: crate::mattermost_compat::id::encode_mm_id(category.team_id),
        sort_order: category.sort_order,
        sorting: category.sorting,
        category_type: category.type_field,
        display_name: category.display_name,
        muted: category.muted,
        collapsed: category.collapsed,
        channel_ids,
        create_at: category.create_at,
        update_at: now,
        delete_at: category.delete_at,
    }))
}

/// DELETE /users/{user_id}/teams/{team_id}/channels/categories/{category_id}
async fn delete_category(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<SingleCategoryPath>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&params.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    let category_id = parse_mm_or_uuid(&params.category_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid category_id".to_string()))?;

    // First check category exists
    let category: Option<CategoryRow> = sqlx::query_as(
        "SELECT * FROM channel_categories WHERE id = $1 AND user_id = $2 AND team_id = $3 AND delete_at = 0"
    )
    .bind(category_id)
    .bind(user_id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await?;

    let category = category.ok_or_else(|| 
        crate::error::AppError::NotFound("Category not found".to_string()))?;

    // Don't allow deleting default categories
    if matches!(category.type_field.as_str(), "channels" | "direct_messages" | "favorites") {
        return Err(crate::error::AppError::BadRequest("Cannot delete default category".to_string()));
    }

    let now = chrono::Utc::now().timestamp_millis();

    // Find default category to move channels to
    let default_category_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM channel_categories WHERE user_id = $1 AND team_id = $2 AND type = 'channels' AND delete_at = 0"
    )
    .bind(user_id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await?;

    // Move channels to default category if it exists
    if let Some(default_id) = default_category_id {
        sqlx::query(
            "UPDATE channel_category_channels SET category_id = $1 WHERE category_id = $2"
        )
        .bind(default_id)
        .bind(category_id)
        .execute(&state.db)
        .await?;
    } else {
        // If no default category, just delete the channel associations
        sqlx::query("DELETE FROM channel_category_channels WHERE category_id = $1")
            .bind(category_id)
            .execute(&state.db)
            .await?;
    }

    // Soft delete the category (set delete_at)
    sqlx::query("UPDATE channel_categories SET delete_at = $2 WHERE id = $1")
        .bind(category_id)
        .bind(now)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}


