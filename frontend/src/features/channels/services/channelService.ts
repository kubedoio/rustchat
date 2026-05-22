// Channel Service - Business logic for channels
// Handles channel selection, persistence, and orchestration

import { log } from '@/utils/log'
import { channelRepository, type CreateChannelRequest } from '../repositories/channelRepository'
import type { Channel, ChannelId } from '../../../core/entities/Channel'
import type { TeamId } from '../../../core/entities/Team'
import type { UserId } from '../../../core/entities/User'
import { useChannelStore } from '../stores/channelStore'
import { useAuthStore } from '../../auth/stores/authStore'
import { AppError } from '../../../core/errors/AppError'

// Local storage key for last selected channels
const LAST_CHANNEL_KEY = 'last_channel_by_team'

class ChannelService {
  private get store() {
    return useChannelStore()
  }

  // Load channels for a team with automatic selection
  async loadChannels(teamId: TeamId): Promise<void> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      const channels = await channelRepository.listByTeam(teamId)
      this.store.setChannels(channels)

      // Try to restore last selected channel for this team
      const lastId = this.getLastChannelId(teamId)
      const hasLastChannel = lastId && channels.some(c => c.id === lastId)

      if (hasLastChannel) {
        this.store.setCurrentChannelId(lastId as ChannelId)
      } else {
        // Auto-select general channel if none selected or last not found
        const general = channels.find(c => c.name === 'general')
        const defaultChannel = general?.id || channels[0]?.id || null

        if (defaultChannel) {
          this.store.setCurrentChannelId(defaultChannel)
          this.saveLastChannelId(teamId, defaultChannel)
        }
      }
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to fetch channels')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Load joinable channels (for browse/join UI)
  async loadJoinableChannels(teamId: TeamId): Promise<void> {
    this.store.setLoading(true)
    try {
      const channels = await channelRepository.listJoinable(teamId)
      this.store.setJoinableChannels(channels)
    } finally {
      this.store.setLoading(false)
    }
  }

  // Select a channel and persist the choice
  selectChannel(channelId: ChannelId): void {
    const channel = this.store.getChannelById(channelId)
    if (!channel) return

    this.store.setCurrentChannelId(channelId)

    if (channel.teamId) {
      this.saveLastChannelId(channel.teamId, channelId)
    }

    // Clear unread counts when selecting
    this.store.clearCounts(channelId)
  }

  // Create a new channel
  async createChannel(data: CreateChannelRequest): Promise<Channel> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      const channel = await channelRepository.create(data)
      this.store.addChannel(channel)

      // Auto-select the new channel
      this.selectChannel(channel.id)

      return channel
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to create channel')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Join an existing channel
  async joinChannel(channelId: ChannelId, userId: UserId): Promise<void> {
    try {
      await channelRepository.join(channelId, userId)

      // Refresh channels to include the joined one
      const channel = await channelRepository.getById(channelId)
      if (channel) {
        this.store.addChannel(channel)
      }
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to join channel')
      throw error
    }
  }

  // Leave a channel
  async leaveChannel(channelId: ChannelId): Promise<void> {
    try {
      await channelRepository.leave(channelId)
      this.store.removeChannel(channelId)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to leave channel')
      throw error
    }
  }

  // Remove member from channel (admin/host)
  async removeMember(channelId: ChannelId, userId: UserId): Promise<void> {
    try {
      await channelRepository.removeMember(channelId, userId)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to remove member')
      throw error
    }
  }

  // Update channel
  async updateChannel(channelId: ChannelId, data: Partial<CreateChannelRequest>): Promise<Channel> {
    try {
      const channel = await channelRepository.update(channelId, data)
      this.store.updateChannel(channel)
      return channel
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to update channel')
      throw error
    }
  }

  // Delete/archive channel
  async deleteChannel(channelId: ChannelId): Promise<void> {
    try {
      await channelRepository.delete(channelId)
      this.store.removeChannel(channelId)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to delete channel')
      throw error
    }
  }

  // Mark channel as read
  async markAsRead(channelId: ChannelId): Promise<void> {
    try {
      await channelRepository.markAsRead(channelId)
      this.store.clearCounts(channelId)
    } catch (error) {
      log.error('Failed to mark channel as read:', error)
    }
  }

  // Load unread counts for all channels
  async loadUnreadCounts(): Promise<void> {
    try {
      const counts = await channelRepository.getUnreadCounts()
      this.store.setUnreadCounts(counts)
    } catch (error) {
      log.error('Failed to load unread counts:', error)
    }
  }

  // WebSocket event handlers
  handleChannelCreated(channel: Channel): void {
    this.store.addChannel(channel)
  }

  handleChannelUpdated(channel: Channel): void {
    this.store.updateChannel(channel)
  }

  handleChannelDeleted(channelId: ChannelId): void {
    this.store.removeChannel(channelId)
  }

  handleUserJoined(channelId: ChannelId, userId: UserId): void {
    // Could trigger a notification or update member list
    log.debug('User joined channel:', channelId, userId)
  }

  handleUserLeft(channelId: ChannelId, userId: UserId): void {
    log.debug('User left channel:', channelId, userId)
    const authStore = useAuthStore()
    if (authStore.user?.id === userId) {
      this.store.removeChannel(channelId)
    }
  }

  handleNewMessage(channelId: ChannelId, hasMention: boolean): void {
    // Only increment if not currently viewing this channel
    if (this.store.currentChannelId !== channelId) {
      this.store.incrementUnread(channelId)
      if (hasMention) {
        this.store.incrementMention(channelId)
      }
    }
  }

  // Private helpers for local storage
  private getLastChannelId(teamId: TeamId): string | null {
    try {
      const stored = localStorage.getItem(LAST_CHANNEL_KEY)
      if (!stored) return null
      const parsed = JSON.parse(stored)
      return parsed[teamId] || null
    } catch {
      return null
    }
  }

  private saveLastChannelId(teamId: TeamId, channelId: ChannelId): void {
    try {
      const stored = localStorage.getItem(LAST_CHANNEL_KEY)
      const parsed = stored ? JSON.parse(stored) : {}
      parsed[teamId] = channelId
      localStorage.setItem(LAST_CHANNEL_KEY, JSON.stringify(parsed))
    } catch {
      // Ignore localStorage errors
    }
  }
}

export const channelService = new ChannelService()
