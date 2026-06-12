//! Admin permissions and roles endpoints

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::api::admin::require_admin;
use crate::api::AppState;
use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::models::Permission;
use crate::repositories::AdminRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/permissions", get(list_permissions))
        .route(
            "/admin/roles/{role}/permissions",
            get(get_role_permissions).put(update_role_permissions),
        )
}

// ============ Permissions ============

pub async fn list_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<Permission>>> {
    require_admin(&auth)?;

    let permissions = AdminRepository::new(&state.db).list_permissions().await?;

    Ok(Json(permissions))
}

pub async fn get_role_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(role): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    require_admin(&auth)?;

    let permissions = AdminRepository::new(&state.db)
        .get_role_permissions(&role)
        .await?;

    Ok(Json(permissions))
}

#[derive(Deserialize)]
pub(crate) struct RolePermissionsUpdate {
    permissions: Vec<String>,
}

pub async fn update_role_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(role): Path<String>,
    Json(input): Json<RolePermissionsUpdate>,
) -> ApiResult<Json<Vec<String>>> {
    require_admin(&auth)?;

    let valid_permissions: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM permissions WHERE id = ANY($1)")
            .bind(&input.permissions)
            .fetch_all(&state.db)
            .await?;

    let valid_ids: Vec<String> = valid_permissions.into_iter().map(|p| p.0).collect();

    AdminRepository::new(&state.db)
        .set_role_permissions(&role, &valid_ids)
        .await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    let role_name = role.clone();
    let permission_count = valid_ids.len();
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::RolePermissionUpdate,
            "role",
            None,
            serde_json::json!({
                "role": role_name,
                "permission_count": permission_count,
            }),
        )
        .await;
    });

    Ok(Json(valid_ids))
}
