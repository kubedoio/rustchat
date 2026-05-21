// Auth-related types for the authentication store

import type { User, UserId } from './User'

// Re-export User for convenience
export type { User, UserId }

// Extended User type with status fields returned by the API
export interface AuthUser extends User {
  status_text?: string | null
  status_emoji?: string | null
  status_expires_at?: string | number | null
  custom_status?: {
    emoji: string | null
    text: string | null
    expires_at?: string | number | null
    expiresAt?: string | number | null
  } | null
}

// Login credentials for authentication
export interface LoginCredentials {
  login_id?: string
  email?: string
  username?: string
  password: string
}

// Registration input data
export interface RegisterInput {
  email: string
  username: string
  password: string
}

// Authentication response from the API
export interface AuthResponse {
  token: string
  user: AuthUser
}

// Auth policy configuration
export interface AuthPolicy {
  allowSignup: boolean
  requireEmailVerification: boolean
  // Password policy fields (camelCase)
  passwordMinLength?: number
  passwordRequireUppercase?: boolean
  passwordRequireLowercase?: boolean
  passwordRequireNumber?: boolean
  passwordRequireSymbol?: boolean
  // Password policy fields (snake_case for API compatibility)
  password_min_length?: number
  password_require_uppercase?: boolean
  password_require_lowercase?: boolean
  password_require_number?: boolean
  password_require_symbol?: boolean
}

// Status update input
export interface StatusUpdateInput {
  status?: string
  presence?: string
  text?: string
  emoji?: string
  duration?: string
  duration_minutes?: number
  dnd_end_time?: number
}

// Status snapshot for syncing user status
export interface StatusSnapshot {
  status?: string | null
  presence?: string | null
  text?: string | null
  emoji?: string | null
  expiresAt?: string | number | null
  expires_at?: string | number | null
}
