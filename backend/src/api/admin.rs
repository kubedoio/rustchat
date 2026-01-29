//! Admin API endpoints for enterprise features

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    AddTeamMember,
    AuditLog,
    AuditLogQuery,
    CreateChannel,
    CreateRetentionPolicy,
    CreateSsoConfig,
    MiroTalkConfig,
    MiroTalkStats,
    Permission,
    RetentionPolicy,
    ServerConfig,
    ServerConfigResponse,
    // SiteConfig, AuthConfig, IntegrationsConfig, ComplianceConfig, EmailConfig,
    SsoConfig,
    TeamMember,
    TeamMemberResponse,
    UpdateChannel,
};
use crate::services::mirotalk::MiroTalkClient;
use sqlx::FromRow;

/// Build admin routes
pub fn router() -> Router<AppState> {
    Router::new()
        // Server config
        .route("/admin/config", get(get_config))
        .route("/admin/config/{category}", patch(update_config))
        // Audit logs
        .route("/admin/audit", get(list_audit_logs))
        // SSO
        .route(
            "/admin/sso",
            get(get_sso_config)
                .post(create_sso_config)
                .put(update_sso_config),
        )
        // Retention
        .route(
            "/admin/retention",
            get(list_retention_policies).post(create_retention_policy),
        )
        .route(
            "/admin/retention/{id}",
            get(get_retention_policy).delete(delete_retention_policy),
        )
        // Permissions
        .route("/admin/permissions", get(list_permissions))
        .route(
            "/admin/roles/{role}/permissions",
            get(get_role_permissions).put(update_role_permissions),
        )
        // Users management
        .route("/admin/users", get(list_users).post(create_admin_user))
        .route("/admin/users/{id}", patch(update_admin_user))
        .route(
            "/admin/users/{id}/deactivate",
            axum::routing::post(deactivate_user),
        )
        .route(
            "/admin/users/{id}/reactivate",
            axum::routing::post(reactivate_user),
        )
        // Teams & Channels management
        .route("/admin/teams", get(list_admin_teams))
        .route(
            "/admin/teams/{id}",
            get(get_admin_team).delete(delete_admin_team),
        )
        .route(
            "/admin/teams/{id}/members",
            get(list_team_members).post(add_team_member),
        )
        .route(
            "/admin/teams/{id}/members/{user_id}",
            axum::routing::delete(remove_team_member),
        )
        .route(
            "/admin/channels",
            get(list_admin_channels).post(create_admin_channel),
        )
        .route(
            "/admin/channels/{id}",
            patch(update_admin_channel).delete(delete_admin_channel),
        )
        // Stats & Health
        .route("/admin/stats", get(get_stats))
        .route("/admin/health", get(get_health))
        // Integrations - MiroTalk
        .route(
            "/admin/integrations/mirotalk",
            get(get_mirotalk_config).put(update_mirotalk_config),
        )
        .route(
            "/admin/integrations/mirotalk/test",
            axum::routing::post(test_mirotalk_connection),
        )
}

/// Check if user is admin
fn require_admin(auth: &AuthUser) -> ApiResult<()> {
    if auth.role != "system_admin" && auth.role != "org_admin" {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }
    Ok(())
}

// ... existing code ...

async fn list_team_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<TeamMemberResponse>>> {
    require_admin(&auth)?;

    let members = sqlx::query_as::<_, TeamMemberResponse>(
        r#"
        SELECT tm.team_id, tm.user_id, tm.role, tm.created_at,
               u.username, u.display_name, u.avatar_url
        FROM team_members tm
        JOIN users u ON tm.user_id = u.id
        WHERE tm.team_id = $1
        ORDER BY u.username
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(members))
}

async fn add_team_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddTeamMember>,
) -> ApiResult<Json<TeamMember>> {
    require_admin(&auth)?;

    let member = sqlx::query_as::<_, TeamMember>(
        r#"
        INSERT INTO team_members (team_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(payload.user_id)
    .bind(payload.role.unwrap_or_else(|| "member".into()))
    .fetch_one(&state.db)
    .await?;

    // Also add user to all public channels in the team
    sqlx::query(
        r#"
        INSERT INTO channel_members (channel_id, user_id)
        SELECT c.id, $1 FROM channels c
        WHERE c.team_id = $2 AND c.channel_type = 'public'::channel_type
        ON CONFLICT (channel_id, user_id) DO NOTHING
        "#,
    )
    .bind(payload.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(member))
}

async fn remove_team_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "removed"})))
}

async fn create_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateChannel>,
) -> ApiResult<Json<crate::models::channel::Channel>> {
    require_admin(&auth)?;

    // Create channel
    let channel: crate::models::channel::Channel = sqlx::query_as(
        r#"
        INSERT INTO channels (team_id, name, display_name, purpose, type, creator_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(input.team_id)
    .bind(&input.name)
    .bind(&input.display_name)
    .bind(&input.purpose)
    .bind(input.channel_type)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    // Broadcast event
    let broadcast = if channel.channel_type == crate::models::ChannelType::Public {
        // Broadcast to entire team
        crate::realtime::WsBroadcast {
            team_id: Some(input.team_id),
            channel_id: None,
            user_id: None,
            exclude_user_id: None,
        }
    } else {
        // Private channel: broadcast only to creator (admin)
        crate::realtime::WsBroadcast {
            user_id: Some(auth.user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        }
    };

    let event = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::ChannelCreated,
        channel.clone(),
        Some(channel.id),
    )
    .with_broadcast(broadcast);

    state.ws_hub.broadcast(event).await;

    Ok(Json(channel))
}

async fn update_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateChannel>,
) -> ApiResult<Json<crate::models::channel::Channel>> {
    require_admin(&auth)?;

    // Update fields
    if let Some(ref display_name) = input.display_name {
        sqlx::query("UPDATE channels SET display_name = $1 WHERE id = $2")
            .bind(display_name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref purpose) = input.purpose {
        sqlx::query("UPDATE channels SET purpose = $1 WHERE id = $2")
            .bind(purpose)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ref header) = input.header {
        sqlx::query("UPDATE channels SET header = $1 WHERE id = $2")
            .bind(header)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    let channel: crate::models::channel::Channel =
        sqlx::query_as("SELECT * FROM channels WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(channel))
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
    let offset = (page - 1) * per_page;

    let logs: Vec<AuditLog> = sqlx::query_as(
        r#"
        SELECT * FROM audit_logs
        WHERE ($1::VARCHAR IS NULL OR action = $1)
          AND ($2::VARCHAR IS NULL OR target_type = $2)
          AND ($3::UUID IS NULL OR actor_user_id = $3)
          AND ($4::TIMESTAMPTZ IS NULL OR created_at >= $4)
          AND ($5::TIMESTAMPTZ IS NULL OR created_at <= $5)
        ORDER BY created_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(&query.action)
    .bind(&query.target_type)
    .bind(query.actor_user_id)
    .bind(query.from_date)
    .bind(query.to_date)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(logs))
}

// ============ SSO Configuration ============

async fn get_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Option<SsoConfig>>> {
    require_admin(&auth)?;

    let org_id = auth
        .org_id
        .ok_or_else(|| AppError::BadRequest("No organization context".to_string()))?;

    let config: Option<SsoConfig> = sqlx::query_as("SELECT * FROM sso_configs WHERE org_id = $1")
        .bind(org_id)
        .fetch_optional(&state.db)
        .await?;

    Ok(Json(config))
}

async fn create_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateSsoConfig>,
) -> ApiResult<Json<SsoConfig>> {
    require_admin(&auth)?;

    let org_id = auth
        .org_id
        .ok_or_else(|| AppError::BadRequest("No organization context".to_string()))?;

    // Validate provider
    if input.provider != "oidc" && input.provider != "saml" {
        return Err(AppError::Validation(
            "Provider must be 'oidc' or 'saml'".to_string(),
        ));
    }

    let scopes = input.scopes.unwrap_or_else(|| {
        vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
        ]
    });

    let config: SsoConfig = sqlx::query_as(
        r#"
        INSERT INTO sso_configs (org_id, provider, display_name, issuer_url, client_id, client_secret_encrypted, scopes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (org_id) DO UPDATE SET
            provider = $2, display_name = $3, issuer_url = $4, 
            client_id = $5, client_secret_encrypted = $6, scopes = $7
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(&input.provider)
    .bind(&input.display_name)
    .bind(&input.issuer_url)
    .bind(&input.client_id)
    .bind(&input.client_secret) // Should be encrypted in production
    .bind(&scopes)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(config))
}

async fn update_sso_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateSsoConfig>,
) -> ApiResult<Json<SsoConfig>> {
    // Same as create with upsert
    create_sso_config(State(state), auth, Json(input)).await
}

// ============ Retention Policies ============

async fn list_retention_policies(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<RetentionPolicy>>> {
    require_admin(&auth)?;

    let policies: Vec<RetentionPolicy> = if let Some(org_id) = auth.org_id {
        sqlx::query_as(
            "SELECT * FROM retention_policies WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(&state.db)
        .await?
    } else if auth.role == "system_admin" {
        sqlx::query_as("SELECT * FROM retention_policies ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?
    } else {
        vec![]
    };

    Ok(Json(policies))
}

async fn create_retention_policy(
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

    let policy: RetentionPolicy = sqlx::query_as(
        r#"
        INSERT INTO retention_policies (org_id, team_id, channel_id, retention_days, delete_files)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.org_id)
    .bind(input.team_id)
    .bind(input.channel_id)
    .bind(input.retention_days)
    .bind(input.delete_files)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(policy))
}

async fn get_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RetentionPolicy>> {
    require_admin(&auth)?;

    let policy: RetentionPolicy = sqlx::query_as("SELECT * FROM retention_policies WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Policy not found".to_string()))?;

    Ok(Json(policy))
}

async fn delete_retention_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM retention_policies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ MiroTalk Integration ============

async fn get_mirotalk_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<MiroTalkConfig>> {
    require_admin(&auth)?;

    let config: MiroTalkConfig = sqlx::query_as("SELECT * FROM mirotalk_config WHERE is_active = true")
        .fetch_optional(&state.db)
        .await?
        .unwrap_or_else(|| MiroTalkConfig {
            is_active: true,
            mode: crate::models::MiroTalkMode::Disabled,
            base_url: "".to_string(),
            api_key_secret: "".to_string(),
            default_room_prefix: None,
            join_behavior: crate::models::JoinBehavior::NewTab,
            updated_at: chrono::Utc::now(),
            updated_by: None,
        });

    Ok(Json(config))
}

async fn update_mirotalk_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<MiroTalkConfig>,
) -> ApiResult<Json<MiroTalkConfig>> {
    require_admin(&auth)?;

    // Upsert
    let config: MiroTalkConfig = sqlx::query_as(
        r#"
        INSERT INTO mirotalk_config (is_active, mode, base_url, api_key_secret, default_room_prefix, join_behavior, updated_by, updated_at)
        VALUES (true, $1, $2, $3, $4, $5, $6, NOW())
        ON CONFLICT (is_active) DO UPDATE SET
            mode = $1,
            base_url = $2,
            api_key_secret = $3,
            default_room_prefix = $4,
            join_behavior = $5,
            updated_by = $6,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(input.mode)
    .bind(input.base_url)
    .bind(input.api_key_secret)
    .bind(input.default_room_prefix)
    .bind(input.join_behavior)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(config))
}

async fn test_mirotalk_connection(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<MiroTalkStats>> {
    require_admin(&auth)?;

    // 1. Fetch current config
    let config: MiroTalkConfig = sqlx::query_as("SELECT * FROM mirotalk_config WHERE is_active = true")
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::Config("MiroTalk config not found".to_string()))?;

    // 2. Init client and call stats
    let client = MiroTalkClient::new(config, state.http_client.clone())?;
    let stats = client.stats().await?;

    Ok(Json(stats))
}

// ============ Permissions ============

async fn list_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<Permission>>> {
    require_admin(&auth)?;

    let permissions: Vec<Permission> =
        sqlx::query_as("SELECT * FROM permissions ORDER BY category, id")
            .fetch_all(&state.db)
            .await?;

    Ok(Json(permissions))
}

async fn get_role_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(role): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    require_admin(&auth)?;

    let permissions: Vec<(String,)> =
        sqlx::query_as("SELECT permission_id FROM role_permissions WHERE role = $1")
            .bind(&role)
            .fetch_all(&state.db)
            .await?;

    Ok(Json(permissions.into_iter().map(|p| p.0).collect()))
}

#[derive(Deserialize)]
struct RolePermissionsUpdate {
    permissions: Vec<String>,
}

async fn update_role_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(role): Path<String>,
    Json(input): Json<RolePermissionsUpdate>,
) -> ApiResult<Json<Vec<String>>> {
    require_admin(&auth)?;

    let valid_permissions: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM permissions WHERE id = ANY($1)",
    )
    .bind(&input.permissions)
    .fetch_all(&state.db)
    .await?;

    let valid_ids: Vec<String> = valid_permissions.into_iter().map(|p| p.0).collect();

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM role_permissions WHERE role = $1")
        .bind(&role)
        .execute(&mut *tx)
        .await?;

    for permission_id in &valid_ids {
        sqlx::query("INSERT INTO role_permissions (role, permission_id) VALUES ($1, $2)")
            .bind(&role)
            .bind(permission_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Json(valid_ids))
}

/// Helper function to log audit events
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub async fn log_audit_event(
    db: &sqlx::PgPool,
    actor_user_id: Option<Uuid>,
    actor_ip: Option<String>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    old_values: Option<serde_json::Value>,
    new_values: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, actor_ip, action, target_type, target_id, old_values, new_values)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(actor_user_id)
    .bind(actor_ip)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(old_values)
    .bind(new_values)
    .execute(db)
    .await?;

    Ok(())
}

// ============ Server Configuration ============

async fn get_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<ServerConfigResponse>> {
    require_admin(&auth)?;

    let config: ServerConfig = sqlx::query_as("SELECT * FROM server_config WHERE id = 'default'")
        .fetch_one(&state.db)
        .await?;

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
        "email" => "email",
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

    Ok(Json(result.0 .0))
}

// ============ User Management ============

#[derive(Debug, serde::Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub role: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct UsersListResponse {
    pub users: Vec<crate::models::User>,
    pub total: i64,
}

async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListUsersQuery>,
) -> ApiResult<Json<UsersListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let users: Vec<crate::models::User> = sqlx::query_as(
        r#"
        SELECT * FROM users
        WHERE ($1::BOOL IS NULL OR is_active = $1)
          AND ($2::VARCHAR IS NULL OR role = $2)
          AND ($3::VARCHAR IS NULL OR username ILIKE '%' || $3 || '%' OR email ILIKE '%' || $3 || '%')
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(match query.status.as_deref() {
        Some("active") => Some(true),
        Some("inactive") => Some(false),
        _ => None,
    })
    .bind(&query.role)
    .bind(&query.search)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(UsersListResponse {
        users,
        total: total.0,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
    pub display_name: Option<String>,
}

async fn create_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateUserInput>,
) -> ApiResult<Json<crate::models::User>> {
    require_admin(&auth)?;

    let password_hash = crate::auth::hash_password(&input.password)?;
    let role = input.role.unwrap_or_else(|| "member".to_string());

    let user: crate::models::User = sqlx::query_as(
        r#"
        INSERT INTO users (username, email, password_hash, role, display_name, is_active)
        VALUES ($1, $2, $3, $4, $5, true)
        RETURNING *
        "#,
    )
    .bind(&input.username)
    .bind(&input.email)
    .bind(&password_hash)
    .bind(&role)
    .bind(&input.display_name)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateUserInput {
    pub role: Option<String>,
    pub display_name: Option<String>,
}

async fn update_admin_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUserInput>,
) -> ApiResult<Json<crate::models::User>> {
    require_admin(&auth)?;

    let user: crate::models::User = sqlx::query_as(
        r#"
        UPDATE users SET
            role = COALESCE($1, role),
            display_name = COALESCE($2, display_name),
            updated_at = NOW()
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(&input.role)
    .bind(&input.display_name)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user))
}

async fn deactivate_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deactivated"})))
}

async fn reactivate_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("UPDATE users SET is_active = true, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "reactivated"})))
}

// ============ Stats & Health ============

#[derive(Debug, serde::Serialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_teams: i64,
    pub total_channels: i64,
    pub messages_24h: i64,
    pub files_count: i64,
}

async fn get_stats(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<SystemStats>> {
    require_admin(&auth)?;

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let active_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = true")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let total_teams: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let total_channels: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM channels")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let messages_24h: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM posts WHERE created_at > NOW() - INTERVAL '24 hours'")
            .fetch_one(&state.db)
            .await
            .unwrap_or((0,));
    let files_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    Ok(Json(SystemStats {
        total_users: total_users.0,
        active_users: active_users.0,
        total_teams: total_teams.0,
        total_channels: total_channels.0,
        messages_24h: messages_24h.0,
        files_count: files_count.0,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub database: DatabaseHealth,
    pub storage: StorageHealth,
    pub websocket: WebSocketHealth,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub latency_ms: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct StorageHealth {
    pub connected: bool,
    #[serde(rename = "type")]
    pub storage_type: String,
}

#[derive(Debug, serde::Serialize)]
pub struct WebSocketHealth {
    pub active_connections: u64,
}

async fn get_health(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<HealthStatus>> {
    require_admin(&auth)?;

    // Check DB
    let start = std::time::Instant::now();
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let db_latency = start.elapsed().as_millis() as u64;

    Ok(Json(HealthStatus {
        status: if db_ok {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        database: DatabaseHealth {
            connected: db_ok,
            latency_ms: db_latency,
        },
        storage: StorageHealth {
            connected: true,
            storage_type: "s3".to_string(),
        },
        websocket: WebSocketHealth {
            active_connections: state.ws_hub.count_connections().await as u64,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    }))
}

// ============ Teams & Channels Management ============

#[derive(Debug, serde::Deserialize)]
pub struct ListTeamsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, serde::Serialize, FromRow)]
pub struct AdminTeamResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub team: crate::models::team::Team,
    pub members_count: i64,
    pub channels_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminTeamsListResponse {
    pub teams: Vec<AdminTeamResponse>,
    pub total: i64,
}

async fn list_admin_teams(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListTeamsQuery>,
) -> ApiResult<Json<AdminTeamsListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let teams: Vec<AdminTeamResponse> = sqlx::query_as(
        r#"
        SELECT t.*, 
               (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) as members_count,
               (SELECT COUNT(*) FROM channels WHERE team_id = t.id) as channels_count
        FROM teams t
        WHERE ($1::VARCHAR IS NULL OR t.name ILIKE '%' || $1 || '%' OR t.display_name ILIKE '%' || $1 || '%')
        ORDER BY t.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&query.search)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(AdminTeamsListResponse {
        teams,
        total: total.0,
    }))
}

async fn get_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AdminTeamResponse>> {
    require_admin(&auth)?;

    let team: AdminTeamResponse = sqlx::query_as(
        r#"
        SELECT t.*, 
               (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) as members_count,
               (SELECT COUNT(*) FROM channels WHERE team_id = t.id) as channels_count
        FROM teams t
        WHERE t.id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(team))
}

async fn delete_admin_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    // Cascade delete in the database handles related members/channels/posts
    sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

#[derive(Debug, serde::Deserialize)]
pub struct ListChannelsQuery {
    pub team_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, serde::Serialize, FromRow)]
pub struct AdminChannelResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub channel: crate::models::channel::Channel,
    pub members_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminChannelsListResponse {
    pub channels: Vec<AdminChannelResponse>,
    pub total: i64,
}

async fn list_admin_channels(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListChannelsQuery>,
) -> ApiResult<Json<AdminChannelsListResponse>> {
    require_admin(&auth)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let channels: Vec<AdminChannelResponse> = sqlx::query_as(
        r#"
        SELECT c.*, 
               (SELECT COUNT(*) FROM channel_members WHERE channel_id = c.id) as members_count
        FROM channels c
        WHERE ($1::UUID IS NULL OR c.team_id = $1)
          AND ($2::VARCHAR IS NULL OR c.name ILIKE '%' || $2 || '%' OR c.display_name ILIKE '%' || $2 || '%')
        ORDER BY c.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(query.team_id)
    .bind(&query.search)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM channels WHERE ($1::UUID IS NULL OR team_id = $1)")
            .bind(query.team_id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(AdminChannelsListResponse {
        channels,
        total: total.0,
    }))
}

async fn delete_admin_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM channels WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}
