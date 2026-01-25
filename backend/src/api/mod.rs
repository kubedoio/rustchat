//! API module for rustchat
//!
//! Provides HTTP routes and handlers.

mod admin;
mod auth;
mod calls;
mod channels;
mod files;
mod health;
mod integrations;
mod oauth;
mod playbooks;
mod posts;
mod preferences;
mod search;
mod site;
mod teams;
mod unreads;
mod users;
mod v4;
mod video;
mod ws;

use std::sync::Arc;

use axum::{http::Method, Router};
use sqlx::PgPool;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::realtime::WsHub;
use crate::storage::S3Client;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub ws_hub: Arc<WsHub>,
    pub s3_client: S3Client,
    pub http_client: reqwest::Client,
    pub start_time: std::time::Instant,
}

/// Build the main application router
pub fn router(
    db: PgPool,
    redis: redis::Client,
    jwt_secret: String,
    jwt_expiry_hours: u64,
    ws_hub: Arc<WsHub>,
    s3_client: S3Client,
) -> Router {
    let state = AppState {
        db,
        redis,
        jwt_secret,
        jwt_expiry_hours,
        ws_hub,
        s3_client,
        http_client: reqwest::Client::new(),
        start_time: std::time::Instant::now(),
    };

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
        ])
        .allow_headers(Any);

    // API v1 routes
    let api_v1 = Router::new()
        .nest("/health", health::router())
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/teams", teams::router())
        .nest("/channels", channels::router())
        .nest("/unreads", unreads::router())
        .merge(posts::router())
        .merge(files::router())
        .merge(search::router())
        .merge(integrations::router())
        .merge(admin::router())
        .merge(preferences::router())
        .merge(playbooks::router())
        .merge(calls::router())
        .merge(oauth::router())
        .merge(site::router())
        .nest("/video", video::router())
        .merge(ws::router());

    let api_v4 = v4::router();

    Router::new()
        .nest("/api/v1", api_v1)
        .nest("/api/v4", api_v4)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
