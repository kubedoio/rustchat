// Unread Repository - Data access for unread counts

import client from '../../../api/client'
import type { ChannelId } from '../../../core/entities/Channel'
import type { TeamId } from '../../../core/entities/Team'
import { withRetry } from '../../../core/services/retry'

export interface ChannelUnread {
  channelId: ChannelId
  teamId: TeamId
  unreadCount: number
  mentionCount: number
}

export interface TeamUnread {
  teamId: TeamId
  unreadCount: number
}

export interface UnreadOverview {
  channels: ChannelUnread[]
  teams: TeamUnread[]
}

export interface ReadState {
  lastReadMessageId: string | null
  firstUnreadMessageId: string | null
}

export const unreadRepository = {
  // Get unread overview for all channels and teams
  async getOverview(): Promise<UnreadOverview> {
    return withRetry(async () => {
      const response = await client.get<{
        channels: Array<{
          channel_id: string
          team_id: string
          unread_count: number
          mention_count: number
        }>
        teams: Array<{
          team_id: string
          unread_count: number
        }>
      }>('/unreads/overview')

      return {
        channels: response.data.channels.map(c => ({
          channelId: c.channel_id as ChannelId,
          teamId: c.team_id as TeamId,
          unreadCount: c.unread_count,
          mentionCount: c.mention_count || 0,
        })),
        teams: response.data.teams.map(t => ({
          teamId: t.team_id as TeamId,
          unreadCount: t.unread_count,
        })),
      }
    })
  },

  // Mark channel as read
  async markAsRead(channelId: ChannelId, targetSeq?: string | number | null): Promise<void> {
    await withRetry(() => client.post(`/channels/${channelId}/read`, { target_seq: targetSeq }))
  },

  // Mark all channels as read
  async markAllAsRead(): Promise<void> {
    await withRetry(() => client.post('/unreads/mark_all_read'))
  },
}
