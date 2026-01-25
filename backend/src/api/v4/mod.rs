use axum::Router;
use crate::api::AppState;

pub mod users;
pub mod teams;
pub mod channels;
pub mod posts;
pub mod config;
pub mod websocket;
pub mod extractors;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(users::router())
        .merge(teams::router())
        .merge(channels::router())
        .merge(posts::router())
        .merge(config::router())
        .merge(websocket::router())
}
