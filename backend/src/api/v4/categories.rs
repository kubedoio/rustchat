use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use super::users::{
    create_category_internal, get_categories_internal, get_category_order_internal,
    resolve_user_id, update_categories_internal, update_category_order_internal,
    CreateCategoryRequest, UpdateCategoriesPayload,
};
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::parse_mm_or_uuid, models as mm};
use crate::repositories::{CategoryRepository, TeamRepository};

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
            get(get_category_order).put(update_category_order),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/channels/categories/{category_id}",
            get(get_category)
                .put(update_category)
                .delete(delete_category),
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
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    get_categories_internal(state, user_id, team_id).await
}

async fn create_category(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Json(input): Json<CreateCategoryRequest>,
) -> ApiResult<Json<mm::SidebarCategory>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    create_category_internal(state, user_id, team_id, input).await
}

async fn update_categories(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Json(input): Json<UpdateCategoriesPayload>,
) -> ApiResult<Json<Vec<mm::SidebarCategory>>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    update_categories_internal(state, user_id, team_id, input.into_request()).await
}

async fn get_category_order(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
) -> ApiResult<Json<Vec<String>>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    get_category_order_internal(state, user_id, team_id).await
}

async fn update_category_order(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<CategoriesPath>,
    Json(order): Json<Vec<String>>,
) -> ApiResult<Json<Vec<String>>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    update_category_order_internal(state, user_id, team_id, order).await
}

/// Resolves a team identifier to a UUID.
/// First tries to parse as UUID/mm-id, then falls back to looking up by team name.
async fn resolve_team_id(state: &AppState, team_id_str: &str) -> ApiResult<Uuid> {
    if let Some(team_id) = parse_mm_or_uuid(team_id_str) {
        return Ok(team_id);
    }

    let repo = TeamRepository::new(&state.db);
    let id = repo.get_id_by_name(team_id_str).await?
        .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;
    Ok(id)
}

#[derive(Deserialize)]
struct SingleCategoryPath {
    user_id: String,
    team_id: String,
    category_id: String,
}

/// GET /users/{user_id}/teams/{team_id}/channels/categories/{category_id}
async fn get_category(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(params): Path<SingleCategoryPath>,
) -> ApiResult<Json<mm::SidebarCategory>> {
    let user_id = resolve_user_id(&params.user_id, &auth)?;
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    let category_id = parse_mm_or_uuid(&params.category_id).unwrap_or_else(|| {
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, params.category_id.as_bytes())
    });

    let cat_repo = CategoryRepository::new(&state.db);

    // Fetch the specific category
    let category = cat_repo.get(category_id, user_id, team_id).await?
        .ok_or_else(|| crate::error::AppError::NotFound("Category not found".to_string()))?;

    // Get channels for this category
    let channel_ids = cat_repo.get_channel_ids(category_id).await.unwrap_or_default();

    let channel_ids: Vec<String> = channel_ids
        .into_iter()
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
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    let category_id = parse_mm_or_uuid(&params.category_id).unwrap_or_else(|| {
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, params.category_id.as_bytes())
    });

    let now = chrono::Utc::now().timestamp_millis();

    let cat_repo = CategoryRepository::new(&state.db);

    // Update the category
    let category = cat_repo.update_returning(
        category_id, user_id, team_id,
        input.display_name.as_deref(),
        input.sorting.as_deref(),
        input.muted,
        input.collapsed,
        now,
    ).await?;

    // Update channel assignments if provided
    if let Some(new_channel_ids) = &input.channel_ids {
        // Delete existing channel associations
        cat_repo.delete_channel_associations(category_id).await?;

        // Insert new associations
        for (idx, ch_id_str) in new_channel_ids.iter().enumerate() {
            if let Some(ch_id) = parse_mm_or_uuid(ch_id_str) {
                cat_repo.insert_channel_association(category_id, ch_id, idx as i32).await?;
            }
        }
    }

    // Get current channel_ids
    let channel_ids = cat_repo.get_channel_ids(category_id).await.unwrap_or_default();

    let channel_ids: Vec<String> = channel_ids
        .into_iter()
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
    let team_id = resolve_team_id(&state, &params.team_id).await?;
    let category_id = parse_mm_or_uuid(&params.category_id).unwrap_or_else(|| {
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, params.category_id.as_bytes())
    });

    let cat_repo = CategoryRepository::new(&state.db);

    // First check category exists
    let category = cat_repo.get(category_id, user_id, team_id).await?
        .ok_or_else(|| crate::error::AppError::NotFound("Category not found".to_string()))?;

    // Don't allow deleting default categories
    if matches!(
        category.type_field.as_str(),
        "channels" | "direct_messages" | "favorites"
    ) {
        return Err(crate::error::AppError::BadRequest(
            "Cannot delete default category".to_string(),
        ));
    }

    let now = chrono::Utc::now().timestamp_millis();

    // Find default category to move channels to
    let default_category_id = cat_repo.find_default_category(user_id, team_id).await?;

    // Move channels to default category if it exists
    if let Some(default_id) = default_category_id {
        cat_repo.migrate_channels_to_category(category_id, default_id).await?;
    } else {
        // If no default category, just delete the channel associations
        cat_repo.delete_channel_associations(category_id).await?;
    }

    // Soft delete the category
    cat_repo.soft_delete(category_id, now).await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}
