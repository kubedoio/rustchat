// Core User Entity - Domain model, framework-agnostic

export type UserId = string
export type PresenceStatus = 'online' | 'away' | 'dnd' | 'offline'

// Export Presence as alias for PresenceStatus for compatibility
export type Presence = PresenceStatus

export interface User {
  id: UserId
  username: string
  email: string
  firstName?: string
  first_name?: string // API compatibility
  lastName?: string
  last_name?: string // API compatibility
  displayName?: string
  display_name?: string // API compatibility
  nickname?: string
  position?: string
  avatarUrl?: string
  avatar_url?: string // API compatibility
  role: 'system_admin' | 'org_admin' | 'user' | 'guest'
  presence: PresenceStatus
  isActive: boolean
  isBot?: boolean
  timezone?: string
  locale?: string
  customStatus?: {
    emoji: string
    text: string
    expiresAt?: Date
  }
  createdAt: Date
  updatedAt: Date
}

// Reference to a user (for embedding in other entities)
export interface UserRef {
  id: UserId
  username: string
  displayName?: string
  avatarUrl?: string
}

// Helper to create a user reference from full user
export function toUserRef(user: User): UserRef {
  return {
    id: user.id,
    username: user.username,
    displayName: user.displayName,
    avatarUrl: user.avatarUrl,
  }
}
