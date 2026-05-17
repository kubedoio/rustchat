//! Administrative models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Audit log entry
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_ip: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// SSO configuration
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SsoConfig {
    pub id: Uuid,
    pub org_id: Uuid,
    /// Legacy provider field - use provider_key and provider_type instead
    pub provider: String,
    /// URL-safe unique key used in OAuth URLs (e.g., "github", "google", "oidc-main")
    pub provider_key: String,
    /// Provider type: "github", "google", "oidc", or "saml"
    pub provider_type: String,
    pub display_name: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    #[serde(skip_serializing)]
    pub client_secret_encrypted: Option<String>,
    pub scopes: Vec<String>,
    pub idp_metadata_url: Option<String>,
    pub idp_entity_id: Option<String>,
    pub is_active: bool,
    pub auto_provision: bool,
    pub default_role: Option<String>,
    /// For Google: allowed email domains
    pub allow_domains: Option<Vec<String>>,
    /// For GitHub: required organization membership
    pub github_org: Option<String>,
    /// For GitHub: required team membership (within org)
    pub github_team: Option<String>,
    /// For OIDC: claim name containing groups (e.g., "groups")
    pub groups_claim: Option<String>,
    /// For OIDC: mapping of provider groups to RustChat roles
    pub role_mappings: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SSO provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProviderType {
    GitHub,
    Google,
    Oidc,
    Saml,
}

impl SsoProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SsoProviderType::GitHub => "github",
            SsoProviderType::Google => "google",
            SsoProviderType::Oidc => "oidc",
            SsoProviderType::Saml => "saml",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "github" => Some(SsoProviderType::GitHub),
            "google" => Some(SsoProviderType::Google),
            "oidc" => Some(SsoProviderType::Oidc),
            "saml" => Some(SsoProviderType::Saml),
            _ => None,
        }
    }

    /// Default scopes for this provider type
    pub fn default_scopes(&self) -> Vec<String> {
        match self {
            SsoProviderType::GitHub => vec!["read:user".to_string(), "user:email".to_string()],
            SsoProviderType::Google | SsoProviderType::Oidc | SsoProviderType::Saml => {
                vec![
                    "openid".to_string(),
                    "profile".to_string(),
                    "email".to_string(),
                ]
            }
        }
    }

    /// Whether this provider uses OIDC discovery
    pub fn uses_oidc_discovery(&self) -> bool {
        matches!(self, SsoProviderType::Google | SsoProviderType::Oidc)
    }
}

/// SSO configuration response (without secrets)
#[derive(Debug, Clone, Serialize)]
pub struct SsoConfigResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    pub provider_key: String,
    pub provider_type: String,
    pub display_name: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub auto_provision: bool,
    pub default_role: Option<String>,
    pub allow_domains: Option<Vec<String>>,
    pub github_org: Option<String>,
    pub github_team: Option<String>,
    pub groups_claim: Option<String>,
    pub role_mappings: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SsoConfig> for SsoConfigResponse {
    fn from(config: SsoConfig) -> Self {
        Self {
            id: config.id,
            org_id: config.org_id,
            provider_key: config.provider_key,
            provider_type: config.provider_type,
            display_name: config.display_name,
            issuer_url: config.issuer_url,
            client_id: config.client_id,
            scopes: config.scopes,
            is_active: config.is_active,
            auto_provision: config.auto_provision,
            default_role: config.default_role,
            allow_domains: config.allow_domains,
            github_org: config.github_org,
            github_team: config.github_team,
            groups_claim: config.groups_claim,
            role_mappings: config.role_mappings,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

/// Retention policy
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RetentionPolicy {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub retention_days: i32,
    pub delete_files: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Permission definition
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Permission {
    pub id: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

// ============ DTOs ============

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAuditLog {
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSsoConfig {
    pub provider_key: String,
    pub provider_type: String,
    pub display_name: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub auto_provision: Option<bool>,
    pub default_role: Option<String>,
    pub allow_domains: Option<Vec<String>>,
    pub github_org: Option<String>,
    pub github_team: Option<String>,
    pub groups_claim: Option<String>,
    pub role_mappings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSsoConfig {
    pub provider_key: Option<String>,
    pub display_name: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub auto_provision: Option<bool>,
    pub default_role: Option<String>,
    pub allow_domains: Option<Vec<String>>,
    pub github_org: Option<String>,
    pub github_team: Option<String>,
    pub groups_claim: Option<String>,
    pub role_mappings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRetentionPolicy {
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub retention_days: i32,
    #[serde(default)]
    pub delete_files: bool,
}

/// Audit log query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogQuery {
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// OAuth provider info for public listing (login page)
#[derive(Debug, Clone, Serialize)]
pub struct OAuthProviderInfo {
    pub id: String,
    pub provider_key: String,
    pub provider_type: String,
    pub display_name: String,
    pub login_url: String,
}

/// Test result for SSO configuration
#[derive(Debug, Clone, Serialize)]
pub struct SsoTestResult {
    pub success: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Admin team response with aggregated counts
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AdminTeamResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub team: crate::models::Team,
    pub members_count: i64,
    pub channels_count: i64,
}

/// Admin channel response with aggregated member count
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AdminChannelResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub channel: crate::models::Channel,
    pub members_count: i64,
}
