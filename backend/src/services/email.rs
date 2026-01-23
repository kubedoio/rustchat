//! Email service using lettre
//!
//! Handles sending emails via SMTP based on server configuration.

use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use tracing::{error, info};

use crate::models::server_config::EmailConfig;

/// Send an email using the provided configuration
pub async fn send_email(
    config: &EmailConfig,
    to_address: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    if config.smtp_host.is_empty() {
        return Err("SMTP host not configured".to_string());
    }

    let email = Message::builder()
        .from(
            format!("{} <{}>", config.from_name, config.from_address)
                .parse()
                .map_err(|e| format!("Invalid from address: {}", e))?,
        )
        .to(to_address
            .parse()
            .map_err(|e| format!("Invalid to address: {}", e))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| format!("Failed to build email: {}", e))?;

    let creds = Credentials::new(
        config.smtp_username.clone(),
        config.smtp_password_encrypted.clone(), // In a real app, decrypt this first
    );

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .map_err(|e| format!("Failed to create transport: {}", e))?
            .credentials(creds)
            .port(config.smtp_port as u16)
            .build();

    // Use Result here to capture sending error
    match mailer.send(email).await {
        Ok(_) => {
            info!("Email sent to {}", to_address);
            Ok(())
        }
        Err(e) => {
            error!("Failed to send email to {}: {}", to_address, e);
            Err(format!("Failed to send email: {}", e))
        }
    }
}
