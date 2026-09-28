//! Admin audit API for membership policies and system-wide auditing

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::api::{admin::require_admin, AppState};
use crate::auth::AuthUser;
use crate::error::ApiResult;

/// Query parameters for audit log listing
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    #[serde(default)]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub policy_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub status: Option<String>,
    pub action: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

fn default_per_page() -> i64 {
    50
}

/// Audit log entry with details
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub policy_id: Option<Uuid>,
    pub policy_name: Option<String>,
    pub run_id: Uuid,
    pub user_id: Uuid,
    pub username: Option<String>,
    pub target_type: String,
    pub target_id: Uuid,
    pub target_name: Option<String>,
    pub action: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Summary statistics for audit dashboard
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditSummary {
    pub total_operations_24h: i64,
    pub successful_operations_24h: i64,
    pub failed_operations_24h: i64,
    pub failure_rate_24h: f64,
    pub pending_operations: i64,
    pub policies_with_failures: i64,
}

/// Failure statistics per policy
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct PolicyFailureStats {
    pub policy_id: Uuid,
    pub policy_name: String,
    pub total_operations: i64,
    pub failed_operations: i64,
    pub failure_rate: f64,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_error_message: Option<String>,
}

/// Recent failures for alerting
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct RecentFailure {
    pub id: Uuid,
    pub policy_id: Option<Uuid>,
    pub policy_name: Option<String>,
    pub user_id: Uuid,
    pub username: Option<String>,
    pub target_type: String,
    pub target_id: Uuid,
    pub action: String,
    pub error_message: String,
    pub created_at: DateTime<Utc>,
}

/// List audit logs with filtering
async fn list_audit_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Json<Vec<AuditLogEntry>>> {
    require_admin(&auth)?;
    let offset = query.page * query.per_page;

    let mut sql = String::from(
        r#"
        SELECT 
            a.id,
            a.policy_id,
            p.name as policy_name,
            a.run_id,
            a.user_id,
            u.username,
            a.target_type,
            a.target_id,
            CASE 
                WHEN a.target_type = 'team' THEN t.display_name
                WHEN a.target_type = 'channel' THEN c.display_name
                ELSE NULL
            END as target_name,
            a.action,
            a.status,
            a.error_message,
            a.created_at
        FROM auto_membership_policy_audit a
        LEFT JOIN auto_membership_policies p ON a.policy_id = p.id
        LEFT JOIN users u ON a.user_id = u.id
        LEFT JOIN teams t ON a.target_type = 'team' AND a.target_id = t.id
        LEFT JOIN channels c ON a.target_type = 'channel' AND a.target_id = c.id
        WHERE 1=1
        "#,
    );

    if query.policy_id.is_some() {
        sql.push_str(" AND a.policy_id = $1");
    }
    if query.user_id.is_some() {
        sql.push_str(&format!(
            " AND a.user_id = ${}",
            if query.policy_id.is_some() { 2 } else { 1 }
        ));
    }
    if query.status.is_some() {
        let idx = 1 + query.policy_id.is_some() as i32 + query.user_id.is_some() as i32;
        sql.push_str(&format!(" AND a.status = ${}", idx));
    }
    if query.action.is_some() {
        let idx = 1
            + query.policy_id.is_some() as i32
            + query.user_id.is_some() as i32
            + query.status.is_some() as i32;
        sql.push_str(&format!(" AND a.action = ${}", idx));
    }
    if query.from_date.is_some() {
        let idx = 1
            + query.policy_id.is_some() as i32
            + query.user_id.is_some() as i32
            + query.status.is_some() as i32
            + query.action.is_some() as i32;
        sql.push_str(&format!(" AND a.created_at >= ${}", idx));
    }
    if query.to_date.is_some() {
        let idx = 1
            + query.policy_id.is_some() as i32
            + query.user_id.is_some() as i32
            + query.status.is_some() as i32
            + query.action.is_some() as i32
            + query.from_date.is_some() as i32;
        sql.push_str(&format!(" AND a.created_at <= ${}", idx));
    }

    sql.push_str(" ORDER BY a.created_at DESC LIMIT $N OFFSET $M");

    // Replace placeholders with actual parameter numbers
    let param_count = 1
        + query.policy_id.is_some() as i32
        + query.user_id.is_some() as i32
        + query.status.is_some() as i32
        + query.action.is_some() as i32
        + query.from_date.is_some() as i32
        + query.to_date.is_some() as i32;

    sql = sql.replace("$N", &format!("${}", param_count));
    sql = sql.replace("$M", &format!("${}", param_count + 1));

    let mut q = sqlx::query_as(&sql);

    if let Some(policy_id) = query.policy_id {
        q = q.bind(policy_id);
    }
    if let Some(user_id) = query.user_id {
        q = q.bind(user_id);
    }
    if let Some(status) = query.status {
        q = q.bind(status);
    }
    if let Some(action) = query.action {
        q = q.bind(action);
    }
    if let Some(from_date) = query.from_date {
        q = q.bind(from_date);
    }
    if let Some(to_date) = query.to_date {
        q = q.bind(to_date);
    }
    q = q.bind(query.per_page);
    q = q.bind(offset);

    let logs: Vec<AuditLogEntry> = q.fetch_all(&state.db).await?;
    Ok(Json(logs))
}

/// Get audit summary statistics
async fn get_audit_summary(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<AuditSummary>> {
    require_admin(&auth)?;
    let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT 
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'success') as successful,
            COUNT(*) FILTER (WHERE status = 'failed') as failed,
            COUNT(*) FILTER (WHERE status = 'pending') as pending,
            COUNT(DISTINCT policy_id) FILTER (WHERE status = 'failed') as policies_with_failures
        FROM auto_membership_policy_audit
        WHERE created_at >= NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let total = row.0;
    let successful = row.1;
    let failed = row.2;
    let pending = row.3;
    let policies_with_failures = row.4;

    let failure_rate = if total > 0 {
        (failed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(AuditSummary {
        total_operations_24h: total,
        successful_operations_24h: successful,
        failed_operations_24h: failed,
        failure_rate_24h: failure_rate,
        pending_operations: pending,
        policies_with_failures,
    }))
}

/// Get failure statistics per policy
async fn get_policy_failure_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<PolicyFailureStats>>> {
    require_admin(&auth)?;
    let stats: Vec<PolicyFailureStats> = sqlx::query_as(
        r#"
        SELECT 
            p.id as policy_id,
            p.name as policy_name,
            COUNT(*) as total_operations,
            COUNT(*) FILTER (WHERE a.status = 'failed') as failed_operations,
            CASE 
                WHEN COUNT(*) > 0 THEN 
                    (COUNT(*) FILTER (WHERE a.status = 'failed')::float / COUNT(*)::float) * 100
                ELSE 0
            END as failure_rate,
            MAX(a.created_at) FILTER (WHERE a.status = 'failed') as last_failure_at,
            (SELECT error_message 
             FROM auto_membership_policy_audit a2 
             WHERE a2.policy_id = p.id AND a2.status = 'failed' 
             ORDER BY a2.created_at DESC LIMIT 1) as last_error_message
        FROM auto_membership_policies p
        LEFT JOIN auto_membership_policy_audit a ON p.id = a.policy_id
            AND a.created_at >= NOW() - INTERVAL '24 hours'
        WHERE p.enabled = true
        GROUP BY p.id, p.name
        HAVING COUNT(*) FILTER (WHERE a.status = 'failed') > 0
        ORDER BY failed_operations DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(stats))
}

/// Get recent failures for alerting
async fn get_recent_failures(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<RecentFailure>>> {
    require_admin(&auth)?;
    let failures: Vec<RecentFailure> = sqlx::query_as(
        r#"
        SELECT 
            a.id,
            a.policy_id,
            p.name as policy_name,
            a.user_id,
            u.username,
            a.target_type,
            a.target_id,
            a.action,
            a.error_message,
            a.created_at
        FROM auto_membership_policy_audit a
        LEFT JOIN auto_membership_policies p ON a.policy_id = p.id
        LEFT JOIN users u ON a.user_id = u.id
        WHERE a.status = 'failed'
        AND a.created_at >= NOW() - INTERVAL '1 hour'
        ORDER BY a.created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(failures))
}

/// Export audit logs (filtered)
async fn export_audit_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Json<Vec<AuditLogEntry>>> {
    require_admin(&auth)?;
    // Similar to list but with higher limit for export
    let mut export_query = query;
    export_query.per_page = 10000; // Max export size
    export_query.page = 0;

    list_audit_logs(State(state), auth, Query(export_query)).await
}

/// Create router for audit endpoints
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/audit/membership", get(list_audit_logs))
        .route("/admin/audit/membership/summary", get(get_audit_summary))
        .route(
            "/admin/audit/membership/failures",
            get(get_policy_failure_stats),
        )
        .route(
            "/admin/audit/membership/recent-failures",
            get(get_recent_failures),
        )
        .route("/admin/audit/membership/export", get(export_audit_logs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_log_query_parses_wire_format_sent_by_the_admin_ui() {
        // The exact query string the frontend's shared param builder
        // produces (empty filters omitted, RFC3339 dates, colons
        // percent-encoded by the serializer). Regression lock for the
        // export 400: the raw filter object sent
        // `policy_id=&from_date=2026-09-21`, which fails
        // Option<Uuid>/Option<DateTime<Utc>> deserialization.
        let query: AuditLogQuery = serde_urlencoded::from_str(
            "status=failed&action=add&from_date=2026-09-21T00%3A00%3A00.000Z&to_date=2026-09-28T00%3A00%3A00.000Z",
        )
        .expect("wire format must deserialize");
        assert_eq!(query.status.as_deref(), Some("failed"));
        assert_eq!(query.action.as_deref(), Some("add"));
        assert!(query.from_date.is_some());
        assert!(query.to_date.is_some());
        assert!(query.policy_id.is_none());

        // The broken form the export previously sent must keep failing —
        // this documents the contract the frontend must not regress to
        // (empty string is not a valid Option<Uuid>, and a bare
        // yyyy-MM-dd date is not a valid DateTime<Utc>).
        assert!(
            serde_urlencoded::from_str::<AuditLogQuery>("policy_id=&from_date=2026-09-21").is_err()
        );
    }
}
