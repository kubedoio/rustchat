use axum::{Router, routing::get, routing::post};
use crate::api::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Discovery / Config
        .route("/config/client", get(super::v4::config_client::get_client_config))
        
        // Auth
        .route("/users/login", post(super::v4::users_login::login))
        .route("/users/me", get(super::v4::users::me))
        .route("/users/me/teams", get(super::v4::users::my_teams))
        .route("/users/me/teams/members", get(super::v4::users::my_team_members))
        .route("/users/me/teams/unread", get(super::v4::users::my_teams_unread))
        .route("/users/me/teams/{team_id}/channels", get(super::v4::users::get_team_channels))
        .route("/users/me/teams/{team_id}/channels/members", get(super::v4::users::get_team_channel_members))
        .route("/users/me/channels/categories", get(super::v4::users::get_categories))
        
        // Licensing
        .route("/license/client", get(super::v4::config_client::get_client_license))
        
        // WebSocket
        .route("/websocket", get(super::websocket::handle_websocket))
        
        // Add more routes here based on discovery
}
