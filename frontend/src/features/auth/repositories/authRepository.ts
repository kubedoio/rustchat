// Auth Repository - Data access for authentication

import client from '../../../api/client'
import type { PresenceStatus, User, UserId } from '../../../core/entities/User'
import { withRetry } from '../../../core/services/retry'

export interface LoginCredentials {
  email?: string
  username?: string
  password: string
}

export interface LoginResponse {
  token: string
  user: User
}

export interface AuthPolicy {
  allowSignup: boolean
  requireEmailVerification: boolean
  allowedDomains?: string[]
  minPasswordLength: number
}

export interface UpdateStatusRequest {
  presence?: 'online' | 'away' | 'dnd' | 'offline'
  text?: string
  emoji?: string
  durationMinutes?: number
}

export const authRepository = {
  // Login with credentials
  async login(credentials: LoginCredentials): Promise<LoginResponse> {
    return withRetry(async () => {
      const response = await client.post('/auth/login', credentials)

      // Normalize user data
      const user = normalizeUser(response.data.user)

      return {
        token: response.data.token,
        user,
      }
    })
  },

  // Fetch current user profile
  async fetchMe(): Promise<User> {
    return withRetry(async () => {
      const response = await client.get('/auth/me')
      return normalizeUser(response.data)
    })
  },

  // Logout on server (optional, mainly for session invalidation)
  async logout(): Promise<void> {
    // Server-side logout if needed
    try {
      await client.post('/auth/logout')
    } catch {
      // Ignore errors during logout
    }
  },

  // Update user status/presence
  async updateStatus(request: UpdateStatusRequest): Promise<{
    presence?: string
    text?: string
    emoji?: string
    expiresAt?: string
  }> {
    return withRetry(async () => {
      const response = await client.put('/users/me/status', {
        presence: request.presence,
        text: request.text,
        emoji: request.emoji,
        duration_minutes: request.durationMinutes,
      })
      return response.data
    })
  },

  // Get auth policy (signup settings, etc)
  async getAuthPolicy(): Promise<AuthPolicy> {
    return withRetry(async () => {
      const response = await client.get('/auth/policy')
      return {
        allowSignup: response.data.allow_signup ?? true,
        requireEmailVerification: response.data.require_email_verification ?? false,
        allowedDomains: response.data.allowed_domains,
        minPasswordLength: response.data.min_password_length ?? 8,
      }
    })
  },

  // Token storage (sessionStorage)
  getStoredToken(): string | null {
    try {
      return sessionStorage.getItem('auth_token')
    } catch {
      return null
    }
  },

  setStoredToken(token: string): void {
    try {
      sessionStorage.setItem('auth_token', token)
    } catch {
      // Ignore storage errors
    }
  },

  clearStoredToken(): void {
    try {
      sessionStorage.removeItem('auth_token')
    } catch {
      // Ignore storage errors
    }
  },
}

// Normalize API user response to domain entity
function normalizeUser(raw: unknown): User {
  const r = raw as Record<string, unknown>
  const customStatus = (r.custom_status as Record<string, unknown> | undefined) || {
    text: r.status_text,
    emoji: r.status_emoji,
    expires_at: r.status_expires_at,
  }

  const role =
    r.role === 'system_admin' || r.role === 'org_admin' || r.role === 'guest' ? r.role : 'user'
  const presence: PresenceStatus =
    r.presence === 'online' || r.presence === 'away' || r.presence === 'dnd'
      ? r.presence
      : 'offline'
  const statusEmoji = typeof customStatus.emoji === 'string' ? customStatus.emoji : ''
  const statusText = typeof customStatus.text === 'string' ? customStatus.text : ''

  return {
    id: r.id as UserId,
    username: r.username as string,
    email: r.email as string,
    displayName: r.display_name as string,
    avatarUrl: (r.avatar_url || r.profile_image) as string | undefined,
    role,
    presence,
    isActive: r.is_active !== false, // Default to true if not specified
    isBot: Boolean(r.is_bot),
    timezone: r.timezone as string | undefined,
    locale: r.locale as string | undefined,
    customStatus:
      statusText || statusEmoji
        ? {
            emoji: statusEmoji,
            text: statusText,
            expiresAt: customStatus.expires_at
              ? new Date(customStatus.expires_at as string | number)
              : undefined,
          }
        : undefined,
    createdAt: new Date((r.created_at || Date.now()) as string | number),
    updatedAt: new Date((r.updated_at || r.created_at || Date.now()) as string | number),
  }
}
