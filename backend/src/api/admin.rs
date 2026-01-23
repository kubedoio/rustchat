//! Admin API endpoints for enterprise features

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    AuditLog, AuditLogQuery, CreateRetentionPolicy, CreateSsoConfig,
    Permission, RetentionPolicy, SsoConfig, ServerConfig, ServerConfigResponse,
    // SiteConfig, AuthConfig, IntegrationsConfig, ComplianceConfig, EmailConfig,
};
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
        .route("/admin/sso", get(get_sso_config).post(create_sso_config).put(update_sso_config))
        // Retention
        .route("/admin/retention", get(list_retention_policies).post(create_retention_policy))
        .route("/admin/retention/{id}", get(get_retention_policy).delete(delete_retention_policy))
        // Permissions
        .route("/admin/permissions", get(list_permissions))
        .route("/admin/roles/{role}/permissions", get(get_role_permissions))
        // Users management
        .route("/admin/users", get(list_users).post(create_admin_user))
        .route("/admin/users/{id}", patch(update_admin_user))
        .route("/admin/users/{id}/deactivate", axum::routing::post(deactivate_user))
        .route("/admin/users/{id}/reactivate", axum::routing::post(reactivate_user))
        // Teams & Channels management
        .route("/admin/teams", get(list_admin_teams))
        .route("/admin/teams/{id}", get(get_admin_team).delete(delete_admin_team))
        .route("/admin/channels", get(list_admin_channels))
        .route("/admin/channels/{id}", axum::routing::delete(delete_admin_channel))
        // Stats & Health
        .route("/admin/stats", get(get_stats))
        .route("/admin/health", get(get_health))
}

/// Check if user is admin
fn require_admin(auth: &AuthUser) -> ApiResult<()> {
    if auth.role != "system_admin" && auth.role != "org_admin" {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }
    Ok(())
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

    let org_id = auth.org_id
        .ok_or_else(|| AppError::BadRequest("No organization context".to_string()))?;

    let config: Option<SsoConfig> = sqlx::query_as(
        "SELECT * FROM sso_configs WHERE org_id = $1"
    )
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

    let org_id = auth.org_id
        .ok_or_else(|| AppError::BadRequest("No organization context".to_string()))?;

    // Validate provider
    if input.provider != "oidc" && input.provider != "saml" {
        return Err(AppError::Validation("Provider must be 'oidc' or 'saml'".to_string()));
    }

    let scopes = input.scopes.unwrap_or_else(|| vec![
        "openid".to_string(), 
        "profile".to_string(), 
        "email".to_string()
    ]);

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
            "SELECT * FROM retention_policies WHERE org_id = $1 ORDER BY created_at DESC"
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
        return Err(AppError::Validation("Exactly one of org_id, team_id, or channel_id required".to_string()));
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

// ============ Permissions ============

async fn list_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<Permission>>> {
    require_admin(&auth)?;

    let permissions: Vec<Permission> = sqlx::query_as(
        "SELECT * FROM permissions ORDER BY category, id"
    )
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

    let permissions: Vec<(String,)> = sqlx::query_as(
        "SELECT permission_id FROM role_permissions WHERE role = $1"
    )
    .bind(&role)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(permissions.into_iter().map(|p| p.0).collect()))
}

/// Helper function to log audit events
#[allow(dead_code)]
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

    let config: ServerConfig = sqlx::query_as(
        "SELECT * FROM server_config WHERE id = 'default'"
    )
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
        _ => return Err(AppError::BadRequest(format!("Invalid config category: {}", category))),
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

    Ok(Json(result.0.0))
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

    Ok(Json(UsersListResponse { users, total: total.0 }))
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

async fn get_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<SystemStats>> {
    require_admin(&auth)?;

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db).await.unwrap_or((0,));
    let active_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = true")
        .fetch_one(&state.db).await.unwrap_or((0,));
    let total_teams: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
        .fetch_one(&state.db).await.unwrap_or((0,));
    let total_channels: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM channels")
        .fetch_one(&state.db).await.unwrap_or((0,));
    let messages_24h: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE created_at > NOW() - INTERVAL '24 hours'")
        .fetch_one(&state.db).await.unwrap_or((0,));
    let files_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&state.db).await.unwrap_or((0,));

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
        status: if db_ok { "healthy".to_string() } else { "degraded".to_string() },
        database: DatabaseHealth {
            connected: db_ok,
            latency_ms: db_latency,
        },
        storage: StorageHealth {
            connected: true,
            storage_type: "s3".to_string(),
        },
        websocket: WebSocketHealth {
            active_connections: 0, // TODO: track actual connections
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

    Ok(Json(AdminTeamsListResponse { teams, total: total.0 }))
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

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM channels WHERE ($1::UUID IS NULL OR team_id = $1)")
        .bind(query.team_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(AdminChannelsListResponse { channels, total: total.0 }))
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
