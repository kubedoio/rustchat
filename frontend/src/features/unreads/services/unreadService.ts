// Unread Service - Business logic for unread counts

import { log } from '@/utils/log';
import { unreadRepository, type ReadState } from '../repositories/unreadRepository'
import type { ChannelId } from '../../../core/entities/Channel'
import type { TeamId } from '../../../core/entities/Team'
import { useUnreadStore } from '../stores/unreadStore'

class UnreadService {
  private get store() {
    return useUnreadStore()
  }

  // Load unread overview
  async loadOverview(): Promise<void> {
    this.store.setLoading(true)
    try {
      const overview = await unreadRepository.getOverview()
      
      // Reset and populate
      this.store.clearAll()

      for (const channel of overview.channels) {
        this.store.setChannelUnread(channel.channelId, channel.unreadCount)
        this.store.setChannelMentions(channel.channelId, channel.mentionCount)
      }

      for (const team of overview.teams) {
        this.store.setTeamUnread(team.teamId, team.unreadCount)
      }
    } catch (error) {
      log.error('Failed to load unread overview:', error)
    } finally {
      this.store.setLoading(false)
    }
  }

  // Mark channel as read
  async markAsRead(channelId: ChannelId, targetSeq?: string | number | null): Promise<void> {
    try {
      await unreadRepository.markAsRead(channelId, targetSeq)

      // Optimistic update for standard "mark channel as read"
      if (!targetSeq) {
        this.store.setChannelUnread(channelId, 0)
        this.store.setChannelMentions(channelId, 0)
        this.store.setReadState(channelId, {
          last_read_message_id: null,
          first_unread_message_id: null
        })
      }
      // If targetSeq is provided, it's "mark as unread from here"
      // Let the WebSocket event or next fetch handle the update
    } catch (error) {
      log.error('Failed to mark channel as read:', error)
      throw error
    }
  }

  // Mark all as read
  async markAllAsRead(): Promise<void> {
    try {
      await unreadRepository.markAllAsRead()
      this.store.clearAll()
    } catch (error) {
      log.error('Failed to mark all as read:', error)
      throw error
    }
  }

  // Handle WebSocket unread update
  handleUnreadUpdate(data: {
    channel_id: string
    team_id: string
    unread_count: number
    mention_count?: number
  }): void {
    const channelId = data.channel_id as ChannelId
    const teamId = data.team_id as TeamId

    this.store.setChannelUnread(channelId, data.unread_count)
    if (data.mention_count !== undefined) {
      this.store.setChannelMentions(channelId, data.mention_count)
    }

    // Recalculate team unread
    this.recalculateTeamUnread(teamId)
  }

  // Handle new message (increment unreads if not viewing channel)
  handleNewMessage(channelId: ChannelId, teamId: TeamId, isMention: boolean): void {
    // Don't increment if viewing this channel
    // This check should be done by the caller or we need access to current channel
    // For now, just increment
    const currentCount = this.store.getChannelUnread(channelId)
    this.store.setChannelUnread(channelId, currentCount + 1)

    if (isMention) {
      const currentMentions = this.store.getChannelMentions(channelId)
      this.store.setChannelMentions(channelId, currentMentions + 1)
    }

    this.recalculateTeamUnread(teamId)
  }

  // Set read state for "new messages" line
  setReadState(channelId: ChannelId, state: ReadState): void {
    this.store.setReadState(channelId, {
      last_read_message_id: state.lastReadMessageId ?? null,
      first_unread_message_id: state.firstUnreadMessageId ?? null,
    })
  }

  // Private: Recalculate team unread total
  private recalculateTeamUnread(_teamId: TeamId): void {
    // This would need access to channel-team mappings
    // For now, the next overview fetch will correct it
  }
}

export const unreadService = new UnreadService()
