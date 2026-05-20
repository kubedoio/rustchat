//! Mattermost Calls Plugin API
//!
//! Implements the com.mattermost.calls plugin interface for Mattermost Mobile compatibility.
//! Routes are mounted under /plugins/com.mattermost.calls/

use crate::api::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub mod commands;
mod turn;

// Re-export moved types so existing `super::sfu` / `super::state` imports keep working
pub mod sfu {
    pub use crate::calls::sfu::*;
}
pub mod state {
    pub use crate::calls::state::*;
}

pub use crate::calls::sfu::VoiceEvent;

mod actions;
mod broadcast;
mod channels;
mod config;
mod helpers;
mod host;
mod lifecycle;
mod management;
mod notifications;
mod posts;
mod recording;
mod signaling;
mod state_helpers;
mod webrtc;
mod websocket;

/// Build the calls plugin router
pub fn router() -> Router<AppState> {
    Router::new()
        // Plugin info endpoints
        .route(
            "/plugins/com.mattermost.calls/version",
            get(config::get_version),
        )
        .route(
            "/plugins/com.mattermost.calls/config",
            get(config::get_config),
        )
        // Channels with calls enabled
        .route(
            "/plugins/com.mattermost.calls/channels",
            get(channels::get_channels),
        )
        // Avoid overlap with /api/v4/plugins/{plugin_id}/enable|disable mutation routes.
        .route(
            "/plugins/com.mattermost.calls/enable",
            post(management::plugin_management_enable_not_implemented),
        )
        .route(
            "/plugins/com.mattermost.calls/disable",
            post(management::plugin_management_disable_not_implemented),
        )
        // Mattermost mobile compatibility: some clients call
        // /plugins/com.mattermost.calls/{channel_id}?mobilev2=true directly.
        .route(
            "/plugins/com.mattermost.calls/{channel_id}",
            get(channels::get_channel_state_mobile).post(channels::set_channel_calls_enabled),
        )
        // Call management endpoints
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/start",
            post(lifecycle::start_call),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/join",
            post(lifecycle::join_call),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/leave",
            post(lifecycle::leave_call),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/end",
            post(lifecycle::end_call_endpoint),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}",
            get(lifecycle::get_call_state),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/react",
            post(actions::send_reaction),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/screen-share",
            post(actions::toggle_screen_share),
        )
        // Mute/unmute endpoints
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/mute",
            post(actions::mute_user),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/unmute",
            post(actions::unmute_user),
        )
        // Raise/lower hand endpoints
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/raise-hand",
            post(actions::raise_hand),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/lower-hand",
            post(actions::lower_hand),
        )
        // WebRTC signaling endpoints
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/offer",
            post(webrtc::handle_offer),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/ice",
            post(webrtc::handle_ice_candidate),
        )
        // Host control endpoints
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/host/mute",
            post(host::host_mute),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/host/mute-others",
            post(host::host_mute_others),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/host/remove",
            post(host::host_remove_user),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/host/lower-hand",
            post(host::host_lower_hand),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/host/make",
            post(host::host_make_moderator),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/host/screen-off",
            post(host::host_screen_off),
        )
        // Notification endpoints
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/dismiss-notification",
            post(notifications::dismiss_notification),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/ring",
            post(notifications::ring_users),
        )
        .route(
            "/plugins/com.mattermost.calls/turn-credentials",
            get(config::get_turn_credentials),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/recording/start",
            post(recording::start_recording),
        )
        .route(
            "/plugins/com.mattermost.calls/calls/{channel_id}/recording/stop",
            post(recording::stop_recording),
        )
        // Slash commands
        .merge(commands::router())
}

// Re-exports for external callers
pub use broadcast::start_voice_event_listener;
pub use websocket::{handle_ws_action, handle_ws_connection_closed};
