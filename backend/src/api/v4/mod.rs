use crate::api::AppState;
use axum::{http::{HeaderName, HeaderValue}, response::IntoResponse, Json, Router};
use tower_http::set_header::SetResponseHeaderLayer;

pub mod channels;
pub mod emoji;
pub mod commands;
pub mod plugins;
pub mod categories;
pub mod compat;
pub mod extractors;
pub mod files;
pub mod posts;
pub mod system;
pub mod teams;
pub mod threads;
pub mod users;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(users::router())
        .merge(teams::router())
        .merge(channels::router())
        .merge(emoji::router())
        .merge(commands::router())
        .merge(plugins::router())
        .merge(categories::router())
        .merge(posts::router())
        .merge(files::router())
        .merge(system::router())
        .merge(threads::router())
        .merge(compat::router::router())
        .fallback(not_implemented)
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-mm-compat"),
            HeaderValue::from_static("1"),
        ))
}

async fn not_implemented() -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "id": "api.not_implemented",
            "message": "Not implemented",
            "status_code": 501
        }))
    )
}
