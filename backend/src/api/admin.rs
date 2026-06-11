//! Admin API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};

use super::AppState;
use crate::auth::policy::{permissions as policy_permissions, AuthzResult, PolicyEngine};
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{AuditLog, AuditLogQuery, ServerConfigResponse};
use crate::repositories::AdminRepository;

/// Build admin routes
pub fn router() -> Router<AppState> {
    // Email routes
    let email_routes = super::admin_email::router();

    Router::new()
        // Merge email routes
        .merge(email_routes)
        // Server config
        .route("/admin/config", get(get_config))
        .route("/admin/config/{category}", patch(update_config))
        // Audit logs
        .route("/admin/audit", get(list_audit_logs))
        // SSO
        .merge(super::admin_sso::router())
        // Retention
        .merge(super::admin_retention::router())
        // Permissions
        .merge(super::admin_permissions::router())
        // Users management
        .merge(super::admin_users::router())
        // Teams & Channels management
        .merge(super::admin_teams::router())
        // Stats & Health
        .merge(super::admin_stats::router())
        // Plugins
        .merge(super::admin_plugins::router())
        // Membership policies
        .merge(super::admin_membership_policies::router())
        // Audit endpoints
        .merge(super::admin_audit::router())
}

/// Check if user is admin
pub fn require_admin(auth: &AuthUser) -> ApiResult<()> {
    match PolicyEngine::check_permission(&auth.role, &policy_permissions::SYSTEM_MANAGE) {
        AuthzResult::Allow => Ok(()),
        AuthzResult::Deny(_) => Err(AppError::AdminRequired),
    }
}

pub fn require_global_admin(auth: &AuthUser) -> ApiResult<()> {
    match PolicyEngine::check_permission(&auth.role, &policy_permissions::ADMIN_FULL) {
        AuthzResult::Allow => Ok(()),
        AuthzResult::Deny(_) => Err(AppError::Forbidden(
            "Global admin access required".to_string(),
        )),
    }
}

pub async fn insert_admin_audit_log(
    db: &sqlx::PgPool,
    actor_user_id: Option<uuid::Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<uuid::Uuid>,
    metadata: serde_json::Value,
) -> ApiResult<()> {
    AdminRepository::new(db)
        .insert_audit_log(actor_user_id, action, target_type, target_id, metadata)
        .await
}

// ============ Server Configuration ============

async fn get_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<ServerConfigResponse>> {
    require_admin(&auth)?;

    let config = AdminRepository::new(&state.db).get_server_config().await?;

    Ok(Json(config.into()))
}

async fn update_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let column = match category.as_str() {
        "site" => "site",
        "authentication" => "authentication",
        "integrations" => "integrations",
        "compliance" => "compliance",
        "experimental" => "experimental",
        _ => {
            return Err(AppError::BadRequest(format!(
                "Invalid config category: {}",
                category
            )))
        }
    };

    let query = format!(
        "UPDATE server_config SET {} = $1, updated_at = NOW(), updated_by = $2 WHERE id = 'default' RETURNING {}",
        column, column
    );

    let result: (sqlx::types::Json<serde_json::Value>,) = sqlx::query_as(&query)
        .bind(sqlx::types::Json(&body))
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    // Broadcast config update to all connected users
    let event = crate::realtime::events::WsEnvelope::event(
        crate::realtime::events::EventType::ConfigUpdated,
        serde_json::json!({
            "category": category,
            "config": result.0.0
        }),
        None,
    );
    state.ws_hub.broadcast(event).await;

    let db = state.db.clone();
    let actor = auth.user_id;
    let category_clone = category.clone();
    tokio::spawn(async move {
        let _ = crate::services::audit::audit(
            &db,
            Some(actor),
            crate::services::audit::AuditAction::ConfigUpdate,
            "config",
            None,
            serde_json::json!({ "category": category_clone }),
        )
        .await;
    });

    Ok(Json(result.0 .0))
}

// ============ Audit Logs ============

async fn list_audit_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Json<Vec<AuditLog>>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(200);

    let logs = AdminRepository::new(&state.db)
        .list_audit_logs(&query, page, per_page)
        .await?;

    Ok(Json(logs))
}
