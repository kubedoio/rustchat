//! Database module for rustchat
//!
//! Provides PostgreSQL connection pool and migration runner.

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::info;

use crate::config::DbPoolConfig;

/// Create a database connection pool and run migrations
pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    // Use default config if none provided
    let config = DbPoolConfig::default();
    connect_with_config(database_url, &config).await
}

/// Create a database connection pool with explicit configuration
pub async fn connect_with_config(
    database_url: &str,
    config: &DbPoolConfig,
) -> anyhow::Result<PgPool> {
    info!(
        max_connections = config.max_connections,
        min_connections = config.min_connections,
        acquire_timeout = config.acquire_timeout_secs,
        "Connecting to database..."
    );

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
        .connect(database_url)
        .await?;

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Check database connectivity
pub async fn health_check(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}

#[cfg(test)]
mod tests {
    // Integration tests would go here with a test database
}
