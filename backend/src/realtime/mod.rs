//! Real-time WebSocket module for rustchat
//!
//! Provides WebSocket hub for presence, typing indicators, and event fan-out.

pub mod hub;
pub mod events;

pub use hub::*;
pub use events::*;
