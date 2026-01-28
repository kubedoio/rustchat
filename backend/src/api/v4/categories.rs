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
