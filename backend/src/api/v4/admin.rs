use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/audits", get(get_audits))
        .route("/admin/keycloak/sync", post(trigger_keycloak_sync))
        .route(
            "/admin/keycloak/sync/users/{user_id}",
            post(trigger_keycloak_user_sync),
        )
}
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::ApiResult;
use crate::error::AppError;
use crate::mattermost_compat::id::parse_mm_or_uuid;
use crate::mattermost_compat::models as mm;
use crate::services::keycloak_sync;

pub async fn get_audits(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Audit>>> {
    // Audit logs contain sensitive PII (IP addresses) - restrict to system admins only
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Missing permission to view audit logs".to_string(),
        ));
    }
    let audits: Vec<mm::Audit> = sqlx::query_as(
        r#"
        SELECT id::text, 
               (extract(epoch from created_at)*1000)::int8 as create_at,
               actor_user_id::text as user_id,
               action,
               metadata::text as extra_info,
               actor_ip as ip_address,
               '' as session_id
        FROM audit_logs
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(audits))
}

async fn trigger_keycloak_sync(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Missing permission to run Keycloak sync".to_string(),
        ));
    }

    let report = keycloak_sync::run_full_sync(&state).await?;
    Ok(Json(serde_json::json!({
        "status": "OK",
        "report": report
    })))
}

async fn trigger_keycloak_user_sync(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::Forbidden(
            "Missing permission to run Keycloak user sync".to_string(),
        ));
    }

    let user_uuid: Uuid = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::InvalidUserId)?;
    let report = keycloak_sync::resync_user(&state, user_uuid).await?;
    Ok(Json(serde_json::json!({
        "status": "OK",
        "report": report
    })))
}
