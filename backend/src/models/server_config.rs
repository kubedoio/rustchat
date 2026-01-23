//! Server configuration model and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Full server configuration entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerConfig {
    pub id: String,
    pub site: sqlx::types::Json<SiteConfig>,
    pub authentication: sqlx::types::Json<AuthConfig>,
    pub integrations: sqlx::types::Json<IntegrationsConfig>,
    pub compliance: sqlx::types::Json<ComplianceConfig>,
    pub email: sqlx::types::Json<EmailConfig>,
    pub experimental: sqlx::types::Json<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

/// Site configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SiteConfig {
    #[serde(default = "default_site_name")]
    pub site_name: String,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub site_description: String,
    #[serde(default)]
    pub site_url: String,
    #[serde(default = "default_max_file_size")]
    pub max_file_size_mb: i32,
    #[serde(default = "default_locale")]
    pub default_locale: String,
    #[serde(default = "default_timezone")]
    pub default_timezone: String,
}

fn default_site_name() -> String {
    "RustChat".to_string()
}
fn default_max_file_size() -> i32 {
    50
}
fn default_locale() -> String {
    "en".to_string()
}
fn default_timezone() -> String {
    "UTC".to_string()
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_true")]
    pub enable_email_password: bool,
    #[serde(default)]
    pub enable_sso: bool,
    #[serde(default)]
    pub require_sso: bool,
    #[serde(default = "default_true")]
    pub allow_registration: bool,
    #[serde(default = "default_password_min_length")]
    pub password_min_length: i32,
    #[serde(default = "default_true")]
    pub password_require_uppercase: bool,
    #[serde(default = "default_true")]
    pub password_require_number: bool,
    #[serde(default)]
    pub password_require_symbol: bool,
    #[serde(default = "default_session_length")]
    pub session_length_hours: i32,
}

fn default_true() -> bool {
    true
}
fn default_password_min_length() -> i32 {
    8
}
fn default_session_length() -> i32 {
    24
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enable_email_password: true,
            enable_sso: false,
            require_sso: false,
            allow_registration: true,
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_symbol: false,
            session_length_hours: 24,
        }
    }
}

/// Integrations configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationsConfig {
    #[serde(default = "default_true")]
    pub enable_webhooks: bool,
    #[serde(default = "default_true")]
    pub enable_slash_commands: bool,
    #[serde(default = "default_true")]
    pub enable_bots: bool,
    #[serde(default = "default_max_webhooks")]
    pub max_webhooks_per_team: i32,
    #[serde(default = "default_webhook_payload")]
    pub webhook_payload_size_kb: i32,
}

fn default_max_webhooks() -> i32 {
    10
}
fn default_webhook_payload() -> i32 {
    100
}

impl Default for IntegrationsConfig {
    fn default() -> Self {
        Self {
            enable_webhooks: true,
            enable_slash_commands: true,
            enable_bots: true,
            max_webhooks_per_team: 10,
            webhook_payload_size_kb: 100,
        }
    }
}

/// Compliance configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplianceConfig {
    #[serde(default)]
    pub message_retention_days: i32,
    #[serde(default)]
    pub file_retention_days: i32,
}

/// Email/SMTP configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailConfig {
    #[serde(default)]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: i32,
    #[serde(default)]
    pub smtp_username: String,
    #[serde(default)]
    pub smtp_password_encrypted: String,
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
    #[serde(default)]
    pub from_address: String,
    #[serde(default = "default_site_name")]
    pub from_name: String,
}

fn default_smtp_port() -> i32 {
    587
}

/// DTO for updating a specific config category
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ConfigUpdate {
    Site(SiteConfig),
    Auth(AuthConfig),
    Integrations(IntegrationsConfig),
    Compliance(ComplianceConfig),
    Email(EmailConfig),
    Experimental(serde_json::Value),
}

/// Response structure matching frontend expectations
#[derive(Debug, Clone, Serialize)]
pub struct ServerConfigResponse {
    pub site: SiteConfig,
    pub authentication: AuthConfig,
    pub integrations: IntegrationsConfig,
    pub compliance: ComplianceConfig,
    pub email: EmailConfig,
    pub experimental: serde_json::Value,
}

impl From<ServerConfig> for ServerConfigResponse {
    fn from(config: ServerConfig) -> Self {
        Self {
            site: config.site.0,
            authentication: config.authentication.0,
            integrations: config.integrations.0,
            compliance: config.compliance.0,
            email: config.email.0,
            experimental: config.experimental.0,
        }
    }
}
