//! Centralized application constants.
//!
//! This module collects magic values that were previously scattered
//! across the backend to make them easier to audit and keep in sync
//! with the frontend.

// ------------------------------------------------------------------
// Pagination
// ------------------------------------------------------------------

/// Default number of items per page for Mattermost-compatible v4 endpoints.
pub const DEFAULT_PAGE_SIZE: i64 = 60;

/// Maximum number of items allowed per page.
pub const MAX_PAGE_SIZE: i64 = 200;

/// Default limit for post/search listings (v1 API).
pub const DEFAULT_SEARCH_LIMIT: i64 = 50;

/// Maximum limit for post/search listings (v1 API).
pub const MAX_SEARCH_LIMIT: i64 = 100;

// ------------------------------------------------------------------
// Roles
// ------------------------------------------------------------------

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_MEMBER: &str = "member";
pub const ROLE_GUEST: &str = "guest";
pub const ROLE_OWNER: &str = "owner";
pub const ROLE_SYSTEM_ADMIN: &str = "system_admin";
pub const ROLE_TEAM_ADMIN: &str = "team_admin";
pub const ROLE_CHANNEL_ADMIN: &str = "channel_admin";
pub const ROLE_ORG_ADMIN: &str = "org_admin";

// ------------------------------------------------------------------
// File upload limits
// ------------------------------------------------------------------

/// Maximum size for image uploads (10 MB).
pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum size for document uploads (25 MB).
pub const MAX_DOCUMENT_SIZE: usize = 25 * 1024 * 1024;

/// Maximum size for other file uploads (50 MB).
pub const MAX_OTHER_FILE_SIZE: usize = 50 * 1024 * 1024;
