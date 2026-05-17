use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::api::admin::require_admin;
use crate::services::membership_policies::{
    AutoMembershipPolicyAudit, CreatePolicyRequest, PolicyRepository, PolicyWithTargets,
    UpdatePolicyRequest,
};

/// Query parameters for listing policies
#[derive(Debug, Deserialize)]
pub struct ListPoliciesQuery {
    scope_type: Option<String>,
    team_id: Option<Uuid>,
    enabled: Option<bool>,
}

/// Query parameters for audit log
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// List all policies
async fn list_policies(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListPoliciesQuery>,
) -> ApiResult<Json<Vec<PolicyWithTargets>>> {
    // Check permission for viewing membership policies
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to view membership policies".to_string(),
        ));
    }
    let repo = PolicyRepository::new(&state.db);

    let scope_type = query.scope_type.and_then(|s| match s.as_str() {
        "global" => Some(crate::services::membership_policies::PolicyScopeType::Global),
        "team" => Some(crate::services::membership_policies::PolicyScopeType::Team),
        _ => None,
    });

    let policies = repo
        .list_policies(scope_type, query.team_id, query.enabled)
        .await?;
    Ok(Json(policies))
}

/// Get a single policy by ID
async fn get_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(policy_id): Path<Uuid>,
) -> ApiResult<Json<PolicyWithTargets>> {
    // Check permission for viewing membership policies
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to view membership policies".to_string(),
        ));
    }
    let repo = PolicyRepository::new(&state.db);

    let policy = repo.get_policy(policy_id).await?.ok_or_else(|| {
        crate::error::AppError::NotFound(format!("Policy {} not found", policy_id))
    })?;

    Ok(Json(policy))
}

/// Create a new policy
async fn create_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    req: axum::extract::Request,
) -> ApiResult<Json<PolicyWithTargets>> {
    // Check permission FIRST before processing body
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to manage membership policies".to_string(),
        ));
    }

    // Read and log the raw body for debugging
    let (_parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024).await.map_err(|e| {
        tracing::error!("Failed to read request body: {}", e);
        crate::error::AppError::BadRequest(format!("Failed to read request body: {}", e))
    })?;

    tracing::debug!(
        "Create policy request body: {}",
        String::from_utf8_lossy(&bytes)
    );

    // Deserialize the request
    let req: CreatePolicyRequest = match serde_json::from_slice(&bytes) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to deserialize CreatePolicyRequest: {}", e);
            return Err(crate::error::AppError::Validation(format!(
                "Invalid request body: {}. Expected fields: name (string), scope_type ('global'|'team'), source_type ('all_users'|'auth_service'|'group'|'role'|'org'), enabled (boolean), targets (array)",
                e
            )));
        }
    };

    // Validate scope consistency
    match req.scope_type {
        crate::services::membership_policies::PolicyScopeType::Global if req.team_id.is_some() => {
            return Err(crate::error::AppError::BadRequest(
                "Global policies cannot have a team_id".to_string(),
            ));
        }
        crate::services::membership_policies::PolicyScopeType::Team if req.team_id.is_none() => {
            return Err(crate::error::AppError::BadRequest(
                "Team policies must have a team_id".to_string(),
            ));
        }
        _ => {}
    }

    let repo = PolicyRepository::new(&state.db);
    let policy = repo.create_policy(req).await?;

    Ok(Json(policy))
}

/// Update a policy
async fn update_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(policy_id): Path<Uuid>,
    Json(req): Json<UpdatePolicyRequest>,
) -> ApiResult<Json<PolicyWithTargets>> {
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to manage membership policies".to_string(),
        ));
    }

    let repo = PolicyRepository::new(&state.db);
    let policy = repo.update_policy(policy_id, req).await?.ok_or_else(|| {
        crate::error::AppError::NotFound(format!("Policy {} not found", policy_id))
    })?;

    Ok(Json(policy))
}

/// Delete a policy
async fn delete_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(policy_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to manage membership policies".to_string(),
        ));
    }

    let repo = PolicyRepository::new(&state.db);
    let deleted = repo.delete_policy(policy_id).await?;

    if !deleted {
        return Err(crate::error::AppError::NotFound(format!(
            "Policy {} not found",
            policy_id
        )));
    }

    Ok(Json(serde_json::json!({
        "status": "OK",
        "message": format!("Policy {} deleted", policy_id)
    })))
}

/// Get audit log for a policy
async fn get_policy_audit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(policy_id): Path<Uuid>,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Json<Vec<AutoMembershipPolicyAudit>>> {
    // Check permission for viewing membership policies
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to view membership policy audit logs".to_string(),
        ));
    }
    use crate::services::membership_policies::get_policy_audit_log;

    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    let audits = get_policy_audit_log(&state.db, policy_id, limit, offset).await?;
    Ok(Json(audits))
}

/// Get policy run status
async fn get_policy_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(policy_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check permission for viewing membership policies
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to view membership policy status".to_string(),
        ));
    }
    use crate::services::membership_policies::get_policy_last_run_status;

    let status = get_policy_last_run_status(&state.db, policy_id).await?;

    match status {
        Some((success, failed)) => Ok(Json(serde_json::json!({
            "policy_id": policy_id,
            "last_run": {
                "success_count": success,
                "failed_count": failed,
                "total": success + failed
            }
        }))),
        None => Ok(Json(serde_json::json!({
            "policy_id": policy_id,
            "last_run": null
        }))),
    }
}

/// Trigger manual policy run for a user (re-sync)
async fn trigger_user_resync(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to trigger user re-sync".to_string(),
        ));
    }

    // Get user's teams and apply policies for each
    let user_teams: Vec<(Uuid,)> =
        sqlx::query_as("SELECT team_id FROM team_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await?;

    let mut total_applied = 0;
    let mut total_failed = 0;

    for (team_id,) in &user_teams {
        use crate::services::membership_policies::apply_auto_membership_for_team_join;

        match apply_auto_membership_for_team_join(&state, user_id, *team_id, "manual_resync").await
        {
            Ok(entries) => {
                total_applied += entries.iter().filter(|e| e.status == "success").count();
                total_failed += entries.iter().filter(|e| e.status == "failed").count();
            }
            Err(e) => {
                tracing::error!(
                    "Failed to apply policies for user {} team {}: {}",
                    user_id,
                    team_id,
                    e
                );
                total_failed += 1;
            }
        }
    }

    Ok(Json(serde_json::json!({
        "status": "OK",
        "user_id": user_id,
        "teams_processed": user_teams.len(),
        "memberships_applied": total_applied,
        "memberships_failed": total_failed
    })))
}

/// Get metadata for membership policy configuration
/// Returns available source types and their configuration options
async fn get_policy_metadata(
    _state: State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    // Check permission for viewing membership policies
    if !auth.has_permission(&permissions::TEAM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to view membership policy metadata".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({
        "source_types": [
            {
                "value": "all_users",
                "label": "All Users",
                "description": "Apply to all users in the system",
                "config_fields": []
            },
            {
                "value": "auth_service",
                "label": "Authentication Service",
                "description": "Apply to users from a specific authentication provider",
                "config_fields": [
                    {
                        "key": "auth_provider",
                        "label": "Auth Provider",
                        "type": "string",
                        "description": "Authentication provider key (e.g., 'oidc', 'github', 'google')",
                        "required": false,
                        "placeholder": "e.g., oidc, github, google"
                    }
                ]
            },
            {
                "value": "group",
                "label": "Group Membership",
                "description": "Apply to members of specific groups (internal groups or OIDC-synced groups)",
                "config_fields": [
                    {
                        "key": "group_ids",
                        "label": "Group IDs",
                        "type": "array",
                        "description": "UUIDs of groups",
                        "required": false
                    },
                    {
                        "key": "group_names",
                        "label": "Group Names",
                        "type": "array",
                        "description": "Names of groups (for OIDC groups, use the group name from your IdP)",
                        "required": false,
                        "placeholder": "e.g., engineering, admin, support"
                    }
                ]
            },
            {
                "value": "role",
                "label": "User Role",
                "description": "Apply to users with specific roles",
                "config_fields": [
                    {
                        "key": "roles",
                        "label": "Roles",
                        "type": "array",
                        "description": "Role names",
                        "required": true,
                        "placeholder": "e.g., member, admin, system_admin"
                    }
                ]
            },
            {
                "value": "org",
                "label": "Organization",
                "description": "Apply to users in specific organizations",
                "config_fields": [
                    {
                        "key": "org_ids",
                        "label": "Organization IDs",
                        "type": "array",
                        "description": "UUIDs of organizations",
                        "required": true
                    }
                ]
            }
        ],
        "scope_types": [
            {
                "value": "global",
                "label": "Global",
                "description": "Applies to all users regardless of team membership"
            },
            {
                "value": "team",
                "label": "Team",
                "description": "Only applies within a specific team context"
            }
        ],
        "target_types": [
            {
                "value": "team",
                "label": "Team",
                "description": "Add users to a team"
            },
            {
                "value": "channel",
                "label": "Channel",
                "description": "Add users to a channel"
            }
        ],
        "role_modes": [
            {
                "value": "member",
                "label": "Member",
                "description": "Regular member"
            },
            {
                "value": "admin",
                "label": "Admin",
                "description": "Team or channel administrator"
            }
        ]
    })))
}

/// Create router for membership policy admin routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/membership-policies",
            get(list_policies).post(create_policy),
        )
        .route(
            "/admin/membership-policies/{policy_id}",
            get(get_policy).put(update_policy).delete(delete_policy),
        )
        .route(
            "/admin/membership-policies/{policy_id}/audit",
            get(get_policy_audit),
        )
        .route(
            "/admin/membership-policies/{policy_id}/status",
            get(get_policy_status),
        )
        .route(
            "/admin/membership-policies/users/{user_id}/resync",
            post(trigger_user_resync),
        )
        .route(
            "/admin/membership-policies/metadata",
            get(get_policy_metadata),
        )
        .route("/admin/groups", get(list_admin_groups))
}
// ============================================================================
// Groups Management (for Membership Policies)
// ============================================================================

#[derive(Debug, serde::Serialize, FromRow)]
pub struct AdminGroupResponse {
    pub id: Uuid,
    pub name: Option<String>,
    pub display_name: String,
    pub description: String,
    pub source: String,
    pub remote_id: Option<String>,
    pub member_count: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListGroupsQuery {
    pub source: Option<String>,
    pub search: Option<String>,
}

/// List all groups for membership policy configuration
pub async fn list_admin_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListGroupsQuery>,
) -> ApiResult<Json<Vec<AdminGroupResponse>>> {
    require_admin(&auth)?;

    let groups: Vec<AdminGroupResponse> = sqlx::query_as(
        r#"
        SELECT 
            g.id,
            g.name,
            g.display_name,
            g.description,
            g.source,
            g.remote_id,
            (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) as member_count
        FROM groups g
        WHERE g.deleted_at IS NULL
          AND ($1::VARCHAR IS NULL OR g.source = $1)
          AND ($2::VARCHAR IS NULL OR 
               g.display_name ILIKE '%' || $2 || '%' OR 
               g.name ILIKE '%' || $2 || '%')
        ORDER BY g.display_name
        "#,
    )
    .bind(&query.source)
    .bind(&query.search)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(groups))
}
