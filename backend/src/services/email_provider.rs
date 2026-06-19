//! Email Provider Abstraction
//!
//! Provides a trait-based abstraction for different email providers (SMTP, SES, SendGrid, etc.)
//! with a concrete SMTP implementation using lettre.

use lettre::{
    message::Mailbox,
    transport::smtp::authentication::Credentials,
    transport::smtp::client::{Tls, TlsParameters},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use std::fmt;
use tracing::{debug, error, info, warn};

use crate::models::email::{MailProviderSettings, TlsMode};

/// Errors that can occur during email sending
#[derive(Debug, Clone)]
pub enum EmailProviderError {
    ConnectionError(String),
    AuthenticationError(String),
    RateLimitError(String),
    InvalidRecipient(String),
    InvalidSender(String),
    ServerError(String),
    ConfigurationError(String),
    Other(String),
}

impl fmt::Display for EmailProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmailProviderError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            EmailProviderError::AuthenticationError(msg) => {
                write!(f, "Authentication error: {}", msg)
            }
            EmailProviderError::RateLimitError(msg) => write!(f, "Rate limit: {}", msg),
            EmailProviderError::InvalidRecipient(msg) => write!(f, "Invalid recipient: {}", msg),
            EmailProviderError::InvalidSender(msg) => write!(f, "Invalid sender: {}", msg),
            EmailProviderError::ServerError(msg) => write!(f, "Server error: {}", msg),
            EmailProviderError::ConfigurationError(msg) => {
                write!(f, "Configuration error: {}", msg)
            }
            EmailProviderError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for EmailProviderError {}

/// Result type for email provider operations
pub type EmailProviderResult<T> = Result<T, EmailProviderError>;

/// Email content to be sent
#[derive(Debug, Clone)]
pub struct EmailContent {
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub headers: Vec<(String, String)>,
}

/// Email address with optional display name
#[derive(Debug, Clone)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

impl EmailAddress {
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: None,
        }
    }

    pub fn with_name(email: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: Some(name.into()),
        }
    }

    pub fn to_mailbox(&self) -> EmailProviderResult<Mailbox> {
        format!("{} <{}>", self.name.as_deref().unwrap_or(""), self.email)
            .parse()
            .map_err(|e| {
                EmailProviderError::InvalidRecipient(format!(
                    "Invalid address '{}': {}",
                    self.email, e
                ))
            })
    }
}

/// Result of a successful email send operation
#[derive(Debug, Clone)]
pub struct SendResult {
    pub message_id: Option<String>,
    pub server_response: String,
}

// ============================================
// SMTP Provider Implementation
// ============================================

/// SMTP email provider
#[derive(Clone)]
pub struct SmtpProvider {
    settings: MailProviderSettings,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpProvider {
    /// Create a new SMTP provider from settings
    pub async fn new(
        settings: MailProviderSettings,
        encryption_key: &str,
    ) -> EmailProviderResult<Self> {
        let transport = Self::build_transport(&settings, encryption_key).await?;

        Ok(Self {
            settings,
            transport,
        })
    }

    /// Build the SMTP transport from settings
    async fn build_transport(
        settings: &MailProviderSettings,
        encryption_key: &str,
    ) -> EmailProviderResult<AsyncSmtpTransport<Tokio1Executor>> {
        let host = &settings.host;
        let port = settings.port as u16;

        if host.is_empty() {
            return Err(EmailProviderError::ConfigurationError(
                "SMTP host is not configured".to_string(),
            ));
        }

        // Build TLS parameters
        let tls_params = if settings.skip_cert_verify {
            TlsParameters::builder(host.clone())
                .dangerous_accept_invalid_certs(true)
                .build()
                .map_err(|e| {
                    EmailProviderError::ConfigurationError(format!("TLS config error: {}", e))
                })?
        } else {
            TlsParameters::new(host.clone()).map_err(|e| {
                EmailProviderError::ConfigurationError(format!("TLS config error: {}", e))
            })?
        };

        // Build credentials if username is provided
        let transport = if settings.username.is_empty() {
            // No authentication
            match settings.tls_mode {
                TlsMode::ImplicitTls => {
                    info!(
                        "Building SMTP transport with Implicit TLS (SMTPS) for {}:{}",
                        host, port
                    );
                    AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                        .map_err(|e| {
                            EmailProviderError::ConfigurationError(format!(
                                "Transport error: {}",
                                e
                            ))
                        })?
                        .port(port)
                        .tls(Tls::Required(tls_params))
                        .build()
                }
                TlsMode::None => {
                    warn!(
                        "Building SMTP transport without encryption for {}:{}",
                        host, port
                    );
                    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                        .port(port)
                        .build()
                }
                TlsMode::Starttls => {
                    info!(
                        "Building SMTP transport with STARTTLS for {}:{}",
                        host, port
                    );
                    AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                        .map_err(|e| {
                            EmailProviderError::ConfigurationError(format!(
                                "Transport error: {}",
                                e
                            ))
                        })?
                        .port(port)
                        .tls(Tls::Required(tls_params))
                        .build()
                }
            }
        } else {
            // With authentication - decrypt the password first
            let password = if settings.password_encrypted.is_empty() {
                String::new()
            } else {
                crate::crypto::decrypt(&settings.password_encrypted, encryption_key).map_err(
                    |e| {
                        EmailProviderError::ConfigurationError(format!(
                            "Failed to decrypt SMTP password: {}",
                            e
                        ))
                    },
                )?
            };
            let creds = Credentials::new(settings.username.clone(), password);

            match settings.tls_mode {
                TlsMode::ImplicitTls => {
                    info!(
                        "Building authenticated SMTP transport with Implicit TLS for {}:{}",
                        host, port
                    );
                    AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                        .map_err(|e| {
                            EmailProviderError::ConfigurationError(format!(
                                "Transport error: {}",
                                e
                            ))
                        })?
                        .credentials(creds)
                        .port(port)
                        .tls(Tls::Required(tls_params))
                        .build()
                }
                TlsMode::None => {
                    warn!(
                        "Building authenticated SMTP transport without encryption for {}:{}",
                        host, port
                    );
                    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                        .port(port)
                        .credentials(creds)
                        .build()
                }
                TlsMode::Starttls => {
                    info!(
                        "Building authenticated SMTP transport with STARTTLS for {}:{}",
                        host, port
                    );
                    AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                        .map_err(|e| {
                            EmailProviderError::ConfigurationError(format!(
                                "Transport error: {}",
                                e
                            ))
                        })?
                        .credentials(creds)
                        .port(port)
                        .tls(Tls::Required(tls_params))
                        .build()
                }
            }
        };

        Ok(transport)
    }

    /// Rebuild the transport (useful after settings change)
    pub async fn rebuild_transport(&mut self, encryption_key: &str) -> EmailProviderResult<()> {
        self.transport = Self::build_transport(&self.settings, encryption_key).await?;
        Ok(())
    }

    /// Update settings and rebuild transport
    pub async fn update_settings(
        &mut self,
        settings: MailProviderSettings,
        encryption_key: &str,
    ) -> EmailProviderResult<()> {
        self.settings = settings;
        self.rebuild_transport(encryption_key).await
    }

    /// Classify an SMTP error into our error types
    fn classify_error(error: &lettre::transport::smtp::Error) -> EmailProviderError {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("authentication")
            || error_str.contains("auth")
            || error_str.contains("535")
        {
            EmailProviderError::AuthenticationError(error.to_string())
        } else if error_str.contains("certificate")
            || error_str.contains("tls")
            || error_str.contains("ssl")
        {
            EmailProviderError::ConnectionError(format!("TLS error: {}", error))
        } else if error_str.contains("dns")
            || error_str.contains("resolve")
            || error_str.contains("connection")
        {
            EmailProviderError::ConnectionError(error.to_string())
        } else if error_str.contains("timeout") || error_str.contains("timed out") {
            EmailProviderError::ConnectionError(format!("Connection timeout: {}", error))
        } else if error_str.contains("relay")
            || error_str.contains("denied")
            || error_str.contains("550")
        {
            EmailProviderError::ServerError(format!("Relay denied: {}", error))
        } else if error_str.contains("rate")
            || error_str.contains("throttle")
            || error_str.contains("421")
        {
            EmailProviderError::RateLimitError(error.to_string())
        } else {
            EmailProviderError::Other(error.to_string())
        }
    }

    /// Test the connection to the mail server
    pub async fn test_connection(&self) -> EmailProviderResult<()> {
        debug!(
            "Testing SMTP connection to {}:{}",
            self.settings.host, self.settings.port
        );

        match self.transport.test_connection().await {
            Ok(true) => {
                info!(
                    "SMTP connection test successful for {}:{}",
                    self.settings.host, self.settings.port
                );
                Ok(())
            }
            Ok(false) => {
                warn!(
                    "SMTP connection test returned false for {}:{}",
                    self.settings.host, self.settings.port
                );
                Err(EmailProviderError::ConnectionError(
                    "SMTP server connected but returned failure".to_string(),
                ))
            }
            Err(e) => {
                error!(
                    "SMTP connection test failed for {}:{}: {}",
                    self.settings.host, self.settings.port, e
                );
                Err(Self::classify_error(&e))
            }
        }
    }

    /// Send an email
    pub async fn send_email(
        &self,
        from: &EmailAddress,
        to: &EmailAddress,
        content: &EmailContent,
    ) -> EmailProviderResult<SendResult> {
        use lettre::message::header::ContentType;

        let from_mailbox = from.to_mailbox()?;
        let to_mailbox = to.to_mailbox()?;

        // Build the email message
        let builder = lettre::Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(&content.subject);

        // Set content based on what's available
        let message = if let Some(html) = &content.body_html {
            // Multi-part message
            builder
                .multipart(
                    lettre::message::MultiPart::alternative()
                        .singlepart(
                            lettre::message::SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(content.body_text.clone()),
                        )
                        .singlepart(
                            lettre::message::SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html.clone()),
                        ),
                )
                .map_err(|e| EmailProviderError::Other(format!("Failed to build message: {}", e)))?
        } else {
            // Text-only message
            builder
                .body(content.body_text.clone())
                .map_err(|e| EmailProviderError::Other(format!("Failed to build message: {}", e)))?
        };

        // Send the email
        match self.transport.send(message).await {
            Ok(response) => {
                let server_response = format!("{:?}", response);
                info!("Email sent successfully to {} via SMTP", to.email);
                Ok(SendResult {
                    message_id: None, // SMTP doesn't always provide message ID in the same way
                    server_response,
                })
            }
            Err(e) => {
                error!("Failed to send email to {}: {}", to.email, e);
                Err(Self::classify_error(&e))
            }
        }
    }

    pub fn provider_type(&self) -> &'static str {
        "smtp"
    }

    pub fn provider_id(&self) -> String {
        format!("smtp:{}:{}", self.settings.host, self.settings.port)
    }
}

// ============================================
// Provider Factory
// ============================================

/// Factory for creating mail providers
pub struct MailProviderFactory;

impl MailProviderFactory {
    /// Create a mail provider from settings
    pub async fn create(
        settings: &MailProviderSettings,
        encryption_key: &str,
    ) -> EmailProviderResult<SmtpProvider> {
        match settings.provider_type {
            crate::models::email::MailProviderType::Smtp => {
                SmtpProvider::new(settings.clone(), encryption_key).await
            }
            crate::models::email::MailProviderType::Ses => {
                // SES provider not yet implemented. Use SMTP provider with SES SMTP credentials
                // as a workaround, or contribute: https://github.com/rustchat/rustchat/issues
                Err(EmailProviderError::ConfigurationError(
                    "SES provider not yet implemented. Use SMTP provider with SES SMTP credentials as a workaround.".to_string(),
                ))
            }
            crate::models::email::MailProviderType::Sendgrid => {
                // SendGrid provider not yet implemented. Use SMTP provider with SendGrid SMTP credentials
                // as a workaround, or contribute: https://github.com/rustchat/rustchat/issues
                Err(EmailProviderError::ConfigurationError(
                    "SendGrid provider not yet implemented. Use SMTP provider with SendGrid SMTP credentials as a workaround.".to_string(),
                ))
            }
        }
    }
}

// ============================================
// Provider Pool / Manager
// ============================================

use std::collections::HashMap;
use tokio::sync::RwLock;

/// Manages mail provider instances
pub struct MailProviderManager {
    providers: RwLock<HashMap<uuid::Uuid, SmtpProvider>>,
}

impl MailProviderManager {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a provider for the given settings
    pub async fn get_or_create(
        &self,
        settings: &MailProviderSettings,
        encryption_key: &str,
    ) -> EmailProviderResult<()> {
        let mut providers = self.providers.write().await;

        // Check if we need to update an existing provider
        if let Some(existing) = providers.get_mut(&settings.id) {
            if existing.provider_id()
                != format!(
                    "{}:{}:{}",
                    settings.provider_type.as_str(),
                    settings.host,
                    settings.port
                )
            {
                // Settings changed, recreate
                let new_provider = MailProviderFactory::create(settings, encryption_key).await?;
                providers.insert(settings.id, new_provider);
            }
        } else {
            // Create new provider
            let provider = MailProviderFactory::create(settings, encryption_key).await?;
            providers.insert(settings.id, provider);
        }

        Ok(())
    }

    /// Get a provider by ID
    pub async fn get(&self, _id: uuid::Uuid) -> Option<SmtpProvider> {
        let providers = self.providers.read().await;
        providers.get(&_id).cloned()
    }

    /// Remove a provider from the cache
    pub async fn remove(&self, id: uuid::Uuid) {
        let mut providers = self.providers.write().await;
        providers.remove(&id);
    }

    /// Clear all providers
    pub async fn clear(&self) {
        let mut providers = self.providers.write().await;
        providers.clear();
    }
}

impl Default for MailProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// Helper Functions
// ============================================

/// Get the default mail provider settings from database
pub async fn get_default_provider(
    db: &sqlx::PgPool,
    tenant_id: Option<uuid::Uuid>,
) -> Result<Option<MailProviderSettings>, sqlx::Error> {
    let provider: Option<MailProviderSettings> = sqlx::query_as(
        r#"
        SELECT * FROM mail_provider_settings
        WHERE (tenant_id = $1 OR (tenant_id IS NULL AND $1 IS NULL))
          AND enabled = true
          AND is_default = true
        ORDER BY tenant_id NULLS LAST
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    Ok(provider)
}

/// Get provider by ID
pub async fn get_provider_by_id(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<MailProviderSettings>, sqlx::Error> {
    let provider: Option<MailProviderSettings> =
        sqlx::query_as("SELECT * FROM mail_provider_settings WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?;

    Ok(provider)
}

/// List all providers for a tenant
pub async fn list_providers(
    db: &sqlx::PgPool,
    tenant_id: Option<uuid::Uuid>,
) -> Result<Vec<MailProviderSettings>, sqlx::Error> {
    let providers: Vec<MailProviderSettings> = sqlx::query_as(
        r#"
        SELECT * FROM mail_provider_settings
        WHERE tenant_id = $1 OR (tenant_id IS NULL AND $1 IS NULL)
        ORDER BY is_default DESC, created_at ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(providers)
}
