use crate::api::AppState;
use axum::Router;

pub mod channels;
pub mod config;
pub mod extractors;
pub mod posts;
pub mod teams;
pub mod users;
pub mod websocket;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(users::router())
        .merge(teams::router())
        .merge(channels::router())
        .merge(posts::router())
        .merge(config::router())
        .merge(websocket::router())
}
