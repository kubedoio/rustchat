//! Admin retention policy endpoints

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use crate::api::{admin::require_admin, AppState};
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{CreateRetentionPolicy, RetentionPolicy};
use crate::repositories::AdminRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/retention",
            get(list_retention_policies).post(create_retention_policy),
        )
        .route(
            "/admin/retention/{id}",
            get(get_retention_policy).delete(delete_retention_policy),
        )
}

pub async fn list_retention_policies(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<RetentionPolicy>>> {
    require_admin(&auth)?;

    let is_global_admin = auth.has_permission(&crate::auth::policy::permissions::ADMIN_FULL);
    let policies = AdminRepository::new(&state.db)
        .list_retention_policies(auth.org_id, is_global_admin)
        .await?;

    Ok(Json(policies))
}

pub async fn create_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateRetentionPolicy>,
) -> ApiResult<Json<RetentionPolicy>> {
    require_admin(&auth)?;

    // Validate scope
    let scope_count = [input.org_id, input.team_id, input.channel_id]
        .iter()
        .filter(|x| x.is_some())
        .count();

    if scope_count != 1 {
        return Err(AppError::Validation(
            "Exactly one of org_id, team_id, or channel_id required".to_string(),
        ));
    }

    let policy = AdminRepository::new(&state.db)
        .insert_retention_policy(&input)
        .await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    let policy_id = policy.id;
    let retention_days = policy.retention_days;
    let delete_files = policy.delete_files;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::RetentionPolicyCreate,
            "retention_policy",
            Some(policy_id),
            serde_json::json!({
                "retention_days": retention_days,
                "delete_files": delete_files,
            }),
        )
        .await;
    });

    Ok(Json(policy))
}

pub async fn get_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RetentionPolicy>> {
    require_admin(&auth)?;

    let policy = AdminRepository::new(&state.db)
        .get_retention_policy(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Policy not found".to_string()))?;

    Ok(Json(policy))
}

pub async fn delete_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    AdminRepository::new(&state.db)
        .delete_retention_policy(id)
        .await?;

    let db = state.db.clone();
    let actor = auth.user_id;
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::RetentionPolicyDelete,
            "retention_policy",
            Some(id),
            serde_json::Value::Null,
        )
        .await;
    });

    Ok(Json(serde_json::json!({"status": "deleted"})))
}
