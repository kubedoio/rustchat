/**
 * Centralized application constants.
 *
 * Values here should be kept in sync with backend/src/constants.rs
 * where applicable.
 */

// ------------------------------------------------------------------
// API base paths
// ------------------------------------------------------------------

export const API_V1_BASE = '/api/v1'
export const API_V4_BASE = '/api/v4'

// ------------------------------------------------------------------
// Pagination / limits
// ------------------------------------------------------------------

export const DEFAULT_MESSAGE_LIMIT = 50

// ------------------------------------------------------------------
// Timeouts (milliseconds)
// ------------------------------------------------------------------

export const TYPING_TIMEOUT = 5000
export const TYPING_CLEANUP_INTERVAL = 3000
export const WS_DISCONNECTED_TIMEOUT = 5000
export const WS_FAILED_TIMEOUT = 30000
export const MAX_RECONNECT_ATTEMPTS = 10
export const RECONNECT_DELAY_BASE_MS = 1000
export const RECONNECT_DELAY_MAX_MS = 10000
export const HTTP_DEFAULT_TIMEOUT = 30000

// ------------------------------------------------------------------
// File upload limits
// ------------------------------------------------------------------

/** Maximum profile image size (10 MB).
 *  Aligned with backend MAX_IMAGE_SIZE in constants.rs.
 */
export const MAX_PROFILE_IMAGE_SIZE = 10 * 1024 * 1024
