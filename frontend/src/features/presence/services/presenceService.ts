// Presence Service - Business logic for presence/typing

import type { UserId, PresenceStatus } from '../../../core/entities/User'
import type { ChannelId } from '../../../core/entities/Channel'
import type { MessageId } from '../../../core/entities/Message'
import { usePresenceStore } from '../stores/presenceStore'
import { TYPING_CLEANUP_INTERVAL, TYPING_TIMEOUT } from '../../../constants'

class PresenceService {
  private get store() {
    return usePresenceStore()
  }

  private cleanupTimer: ReturnType<typeof setInterval> | null = null

  // Initialize presence system
  initialize(): void {
    this.startCleanupTimer()
  }

  // Cleanup on service destruction
  destroy(): void {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer)
      this.cleanupTimer = null
    }
  }

  // Set current user's presence
  setSelfPresence(userId: UserId, username: string, presence: PresenceStatus): void {
    this.store.setSelfPresence({ userId, username, presence })
  }

  // Set another user's presence
  setUserPresence(userId: UserId, username: string, presence: PresenceStatus): void {
    this.store.setUserPresence(userId, username, presence)
  }

  // Update presence from WebSocket event
  handlePresenceUpdate(userId: UserId, presence: PresenceStatus): void {
    const normalizedPresence = presence.toLowerCase() as PresenceStatus
    this.store.updatePresenceFromEvent(userId, normalizedPresence)
  }

  // Typing indicators
  addTypingUser(
    userId: UserId, 
    username: string, 
    channelId: ChannelId, 
    threadRootId?: MessageId
  ): void {
    this.store.addTypingUser(userId, username, channelId, threadRootId)
  }

  removeTypingUser(
    userId: UserId, 
    channelId: ChannelId, 
    threadRootId?: MessageId
  ): void {
    this.store.removeTypingUser(userId, channelId, threadRootId)
  }

  // Get typing users (returns computed ref)
  getTypingUsers(channelId: ChannelId, threadRootId?: MessageId) {
    return this.store.getTypingUsersForChannel(channelId, threadRootId)
  }

  // Get user presence (returns computed ref)
  getUserPresence(userId: UserId) {
    return this.store.getUserPresence(userId)
  }

  // WebSocket event handlers
  handleStatusChangeEvent(data: { user_id: string; status: string }): void {
    this.handlePresenceUpdate(data.user_id as UserId, data.status as PresenceStatus)
  }

  handleTypingEvent(data: {
    user_id: string
    username?: string
    channel_id: string
    thread_root_id?: string
    is_typing: boolean
  }): void {
    const userId = data.user_id as UserId
    const channelId = data.channel_id as ChannelId
    const threadRootId = data.thread_root_id as MessageId | undefined
    const username = data.username || ''

    if (data.is_typing) {
      this.addTypingUser(userId, username, channelId, threadRootId)
    } else {
      this.removeTypingUser(userId, channelId, threadRootId)
    }
  }

  // Private: Start cleanup timer for stale typing indicators
  private startCleanupTimer(): void {
    this.cleanupTimer = setInterval(() => {
      this.cleanupStaleTypingIndicators()
    }, TYPING_CLEANUP_INTERVAL)
  }

  private cleanupStaleTypingIndicators(): void {
    const now = Date.now()
    const staleKeys: string[] = []

    // Access the raw map through the store's internal state
    const typingUsers = this.store.getTypingUsersMap()
    
    for (const [key, user] of typingUsers.entries()) {
      if (now - user.timestamp > TYPING_TIMEOUT) {
        staleKeys.push(key)
      }
    }

    for (const key of staleKeys) {
      typingUsers.delete(key)
    }
  }
}

export const presenceService = new PresenceService()
