// Channel Repository - Data access for channels
// Maps API responses to domain entities

import { channelsApi, categoriesApi } from '../../../api/channels'
import type { 
  Channel, 
  ChannelId, 
  ChannelType,
  ChannelMember
} from '../../../core/entities/Channel'
import type { TeamId } from '../../../core/entities/Team'
import type { UserId } from '../../../core/entities/User'
import { withRetry } from '../../../core/services/retry'
import type { SidebarCategory } from '../../../api/channels'
import { isNotFoundError } from '../../../core/errors/errorUtils'

export interface CreateChannelRequest {
  teamId: TeamId
  name: string
  displayName: string
  type: ChannelType
  header?: string
  purpose?: string
  targetUserId?: UserId
}

export interface ChannelUnreadCounts {
  channelId: ChannelId
  unreadCount: number
  mentionCount: number
}

export interface ChannelNotifyProps {
  desktop?: string
  mobile?: string
  mark_unread?: string
  ignore_channel_mentions?: string
}

export const channelRepository = {
  // List channels for a team
  async listByTeam(teamId: TeamId): Promise<Channel[]> {
    return withRetry(async () => {
      const response = await channelsApi.list(teamId)
      return response.data.map(normalizeChannel)
    })
  },

  // List joinable channels
  async listJoinable(teamId: TeamId): Promise<Channel[]> {
    return withRetry(async () => {
      const response = await channelsApi.listJoinable(teamId)
      return response.data.map(normalizeChannel)
    })
  },

  // Get single channel
  async getById(channelId: ChannelId): Promise<Channel | null> {
    return withRetry(async () => {
      try {
        const response = await channelsApi.get(channelId)
        return normalizeChannel(response.data)
      } catch (error: unknown) {
        if (isNotFoundError(error)) {
          return null
        }
        throw error
      }
    })
  },

  // Create channel
  async create(data: CreateChannelRequest): Promise<Channel> {
    return withRetry(async () => {
      const response = await channelsApi.create({
        team_id: data.teamId,
        name: data.name,
        display_name: data.displayName,
        channel_type: data.type,
        header: data.header,
        purpose: data.purpose,
        target_user_id: data.targetUserId
      })
      return normalizeChannel(response.data)
    })
  },

  // Update channel
  async update(
    channelId: ChannelId, 
    data: Partial<CreateChannelRequest>
  ): Promise<Channel> {
    return withRetry(async () => {
      const response = await channelsApi.update(channelId, {
        team_id: data.teamId,
        name: data.name,
        display_name: data.displayName,
        channel_type: data.type,
        header: data.header,
        purpose: data.purpose,
        target_user_id: data.targetUserId
      })
      return normalizeChannel(response.data)
    })
  },

  // Delete/archive channel
  async delete(channelId: ChannelId): Promise<void> {
    await withRetry(() => channelsApi.delete(channelId))
  },

  // Join channel
  async join(channelId: ChannelId, userId: UserId): Promise<void> {
    await withRetry(() => channelsApi.join(channelId, userId))
  },

  // Leave channel
  async leave(channelId: ChannelId): Promise<void> {
    await withRetry(() => channelsApi.leave(channelId))
  },

  // Remove member (host/admin only)
  async removeMember(channelId: ChannelId, userId: UserId): Promise<void> {
    await withRetry(() => channelsApi.removeMember(channelId, userId))
  },

  // Get channel members
  async getMembers(channelId: ChannelId): Promise<ChannelMember[]> {
    return withRetry(async () => {
      const response = await channelsApi.getMembers(channelId)
      return response.data.map(normalizeChannelMember)
    })
  },

  // Get unread counts for all channels
  async getUnreadCounts(): Promise<ChannelUnreadCounts[]> {
    return withRetry(async () => {
      const response = await channelsApi.getUnreadCounts()
      return response.data.map(item => ({
        channelId: item.channel_id as ChannelId,
        unreadCount: item.count,
        mentionCount: 0 // API returns separate mention counts
      }))
    })
  },

  // Mark channel as read (MM-compatible)
  async markAsRead(channelId: ChannelId, userId: UserId = 'me'): Promise<void> {
    await withRetry(() => channelsApi.markAsRead(channelId, userId))
  },

  // Mark channel as unread (MM-compatible)
  async markAsUnread(channelId: ChannelId, userId: UserId = 'me'): Promise<void> {
    await withRetry(() => channelsApi.markAsUnread(channelId, userId))
  },

  // Get channel member info including notify_props
  async getMember(channelId: ChannelId, userId: UserId): Promise<ChannelMember | null> {
    return withRetry(async () => {
      try {
        const response = await channelsApi.getMember(channelId, userId)
        return normalizeChannelMember(response.data)
      } catch (error: unknown) {
        if (isNotFoundError(error)) {
          return null
        }
        throw error
      }
    })
  },

  // Update notify props (for mute/unmute)
  async updateNotifyProps(
    channelId: ChannelId, 
    userId: UserId, 
    props: ChannelNotifyProps
  ): Promise<void> {
    return withRetry(async () => {
      await channelsApi.updateNotifyProps(channelId, userId, props)
    })
  },

  // Add member to channel
  async addMember(channelId: ChannelId, userId: UserId): Promise<ChannelMember> {
    return withRetry(async () => {
      const response = await channelsApi.addMember(channelId, userId)
      return normalizeChannelMember(response.data)
    })
  },

  // Get sidebar categories for a user/team
  async getCategories(userId: UserId, teamId: TeamId): Promise<SidebarCategory[]> {
    return withRetry(async () => {
      const response = await categoriesApi.getCategories(userId, teamId)
      return response.data.categories
    })
  },

  // Update sidebar categories (for moving channels)
  async updateCategories(
    userId: UserId, 
    teamId: TeamId, 
    categories: SidebarCategory[]
  ): Promise<SidebarCategory[]> {
    return withRetry(async () => {
      const response = await categoriesApi.updateCategories(userId, teamId, categories)
      return response.data
    })
  }
}

// Normalize API Channel to domain entity
function normalizeChannel(raw: unknown): Channel {
  const r = raw as Record<string, unknown>
  return {
    id: r.id as ChannelId,
    teamId: r.team_id as string,
    name: r.name as string,
    displayName: r.display_name as string,
    type: (r.type || r.channel_type) as Channel['type'], // Support both field names (backend now sends 'type')
    purpose: r.purpose as string | undefined,
    header: r.header as string | undefined,
    creatorId: r.creator_id as UserId,
    createdAt: new Date(r.created_at as string | number),
    updatedAt: new Date((r.updated_at || r.created_at) as string | number),
    isArchived: Boolean(r.is_archived),
    memberCount: r.member_count as number | undefined
  }
}

function normalizeChannelMember(raw: unknown): ChannelMember {
  const r = raw as Record<string, unknown>
  const notifyProps = (r.notify_props as Record<string, unknown> | undefined) ?? {}
  return {
    channelId: r.channel_id as ChannelId,
    userId: r.user_id as UserId,
    roles: (r.roles as string[] | undefined) || [],
    joinedAt: new Date(r.joined_at as string | number),
    lastViewedAt: r.last_viewed_at ? new Date(r.last_viewed_at as string | number) : undefined,
    notifyProps: {
      desktop: normalizeNotifySetting(notifyProps.desktop, 'default'),
      mobile: normalizeNotifySetting(notifyProps.mobile, 'default'),
      markUnread: normalizeMarkUnread(notifyProps.mark_unread)
    }
  }
}

function normalizeNotifySetting(value: unknown, fallback: 'default'): 'default' | 'all' | 'mention' | 'none' {
  return value === 'all' || value === 'mention' || value === 'none' || value === 'default'
    ? value
    : fallback
}

function normalizeMarkUnread(value: unknown): 'all' | 'mention' {
  return value === 'mention' ? 'mention' : 'all'
}
