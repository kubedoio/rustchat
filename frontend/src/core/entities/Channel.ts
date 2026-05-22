import type { UserId } from './User'

export type ChannelId = string

export type ChannelType = 'public' | 'private' | 'direct' | 'group'

export interface Channel {
  id: ChannelId
  teamId?: string // Use string instead of TeamId to avoid circular dependency
  name: string
  displayName: string
  type: ChannelType
  purpose?: string
  header?: string
  creatorId: UserId
  createdAt: Date
  updatedAt: Date

  // Membership
  memberCount?: number
  isArchived: boolean

  // For DM/Group channels
  participantIds?: UserId[]

  // Unread state (client-side only)
  unreadCount?: number
  mentionCount?: number
}

// Alias for DM channels
export type DMChannel = Channel & { type: 'direct' | 'group' }

export interface ChannelMember {
  channelId: ChannelId
  userId: UserId
  roles: string[]
  joinedAt: Date
  lastViewedAt?: Date
  notifyProps: {
    desktop: 'default' | 'all' | 'mention' | 'none'
    mobile: 'default' | 'all' | 'mention' | 'none'
    markUnread: 'all' | 'mention'
  }
}
