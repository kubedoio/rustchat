use axum::Router;
use crate::api::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
}
