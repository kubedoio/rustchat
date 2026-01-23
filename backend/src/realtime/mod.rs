//! Real-time WebSocket module for rustchat
//!
//! Provides WebSocket hub for presence, typing indicators, and event fan-out.

pub mod events;
pub mod hub;

pub use events::*;
pub use hub::*;
