//! Database module for rustchat
//!
//! Provides PostgreSQL connection pool and migration runner.

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::info;

/// Create a database connection pool and run migrations
pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await?;

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Check database connectivity
pub async fn health_check(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    // Integration tests would go here with a test database
}
