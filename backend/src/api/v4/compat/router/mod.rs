use axum::{Router, routing::get, routing::post};
use crate::api::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Discovery / Config
        .route("/api/v4/config/client", get(super::v4::config_client::get_client_config))
        
        // Auth
        .route("/api/v4/users/login", post(super::v4::users_login::login))
        
        // WebSocket
        .route("/api/v4/websocket", get(super::websocket::handle_websocket))
        
        // Add more routes here based on discovery
}
