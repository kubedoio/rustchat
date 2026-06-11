//! Typed audit logging helper

use serde_json::Value;
use uuid::Uuid;

use crate::api::admin::insert_admin_audit_log;
use crate::error::AppError;

/// Security-sensitive and admin actions that should be recorded in the audit log.
pub enum AuditAction {
    UserSoftDelete,
    UserWipe,
    UserCreate,
    UserUpdate,
    TeamMemberAdd,
    TeamMemberRemove,
    TeamDelete,
    ChannelDelete,
    RolePermissionUpdate,
    ConfigUpdate,
    SsoConfigCreate,
    SsoConfigUpdate,
    SsoConfigDelete,
    EmailProviderCreate,
    EmailProviderUpdate,
    EmailProviderDelete,
    FileDownload,
    FileDownloadDenied,
    LoginSuccess,
    LoginFailed,
    ApiKeyCreated,
    ApiKeyRevoked,
}

impl AuditAction {
    /// Machine-readable action identifier stored in the audit_logs table.
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::UserSoftDelete => "user.soft_delete",
            AuditAction::UserWipe => "user.wipe",
            AuditAction::UserCreate => "user.create",
            AuditAction::UserUpdate => "user.update",
            AuditAction::TeamMemberAdd => "team.member_add",
            AuditAction::TeamMemberRemove => "team.member_remove",
            AuditAction::TeamDelete => "team.delete",
            AuditAction::ChannelDelete => "channel.delete",
            AuditAction::RolePermissionUpdate => "role.permission_update",
            AuditAction::ConfigUpdate => "config.update",
            AuditAction::SsoConfigCreate => "sso_config.create",
            AuditAction::SsoConfigUpdate => "sso_config.update",
            AuditAction::SsoConfigDelete => "sso_config.delete",
            AuditAction::EmailProviderCreate => "email_provider.create",
            AuditAction::EmailProviderUpdate => "email_provider.update",
            AuditAction::EmailProviderDelete => "email_provider.delete",
            AuditAction::FileDownload => "file.download",
            AuditAction::FileDownloadDenied => "file.download_denied",
            AuditAction::LoginSuccess => "auth.login_success",
            AuditAction::LoginFailed => "auth.login_failed",
            AuditAction::ApiKeyCreated => "api_key.created",
            AuditAction::ApiKeyRevoked => "api_key.revoked",
        }
    }
}

/// Insert an audit log entry.
///
/// Wraps [`insert_admin_audit_log`] with a typed action so callers don't have to
/// hard-code string constants.
pub async fn audit(
    db: &sqlx::PgPool,
    actor_user_id: Uuid,
    action: AuditAction,
    target_type: &str,
    target_id: Option<Uuid>,
    metadata: Value,
) -> Result<(), AppError> {
    insert_admin_audit_log(
        db,
        actor_user_id,
        action.as_str(),
        target_type,
        target_id,
        metadata,
    )
    .await
}
