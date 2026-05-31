//! Configuration module for rustchat
//!
//! Supports loading configuration from environment variables and .env files.

use anyhow::anyhow;
use serde::Deserialize;

pub mod security;

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Runtime environment (development, staging, production)
    #[serde(default = "default_environment")]
    pub environment: String,

    /// Server host address
    #[serde(default = "default_host")]
    pub server_host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub server_port: u16,

    /// PostgreSQL database URL
    pub database_url: String,

    /// Database connection pool configuration
    #[serde(default)]
    pub db_pool: DbPoolConfig,

    /// Redis connection URL
    #[serde(default = "default_redis_url")]
    pub redis_url: String,

    /// Require cluster websocket fan-out at startup.
    /// If true, startup fails when cluster pub/sub cannot be initialized.
    #[serde(default = "default_require_cluster_fanout")]
    pub require_cluster_fanout: bool,

    /// JWT secret key
    pub jwt_secret: String,

    /// Optional JWT issuer claim (`iss`) to embed and validate.
    #[serde(default)]
    pub jwt_issuer: Option<String>,

    /// Optional JWT audience claim (`aud`) to embed and validate.
    #[serde(default)]
    pub jwt_audience: Option<String>,

    /// Encryption key for sensitive data
    pub encryption_key: String,

    /// JWT token expiry in hours
    #[serde(default = "default_jwt_expiry")]
    pub jwt_expiry_hours: u64,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// S3 endpoint URL
    #[serde(default)]
    pub s3_endpoint: Option<String>,

    /// Public S3 endpoint URL (for presigned URLs returned to clients)
    #[serde(default)]
    pub s3_public_endpoint: Option<String>,

    /// S3 bucket name
    #[serde(default = "default_s3_bucket")]
    pub s3_bucket: String,

    /// S3 access key
    #[serde(default)]
    pub s3_access_key: Option<String>,

    /// S3 secret key
    #[serde(default)]
    pub s3_secret_key: Option<String>,

    /// S3 region
    #[serde(default = "default_s3_region")]
    pub s3_region: String,

    /// Initial admin email
    #[serde(default)]
    pub admin_user: Option<String>,

    /// Initial admin password
    #[serde(default)]
    pub admin_password: Option<String>,

    /// Comma-separated CORS origin allowlist.
    /// Example: "https://chat.example.com,https://admin.example.com"
    #[serde(default)]
    pub cors_allowed_origins: Option<String>,

    /// Cloudflare Turnstile configuration
    #[serde(default)]
    pub turnstile: TurnstileConfig,

    /// Calls plugin configuration
    #[serde(default)]
    pub calls: CallsConfig,

    /// Security policy configuration
    #[serde(default)]
    pub security: SecurityConfig,

    /// Unread/read parity behavior configuration
    #[serde(default)]
    pub unread: UnreadConfig,

    /// Keycloak group sync configuration
    #[serde(default)]
    pub keycloak_sync: KeycloakSyncConfig,

    /// Messaging policy configuration
    #[serde(default)]
    pub messaging: MessagingConfig,

    /// Compatibility-oriented feature flags.
    #[serde(default)]
    pub compatibility: CompatibilityConfig,
}

/// Calls plugin configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CallsConfig {
    /// Enable Calls plugin
    #[serde(default = "default_calls_enabled")]
    pub enabled: bool,

    /// RTC UDP port for WebRTC
    #[serde(default = "default_calls_udp_port")]
    pub udp_port: u16,

    /// RTC TCP port for WebRTC (if needed for firewall traversal)
    #[serde(default = "default_calls_tcp_port")]
    pub tcp_port: u16,

    /// ICE host override (public IP or hostname for NAT)
    #[serde(default)]
    pub ice_host_override: Option<String>,

    /// TURN server enabled (from TURN_SERVER_ENABLED env var)
    #[serde(default = "default_turn_server_enabled")]
    pub turn_server_enabled: bool,

    /// TURN server URL (from TURN_SERVER_URL env var)
    #[serde(default = "default_turn_server_url")]
    pub turn_server_url: String,

    /// TURN server username (from TURN_SERVER_USERNAME env var)
    #[serde(default)]
    pub turn_server_username: String,

    /// TURN server credential (from TURN_SERVER_CREDENTIAL env var)
    #[serde(default)]
    pub turn_server_credential: String,

    /// TURN credentials TTL in minutes
    #[serde(default = "default_turn_ttl")]
    pub turn_ttl_minutes: u64,

    /// TURN static auth secret (for REST API style ephemeral credentials)
    #[serde(default)]
    pub turn_static_auth_secret: String,

    /// STUN server URLs
    #[serde(default = "default_stun_servers")]
    pub stun_servers: Vec<String>,

    /// Call state backend mode: memory, redis, auto
    #[serde(default = "default_calls_state_backend")]
    pub state_backend: String,
}

impl Default for CallsConfig {
    fn default() -> Self {
        Self {
            enabled: default_calls_enabled(),
            udp_port: default_calls_udp_port(),
            tcp_port: default_calls_tcp_port(),
            ice_host_override: None,
            turn_server_enabled: default_turn_server_enabled(),
            turn_server_url: default_turn_server_url(),
            turn_server_username: String::new(),
            turn_server_credential: String::new(),
            turn_ttl_minutes: default_turn_ttl(),
            turn_static_auth_secret: String::new(),
            stun_servers: default_stun_servers(),
            state_backend: default_calls_state_backend(),
        }
    }
}

/// Unread/read parity configuration
#[derive(Debug, Clone, Deserialize)]
pub struct UnreadConfig {
    /// Enable unread cache v2 code paths.
    #[serde(default = "default_unread_v2_enabled")]
    pub unread_v2_enabled: bool,

    /// Enable websocket `post_unread` fan-out.
    #[serde(default = "default_post_unread_ws_enabled")]
    pub post_unread_ws_enabled: bool,

    /// Enable team unread v2 aggregation path.
    #[serde(default = "default_team_unread_v2_enabled")]
    pub team_unread_v2_enabled: bool,

    /// Enable collapsed threads semantics for unread behavior.
    #[serde(default = "default_collapsed_threads_enabled")]
    pub collapsed_threads_enabled: bool,

    /// Enable post priority/urgent mention counting.
    #[serde(default = "default_post_priority_enabled")]
    pub post_priority_enabled: bool,

    /// Auto-follow thread when marking reply unread.
    #[serde(default = "default_thread_auto_follow")]
    pub thread_auto_follow: bool,
}

impl Default for UnreadConfig {
    fn default() -> Self {
        Self {
            unread_v2_enabled: default_unread_v2_enabled(),
            post_unread_ws_enabled: default_post_unread_ws_enabled(),
            team_unread_v2_enabled: default_team_unread_v2_enabled(),
            collapsed_threads_enabled: default_collapsed_threads_enabled(),
            post_priority_enabled: default_post_priority_enabled(),
            thread_auto_follow: default_thread_auto_follow(),
        }
    }
}

/// Keycloak synchronization configuration
#[derive(Debug, Clone, Deserialize)]
pub struct KeycloakSyncConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_keycloak_provider_key")]
    pub provider_key: String,
    #[serde(default)]
    pub admin_base_url: String,
    #[serde(default)]
    pub realm: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default = "default_keycloak_interval_seconds")]
    pub interval_seconds: u64,
    #[serde(default = "default_keycloak_mapping_mode")]
    pub mapping_mode: String,
}

impl Default for KeycloakSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider_key: default_keycloak_provider_key(),
            admin_base_url: String::new(),
            realm: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            interval_seconds: default_keycloak_interval_seconds(),
            mapping_mode: default_keycloak_mapping_mode(),
        }
    }
}

/// Messaging policy configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MessagingConfig {
    #[serde(default)]
    pub dm_acl_enabled: bool,
    #[serde(default = "default_status_expiry_poll_interval_seconds")]
    pub status_expiry_poll_interval_seconds: u64,
    #[serde(default = "default_max_message_length")]
    pub max_message_length: usize,
    #[serde(default = "default_max_file_count")]
    pub max_file_count: usize,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            dm_acl_enabled: false,
            status_expiry_poll_interval_seconds: default_status_expiry_poll_interval_seconds(),
            max_message_length: default_max_message_length(),
            max_file_count: default_max_file_count(),
        }
    }
}

/// Compatibility feature flags.
#[derive(Debug, Clone, Deserialize)]
pub struct CompatibilityConfig {
    /// Mirrors FeatureFlagMobileSSOCodeExchange behavior.
    #[serde(default = "default_compat_mobile_sso_code_exchange")]
    pub mobile_sso_code_exchange: bool,
}

impl Default for CompatibilityConfig {
    fn default() -> Self {
        Self {
            mobile_sso_code_exchange: default_compat_mobile_sso_code_exchange(),
        }
    }
}

fn default_compat_mobile_sso_code_exchange() -> bool {
    true
}

fn default_max_message_length() -> usize {
    4000
}

fn default_max_file_count() -> usize {
    10
}

fn default_calls_enabled() -> bool {
    false // Disabled by default
}

fn default_calls_udp_port() -> u16 {
    8443
}

fn default_calls_tcp_port() -> u16 {
    8443
}

fn default_turn_server_enabled() -> bool {
    false // Disabled by default; operators must explicitly enable and configure their own TURN server
}

fn default_turn_server_url() -> String {
    String::new() // Empty by default; set to your own TURN server when enabled
}

fn default_turn_ttl() -> u64 {
    1440 // 24 hours
}

fn default_stun_servers() -> Vec<String> {
    vec![
        "stun:stun.l.google.com:19302".to_string(),
        "stun:stun1.l.google.com:19302".to_string(),
        "stun:stun2.l.google.com:19302".to_string(),
        "stun:stun.services.mozilla.com".to_string(),
    ]
}

fn default_calls_state_backend() -> String {
    "auto".to_string()
}

fn default_unread_v2_enabled() -> bool {
    false
}

fn default_post_unread_ws_enabled() -> bool {
    true
}

fn default_team_unread_v2_enabled() -> bool {
    true
}

fn default_collapsed_threads_enabled() -> bool {
    true
}

fn default_post_priority_enabled() -> bool {
    false
}

fn default_thread_auto_follow() -> bool {
    true
}

fn default_keycloak_provider_key() -> String {
    "oidc-main".to_string()
}

fn default_keycloak_interval_seconds() -> u64 {
    300
}

fn default_keycloak_mapping_mode() -> String {
    "attributes_only".to_string()
}

fn default_status_expiry_poll_interval_seconds() -> u64 {
    5
}

/// Database connection pool configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DbPoolConfig {
    /// Maximum number of connections in the pool
    #[serde(default = "default_db_pool_max_connections")]
    pub max_connections: u32,

    /// Minimum number of connections to maintain
    #[serde(default = "default_db_pool_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_db_pool_acquire_timeout")]
    pub acquire_timeout_secs: u64,

    /// Idle connection timeout in seconds
    #[serde(default = "default_db_pool_idle_timeout")]
    pub idle_timeout_secs: u64,

    /// Max connection lifetime in seconds
    #[serde(default = "default_db_pool_max_lifetime")]
    pub max_lifetime_secs: u64,
}

impl Default for DbPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_db_pool_max_connections(),
            min_connections: default_db_pool_min_connections(),
            acquire_timeout_secs: default_db_pool_acquire_timeout(),
            idle_timeout_secs: default_db_pool_idle_timeout(),
            max_lifetime_secs: default_db_pool_max_lifetime(),
        }
    }
}

fn default_db_pool_max_connections() -> u32 {
    // Default: 20 connections (increased from conservative defaults)
    // Adjust based on your database capacity and load
    std::env::var("DB_POOL_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
}

fn default_db_pool_min_connections() -> u32 {
    // Default: 5 connections maintained
    std::env::var("DB_POOL_MIN_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
}

fn default_db_pool_acquire_timeout() -> u64 {
    // Default: 3 seconds to acquire a connection
    std::env::var("DB_POOL_ACQUIRE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3)
}

fn default_db_pool_idle_timeout() -> u64 {
    // Default: 10 minutes
    std::env::var("DB_POOL_IDLE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600)
}

fn default_db_pool_max_lifetime() -> u64 {
    // Default: 30 minutes
    std::env::var("DB_POOL_MAX_LIFETIME")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800)
}

/// Security policy configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    /// OAuth token delivery method. Only `cookie` is supported.
    #[serde(default = "default_oauth_token_delivery")]
    pub oauth_token_delivery: String,

    /// Enable global rate limiting for auth endpoints
    #[serde(default = "default_rate_limit_enabled")]
    pub rate_limit_enabled: bool,

    /// Rate limit: requests per minute per IP for auth endpoints
    #[serde(default = "default_rate_limit_auth_per_minute")]
    pub rate_limit_auth_per_minute: u32,

    /// Rate limit: WebSocket connection attempts per minute per IP
    #[serde(default = "default_rate_limit_ws_per_minute")]
    pub rate_limit_ws_per_minute: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            oauth_token_delivery: default_oauth_token_delivery(),
            rate_limit_enabled: default_rate_limit_enabled(),
            rate_limit_auth_per_minute: default_rate_limit_auth_per_minute(),
            rate_limit_ws_per_minute: default_rate_limit_ws_per_minute(),
        }
    }
}

fn default_oauth_token_delivery() -> String {
    // Secure-by-default: one-time code exchange flow.
    "cookie".to_string()
}

fn default_rate_limit_enabled() -> bool {
    true
}

fn default_rate_limit_auth_per_minute() -> u32 {
    10
}

fn default_rate_limit_ws_per_minute() -> u32 {
    30
}

/// Cloudflare Turnstile configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TurnstileConfig {
    /// Enable Turnstile protection
    #[serde(default = "default_turnstile_enabled")]
    pub enabled: bool,
    /// Site key for frontend (public)
    #[serde(default)]
    pub site_key: String,
    /// Secret key for backend verification
    #[serde(default)]
    pub secret_key: String,
}

impl Default for TurnstileConfig {
    fn default() -> Self {
        Self {
            enabled: default_turnstile_enabled(),
            site_key: String::new(),
            secret_key: String::new(),
        }
    }
}

fn default_turnstile_enabled() -> bool {
    false // Disabled by default
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_require_cluster_fanout() -> bool {
    false
}

fn default_jwt_expiry() -> u64 {
    24
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_s3_bucket() -> String {
    "rustchat".to_string()
}

fn default_s3_region() -> String {
    "us-east-1".to_string()
}

impl Config {
    fn apply_calls_env_overrides(&mut self) -> anyhow::Result<()> {
        // Primary calls env vars used by local docker-compose.
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_ENABLED") {
            self.calls.enabled = parse_bool_env("RUSTCHAT_CALLS_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_UDP_PORT") {
            self.calls.udp_port = parse_u16_env("RUSTCHAT_CALLS_UDP_PORT", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_TCP_PORT") {
            self.calls.tcp_port = parse_u16_env("RUSTCHAT_CALLS_TCP_PORT", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_ICE_HOST_OVERRIDE") {
            let trimmed = raw.trim();
            self.calls.ice_host_override = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_STATE_BACKEND") {
            self.calls.state_backend = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_STUN_SERVERS") {
            self.calls.stun_servers = parse_csv_list(&raw);
        }

        // Mattermost-compatible TURN env vars used by deployments.
        if let Ok(raw) = std::env::var("TURN_SERVER_ENABLED") {
            self.calls.turn_server_enabled = parse_bool_env("TURN_SERVER_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("TURN_SERVER_URL") {
            self.calls.turn_server_url = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("TURN_SERVER_USERNAME") {
            self.calls.turn_server_username = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("TURN_SERVER_CREDENTIAL") {
            self.calls.turn_server_credential = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("TURN_SERVER_STATIC_AUTH_SECRET") {
            self.calls.turn_static_auth_secret = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("TURN_SERVER_TTL_MINUTES") {
            self.calls.turn_ttl_minutes = parse_u64_env("TURN_SERVER_TTL_MINUTES", &raw)?;
        }

        // Explicit RUSTCHAT_CALLS_* TURN vars, when present, take precedence.
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_TURN_SERVER_ENABLED") {
            self.calls.turn_server_enabled =
                parse_bool_env("RUSTCHAT_CALLS_TURN_SERVER_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_TURN_SERVER_URL") {
            self.calls.turn_server_url = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_TURN_SERVER_USERNAME") {
            self.calls.turn_server_username = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_TURN_SERVER_CREDENTIAL") {
            self.calls.turn_server_credential = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_TURN_STATIC_AUTH_SECRET") {
            self.calls.turn_static_auth_secret = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_CALLS_TURN_TTL_MINUTES") {
            self.calls.turn_ttl_minutes = parse_u64_env("RUSTCHAT_CALLS_TURN_TTL_MINUTES", &raw)?;
        }

        Ok(())
    }

    fn apply_unread_env_overrides(&mut self) -> anyhow::Result<()> {
        if let Ok(raw) = std::env::var("RUSTCHAT_UNREAD_V2_ENABLED") {
            self.unread.unread_v2_enabled = parse_bool_env("RUSTCHAT_UNREAD_V2_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_POST_UNREAD_WS_ENABLED") {
            self.unread.post_unread_ws_enabled =
                parse_bool_env("RUSTCHAT_POST_UNREAD_WS_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_TEAM_UNREAD_V2_ENABLED") {
            self.unread.team_unread_v2_enabled =
                parse_bool_env("RUSTCHAT_TEAM_UNREAD_V2_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_COLLAPSED_THREADS_ENABLED") {
            self.unread.collapsed_threads_enabled =
                parse_bool_env("RUSTCHAT_COLLAPSED_THREADS_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_POST_PRIORITY_ENABLED") {
            self.unread.post_priority_enabled =
                parse_bool_env("RUSTCHAT_POST_PRIORITY_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_THREAD_AUTO_FOLLOW") {
            self.unread.thread_auto_follow = parse_bool_env("RUSTCHAT_THREAD_AUTO_FOLLOW", &raw)?;
        }
        Ok(())
    }

    fn apply_keycloak_env_overrides(&mut self) -> anyhow::Result<()> {
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_ENABLED") {
            self.keycloak_sync.enabled = parse_bool_env("RUSTCHAT_KEYCLOAK_SYNC_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_PROVIDER_KEY") {
            self.keycloak_sync.provider_key = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_ADMIN_BASE_URL") {
            self.keycloak_sync.admin_base_url = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_REALM") {
            self.keycloak_sync.realm = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_CLIENT_ID") {
            self.keycloak_sync.client_id = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_CLIENT_SECRET") {
            self.keycloak_sync.client_secret = raw.trim().to_string();
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_INTERVAL_SECONDS") {
            self.keycloak_sync.interval_seconds =
                parse_u64_env("RUSTCHAT_KEYCLOAK_SYNC_INTERVAL_SECONDS", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_KEYCLOAK_SYNC_MAPPING_MODE") {
            self.keycloak_sync.mapping_mode = raw.trim().to_string();
        }
        Ok(())
    }

    fn apply_messaging_env_overrides(&mut self) -> anyhow::Result<()> {
        if let Ok(raw) = std::env::var("RUSTCHAT_MESSAGING_DM_ACL_ENABLED") {
            self.messaging.dm_acl_enabled =
                parse_bool_env("RUSTCHAT_MESSAGING_DM_ACL_ENABLED", &raw)?;
        }
        if let Ok(raw) = std::env::var("RUSTCHAT_STATUS_EXPIRY_POLL_INTERVAL_SECONDS") {
            self.messaging.status_expiry_poll_interval_seconds =
                parse_u64_env("RUSTCHAT_STATUS_EXPIRY_POLL_INTERVAL_SECONDS", &raw)?;
        }
        Ok(())
    }

    fn apply_compatibility_env_overrides(&mut self) -> anyhow::Result<()> {
        if let Ok(raw) = std::env::var("RUSTCHAT_COMPAT_MOBILE_SSO_CODE_EXCHANGE") {
            self.compatibility.mobile_sso_code_exchange =
                parse_bool_env("RUSTCHAT_COMPAT_MOBILE_SSO_CODE_EXCHANGE", &raw)?;
        }
        Ok(())
    }

    /// Load configuration from environment variables
    pub fn load() -> anyhow::Result<Self> {
        let mut builder = config::Config::builder();

        // Load RUSTCHAT_ prefixed variables
        builder = builder.add_source(
            config::Environment::default()
                .prefix("RUSTCHAT")
                .try_parsing(true),
        );

        // Load TURN_SERVER_ prefixed variables (for backwards compatibility with Mattermost-style env vars)
        builder = builder.add_source(
            config::Environment::default()
                .prefix("TURN_SERVER")
                .try_parsing(true)
                .separator("_"),
        );

        let config = builder.build()?;
        let mut settings: Config = config.try_deserialize()?;
        settings.apply_calls_env_overrides()?;
        settings.apply_unread_env_overrides()?;
        settings.apply_keycloak_env_overrides()?;
        settings.apply_messaging_env_overrides()?;
        settings.apply_compatibility_env_overrides()?;

        // Validate security settings
        settings.validate_security()?;

        Ok(settings)
    }

    /// Validate security-critical configuration
    fn validate_security(&self) -> anyhow::Result<()> {
        let validation = security::validate_secrets(self);

        // Log all warnings
        for warning in &validation.warnings {
            tracing::warn!("Security configuration warning: {}", warning);
        }

        if self.is_production() {
            // In production, fail fast on security issues
            if !validation.is_valid {
                for error in &validation.errors {
                    tracing::error!("Security configuration error: {}", error);
                }
                anyhow::bail!(
                    "Security validation failed with {} error(s). Fix the issues above before starting in production mode.",
                    validation.errors.len()
                );
            }
        } else {
            // In development, log errors but continue
            for error in &validation.errors {
                tracing::warn!("Security configuration issue (allowed in dev): {}", error);
            }
        }

        let oauth_delivery = self
            .security
            .oauth_token_delivery
            .trim()
            .to_ascii_lowercase();
        if oauth_delivery != "cookie" {
            anyhow::bail!(
                "Invalid RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY value '{}'. Query-token delivery has been removed; expected 'cookie'.",
                self.security.oauth_token_delivery
            );
        }

        if let Ok(raw) = std::env::var("RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN") {
            let enabled = parse_bool_env("RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN", &raw)?;
            if enabled {
                anyhow::bail!(
                    "RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN=true is no longer supported. WebSocket query-token authentication has been removed."
                );
            } else {
                tracing::warn!(
                    "RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN is deprecated and ignored. Remove it from your environment."
                );
            }
        }

        if self.is_production() {
            if self
                .jwt_issuer
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .is_none()
            {
                anyhow::bail!(
                    "RUSTCHAT_JWT_ISSUER is required in production and must be non-empty."
                );
            }
            if self
                .jwt_audience
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .is_none()
            {
                anyhow::bail!(
                    "RUSTCHAT_JWT_AUDIENCE is required in production and must be non-empty."
                );
            }

            if let Ok(site_url) = std::env::var("RUSTCHAT_SITE_URL") {
                let normalized = site_url.trim().to_ascii_lowercase();
                if !normalized.is_empty() && !normalized.starts_with("https://") {
                    anyhow::bail!(
                        "RUSTCHAT_SITE_URL must use https:// in production (current value: '{}').",
                        site_url
                    );
                }
            } else {
                anyhow::bail!("RUSTCHAT_SITE_URL is required in production and must use https://.");
            }

            if let Some(origins) = self.cors_allowed_origins.as_deref() {
                let insecure_origins: Vec<&str> = origins
                    .split(',')
                    .map(str::trim)
                    .filter(|origin| !origin.is_empty())
                    .filter(|origin| origin.to_ascii_lowercase().starts_with("http://"))
                    .collect();

                if !insecure_origins.is_empty() {
                    anyhow::bail!(
                        "In production, RUSTCHAT_CORS_ALLOWED_ORIGINS must use https:// only. Insecure origin(s): {}",
                        insecure_origins.join(", ")
                    );
                }
            }
        } else if let Ok(site_url) = std::env::var("RUSTCHAT_SITE_URL") {
            let normalized = site_url.trim().to_ascii_lowercase();
            if !normalized.is_empty() && !normalized.starts_with("https://") {
                tracing::warn!(
                    "RUSTCHAT_SITE_URL does not use https:// (allowed in non-production)"
                );
            }
        }

        Ok(())
    }

    pub fn is_production(&self) -> bool {
        matches!(
            self.environment.trim().to_ascii_lowercase().as_str(),
            "prod" | "production"
        )
    }
}

fn parse_bool_env(name: &str, raw: &str) -> anyhow::Result<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(anyhow!("invalid boolean for {}: {}", name, raw)),
    }
}

fn parse_u16_env(name: &str, raw: &str) -> anyhow::Result<u16> {
    raw.trim()
        .parse::<u16>()
        .map_err(|e| anyhow!("invalid u16 for {}: {} ({})", name, raw, e))
}

fn parse_u64_env(name: &str, raw: &str) -> anyhow::Result<u64> {
    raw.trim()
        .parse::<u64>()
        .map_err(|e| anyhow!("invalid u64 for {}: {} ({})", name, raw, e))
}

fn parse_csv_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        assert_eq!(default_host(), "0.0.0.0");
        assert_eq!(default_port(), 3000);
        assert_eq!(default_log_level(), "info");
    }
}
