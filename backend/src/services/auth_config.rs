//! Service for handling authentication configuration and password validation

use sqlx::PgPool;
use crate::error::AppError;
use crate::models::AuthConfig;

/// Get the current password policy from server configuration
pub async fn get_password_rules(db: &PgPool) -> Result<AuthConfig, AppError> {
    let config: (sqlx::types::Json<AuthConfig>,) = sqlx::query_as(
        "SELECT authentication FROM server_config WHERE id = 'default'"
    )
    .fetch_one(db)
    .await?;

    Ok(config.0.0)
}

/// Validate a password against the provided configuration
pub fn validate_password(password: &str, config: &AuthConfig) -> Result<(), AppError> {
    if password.len() < config.password_min_length as usize {
        return Err(AppError::Validation(format!(
            "Password must be at least {} characters long",
            config.password_min_length
        )));
    }

    if config.password_require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        return Err(AppError::Validation(
            "Password must contain at least one uppercase letter".to_string(),
        ));
    }

    if config.password_require_number && !password.chars().any(|c| c.is_numeric()) {
        return Err(AppError::Validation(
            "Password must contain at least one number".to_string(),
        ));
    }

    if config.password_require_symbol && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(AppError::Validation(
            "Password must contain at least one symbol".to_string(),
        ));
    }

    Ok(())
}
