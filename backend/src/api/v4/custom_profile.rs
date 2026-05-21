//! Custom Profile Attributes API endpoints for Mattermost mobile compatibility

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use crate::models::{
    CustomProfileAttribute, CustomProfileField, CustomProfileFieldResponse,
    UserCustomProfileAttributeSimple,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/custom_profile_attributes/fields",
            get(get_custom_profile_fields).post(create_custom_profile_field),
        )
        .route(
            "/custom_profile_attributes/fields/{field_id}",
            patch(patch_custom_profile_field).delete(delete_custom_profile_field),
        )
        .route(
            "/custom_profile_attributes/values",
            patch(update_custom_profile_values),
        )
        .route(
            "/custom_profile_attributes/group",
            get(get_custom_profile_group),
        )
        .route(
            "/users/{user_id}/custom_profile_attributes",
            get(get_user_custom_profile_attributes).patch(patch_user_custom_profile_attributes),
        )
}

fn ensure_manage_system(auth: &MmAuthUser) -> ApiResult<()> {
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Insufficient permissions to manage custom profile fields".to_string(),
        ));
    }

    Ok(())
}

/// GET /custom_profile_attributes/fields - Get all custom profile field definitions
async fn get_custom_profile_fields(
    State(state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<CustomProfileFieldResponse>>> {
    let fields: Vec<CustomProfileField> = sqlx::query_as(
        r#"
        SELECT id, group_id, name, field_type, attrs, target_id, target_type,
               created_at, updated_at, deleted_at
        FROM custom_profile_fields
        WHERE deleted_at IS NULL
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let response: Vec<CustomProfileFieldResponse> = fields.into_iter().map(Into::into).collect();
    Ok(Json(response))
}

/// POST /custom_profile_attributes/fields - Create custom profile field (compatibility stub)
async fn create_custom_profile_field(
    State(_state): State<AppState>,
    auth: MmAuthUser,
    Json(_payload): Json<serde_json::Value>,
) -> ApiResult<axum::response::Response> {
    ensure_manage_system(&auth)?;

    Ok(crate::api::v4::mm_not_implemented(
        "api.custom_profile_attributes.create.not_implemented",
        "Creating custom profile fields is not implemented.",
        "The compatibility surface is present, but this operation is not implemented in this build.",
    )
    .into_response())
}

/// PATCH /custom_profile_attributes/fields/{field_id} - Patch custom profile field (compatibility stub)
async fn patch_custom_profile_field(
    State(_state): State<AppState>,
    auth: MmAuthUser,
    Path(_field_id): Path<String>,
    Json(_payload): Json<serde_json::Value>,
) -> ApiResult<axum::response::Response> {
    ensure_manage_system(&auth)?;

    Ok(crate::api::v4::mm_not_implemented(
        "api.custom_profile_attributes.patch.not_implemented",
        "Patching custom profile fields is not implemented.",
        "The compatibility surface is present, but this operation is not implemented in this build.",
    )
    .into_response())
}

/// DELETE /custom_profile_attributes/fields/{field_id} - Delete custom profile field (compatibility stub)
async fn delete_custom_profile_field(
    State(_state): State<AppState>,
    auth: MmAuthUser,
    Path(_field_id): Path<String>,
) -> ApiResult<axum::response::Response> {
    ensure_manage_system(&auth)?;

    Ok(crate::api::v4::mm_not_implemented(
        "api.custom_profile_attributes.delete.not_implemented",
        "Deleting custom profile fields is not implemented.",
        "The compatibility surface is present, but this operation is not implemented in this build.",
    )
    .into_response())
}

/// GET /custom_profile_attributes/group - Return CPA group metadata
async fn get_custom_profile_group(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "id": "custom_profile_attributes"
    })))
}

/// GET /users/{user_id}/custom_profile_attributes - Get a user's custom profile attributes
async fn get_user_custom_profile_attributes(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<UserCustomProfileAttributeSimple>> {
    let user_id = parse_mm_or_uuid(&user_id).ok_or_else(|| AppError::InvalidUserId)?;

    let attrs: Vec<CustomProfileAttribute> = sqlx::query_as(
        r#"
        SELECT id, field_id, user_id, value
        FROM custom_profile_attributes
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let mut result: HashMap<String, serde_json::Value> = HashMap::new();
    for attr in attrs {
        result.insert(
            encode_mm_id(attr.field_id),
            decode_custom_profile_value(&attr.value),
        );
    }

    Ok(Json(result))
}

/// PATCH /custom_profile_attributes/values - Update custom profile attribute values
async fn update_custom_profile_values(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(values): Json<UserCustomProfileAttributeSimple>,
) -> ApiResult<Json<UserCustomProfileAttributeSimple>> {
    let updated_values = patch_custom_profile_values_for_user(&state, auth.user_id, values).await?;
    Ok(Json(updated_values))
}

/// PATCH /users/{user_id}/custom_profile_attributes - Update custom profile values for target user
async fn patch_user_custom_profile_attributes(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    Json(values): Json<UserCustomProfileAttributeSimple>,
) -> ApiResult<Json<UserCustomProfileAttributeSimple>> {
    let target_user_id = parse_mm_or_uuid(&user_id).ok_or_else(|| AppError::InvalidUserId)?;

    if !auth.can_access_owned(target_user_id, &permissions::USER_MANAGE) {
        return Err(AppError::Forbidden(
            "Cannot update another user's custom profile attributes".to_string(),
        ));
    }

    let updated_values =
        patch_custom_profile_values_for_user(&state, target_user_id, values).await?;
    Ok(Json(updated_values))
}

async fn patch_custom_profile_values_for_user(
    state: &AppState,
    user_id: Uuid,
    values: UserCustomProfileAttributeSimple,
) -> ApiResult<UserCustomProfileAttributeSimple> {
    let mut updated_values = HashMap::with_capacity(values.len());

    for (field_id_str, value) in values {
        let field_id = parse_mm_or_uuid(&field_id_str)
            .ok_or_else(|| AppError::BadRequest(format!("Invalid field_id: {}", field_id_str)))?;

        let field_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM custom_profile_fields WHERE id = $1 AND deleted_at IS NULL)",
        )
        .bind(field_id)
        .fetch_one(&state.db)
        .await?;

        if !field_exists {
            return Err(AppError::NotFound(format!(
                "Custom profile field {field_id_str} not found"
            )));
        }

        let stored_value = serde_json::to_string(&value).map_err(|_| {
            AppError::BadRequest(format!(
                "Invalid custom profile value for field_id: {field_id_str}"
            ))
        })?;

        sqlx::query(
            r#"
            INSERT INTO custom_profile_attributes (field_id, user_id, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (field_id, user_id)
            DO UPDATE SET value = EXCLUDED.value
            "#,
        )
        .bind(field_id)
        .bind(user_id)
        .bind(&stored_value)
        .execute(&state.db)
        .await?;

        updated_values.insert(encode_mm_id(field_id), value);
    }

    Ok(updated_values)
}

fn decode_custom_profile_value(raw_value: &str) -> serde_json::Value {
    serde_json::from_str(raw_value)
        .unwrap_or_else(|_| serde_json::Value::String(raw_value.to_string()))
}
