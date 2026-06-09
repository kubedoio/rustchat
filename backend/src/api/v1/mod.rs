//! API v1 routes for new entity-specific endpoints
//!
//! This module contains Phase 1 entity registration and management endpoints.
//! These are separate from the main v1 routes which are historically mixed with v4.

pub mod agents;
pub mod entities;
pub mod knowledge;

use axum::Router;

use crate::api::AppState;

/// Build the API v1 entity routes
pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/entities", entities::router())
        .nest("/agents", agents::router())
        .nest("/knowledge", knowledge::router())
}
