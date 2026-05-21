//! rustchat - Self-hosted team collaboration platform
//!
//! This crate provides the core functionality for rustchat,
//! a messaging platform built in Rust.

pub mod api;
pub mod auth;
pub mod calls;
pub mod config;
pub mod constants;
pub mod crypto;
pub mod db;
pub mod error;
pub mod jobs;
pub mod mattermost_compat;
pub mod middleware;
pub mod models;
pub mod realtime;
pub mod repositories;
pub mod services;
pub mod state;
pub mod storage;
pub mod telemetry;
