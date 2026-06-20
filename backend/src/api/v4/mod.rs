use crate::api::AppState;
use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderName, HeaderValue},
    response::IntoResponse,
    Json, Router,
};
use serde_json::json;
use tower_http::set_header::SetResponseHeaderLayer;

pub mod access_control;
pub mod admin;
pub mod bots;
pub mod calls_plugin;
pub mod categories;
pub mod channel_bookmarks;
pub mod channels;
pub mod commands;
pub mod config_client;
pub mod content_flagging;
pub mod custom_profile;
pub mod emoji;
pub mod extractors;
pub mod files;
pub mod groups;
pub mod hooks;
pub mod image;
pub mod imports_exports;
pub mod jobs;
pub mod oauth;
pub mod plugins;
pub mod posts;

pub mod reports;
pub mod roles;
pub mod schemes;
pub mod status;
pub mod stubs;
pub mod system;
pub mod teams;
pub mod terms_of_service;
pub mod threads;
pub mod uploads;
pub mod usage;
pub mod users;
pub mod websocket;

/// Create v4 router with configurable body size limits
///
/// # Arguments
/// * `small_limit` - For simple JSON APIs (auth, status, etc.)
/// * `medium_limit` - For larger payloads (posts with content, user profiles)
/// * `large_limit` - For file uploads
pub fn router_with_body_limits(
    state: AppState,
    small_limit: usize,
    medium_limit: usize,
    large_limit: usize,
) -> Router<AppState> {
    let websocket_router = Router::new()
        .route(
            "/websocket",
            axum::routing::get(websocket::handle_websocket),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::websocket_ip_rate_limit,
        ));

    Router::new()
        // User management - medium limit for profiles
        .merge(users::router(state.clone()).layer(DefaultBodyLimit::max(medium_limit)))
        // Teams - small limit
        .merge(teams::router(state.clone()).layer(DefaultBodyLimit::max(small_limit)))
        // Groups - small limit
        .merge(groups::router().layer(DefaultBodyLimit::max(small_limit)))
        // Channels - medium limit for descriptions/settings
        .merge(channels::router(state.clone()).layer(DefaultBodyLimit::max(medium_limit)))
        // Emoji - small limit
        .merge(emoji::router(state.clone()).layer(DefaultBodyLimit::max(small_limit)))
        // Commands - small limit
        .merge(commands::router().layer(DefaultBodyLimit::max(small_limit)))
        // Plugins - medium limit
        .merge(plugins::router().layer(DefaultBodyLimit::max(medium_limit)))
        // Categories - small limit
        .merge(categories::router().layer(DefaultBodyLimit::max(small_limit)))
        // Posts - medium limit for message content
        .merge(posts::router(state.clone()).layer(DefaultBodyLimit::max(medium_limit)))
        // Status - small limit
        .merge(status::router().layer(DefaultBodyLimit::max(small_limit)))
        // Bookmarks - small limit
        .merge(channel_bookmarks::router().layer(DefaultBodyLimit::max(small_limit)))
        // Files - large limit for uploads
        .merge(files::router(state.clone()).layer(DefaultBodyLimit::max(large_limit)))
        // System - small limit
        .merge(system::router().layer(DefaultBodyLimit::max(small_limit)))
        // Config - small limit
        .merge(config_client::router().layer(DefaultBodyLimit::max(small_limit)))
        // Hooks - medium limit for webhook payloads
        .merge(hooks::router().layer(DefaultBodyLimit::max(medium_limit)))
        // Bots - small limit
        .merge(bots::router().layer(DefaultBodyLimit::max(small_limit)))
        // Email admin routes (must be before admin::router() to avoid fallback shadowing)
        .merge(crate::api::admin_email::router().layer(DefaultBodyLimit::max(small_limit)))
        // Admin - medium limit
        .merge(admin::router().layer(DefaultBodyLimit::max(medium_limit)))
        // OAuth - small limit
        .merge(oauth::router().layer(DefaultBodyLimit::max(small_limit)))
        // Schemes - small limit
        .merge(schemes::router().layer(DefaultBodyLimit::max(small_limit)))
        // Access control - small limit
        .merge(access_control::router(state.clone()).layer(DefaultBodyLimit::max(small_limit)))
        // Content flagging - small limit
        .merge(content_flagging::router(state.clone()).layer(DefaultBodyLimit::max(small_limit)))
        // Usage - small limit
        .merge(usage::router().layer(DefaultBodyLimit::max(small_limit)))
        // Custom profile - medium limit
        .merge(custom_profile::router().layer(DefaultBodyLimit::max(medium_limit)))
        // Roles - small limit
        .merge(roles::router().layer(DefaultBodyLimit::max(small_limit)))
        // Jobs - small limit
        .merge(jobs::router().layer(DefaultBodyLimit::max(small_limit)))
        // Reports - small limit
        .merge(reports::router().layer(DefaultBodyLimit::max(small_limit)))
        // Imports/exports - large limit for data files
        .merge(imports_exports::router().layer(DefaultBodyLimit::max(large_limit)))
        // Terms of service - medium limit
        .merge(terms_of_service::router().layer(DefaultBodyLimit::max(medium_limit)))
        // Uploads - large limit for file uploads
        .merge(
            uploads::router()
                .layer(DefaultBodyLimit::max(large_limit))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::middleware::rate_limit::upload_ip_rate_limit,
                )),
        )
        // Threads - medium limit
        .merge(threads::router().layer(DefaultBodyLimit::max(medium_limit)))
        // Image - medium limit for image processing
        .merge(image::router().layer(DefaultBodyLimit::max(medium_limit)))
        // Mattermost Calls plugin API - small limit
        .merge(calls_plugin::router().layer(DefaultBodyLimit::max(small_limit)))
        // Mattermost API Compatibility Stubs - small limit for general endpoints, medium limit for brand image
        .merge(stubs::router(state.clone()).layer(DefaultBodyLimit::max(small_limit)))
        .merge(stubs::brand_router().layer(DefaultBodyLimit::max(medium_limit)))
        // WebSocket - no body limit (upgrade request)
        .merge(websocket_router)
        .fallback(not_implemented)
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-mm-compat"),
            HeaderValue::from_static("1"),
        ))
}

pub fn mm_not_implemented(
    id: &str,
    message: &str,
    detailed_error: &str,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "id": id,
            "message": message,
            "detailed_error": detailed_error,
            "request_id": "",
            "status_code": 501
        })),
    )
}

async fn not_implemented() -> impl IntoResponse {
    mm_not_implemented(
        "api.route.not_implemented.app_error",
        "This API route is not implemented.",
        "The requested Mattermost v4 endpoint is not available in this build.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_router_builds_without_overlaps() {
        let _builder: fn(AppState, usize, usize, usize) -> axum::Router<AppState> =
            router_with_body_limits;
    }
}
