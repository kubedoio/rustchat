use crate::api::AppState;
use axum::{http::{HeaderName, HeaderValue}, response::IntoResponse, Json, Router};
use tower_http::set_header::SetResponseHeaderLayer;

pub mod channels;
pub mod categories;
pub mod config;
pub mod extractors;
pub mod files;
pub mod posts;
pub mod system;
pub mod teams;
pub mod users;
pub mod websocket;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(users::router())
        .merge(teams::router())
        .merge(channels::router())
        .merge(categories::router())
        .merge(posts::router())
        .merge(files::router())
        .merge(config::router())
        .merge(system::router())
        .merge(websocket::router())
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
