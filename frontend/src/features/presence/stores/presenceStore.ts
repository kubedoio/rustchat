// Presence Store - Pure state management for presence/typing

import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import type { UserId, PresenceStatus } from '../../../core/entities/User'
import type { ChannelId } from '../../../core/entities/Channel'
import type { MessageId } from '../../../core/entities/Message'

export interface PresenceUser {
  userId: UserId
  username: string
  presence: PresenceStatus
  lastActiveAt?: Date
}

export interface TypingUser {
  userId: UserId
  username: string
  channelId: ChannelId
  timestamp: number
  threadRootId?: MessageId
}

export const usePresenceStore = defineStore('presenceStore', () => {
  // State
  const self = ref<PresenceUser | null>(null)
  const presenceMap = ref<Map<UserId, PresenceUser>>(new Map())
  const typingUsers = ref<Map<string, TypingUser>>(new Map())

  // Getters
  const onlineCount = computed(() => {
    let count = 0
    for (const user of presenceMap.value.values()) {
      if (user.presence === 'online') count++
    }
    if (self.value?.presence === 'online') count++
    return count
  })

  // Actions
  function setSelfPresence(data: Partial<PresenceUser> & { userId: UserId }) {
    if (!self.value) {
      self.value = {
        userId: data.userId,
        username: data.username || '',
        presence: data.presence || 'online',
        lastActiveAt: data.lastActiveAt || new Date(),
      }
    } else {
      if (data.presence !== undefined) self.value.presence = data.presence
      if (data.username !== undefined) self.value.username = data.username
      if (data.lastActiveAt !== undefined) self.value.lastActiveAt = data.lastActiveAt
    }
  }

  function setUserPresence(userId: UserId, username: string, presence: PresenceStatus) {
    presenceMap.value.set(userId, {
      userId,
      username,
      presence,
      lastActiveAt: new Date(),
    })
  }

  function updatePresenceFromEvent(userId: UserId, presence: PresenceStatus) {
    if (self.value?.userId === userId) {
      self.value.presence = presence
      self.value.lastActiveAt = new Date()
    } else {
      const user = presenceMap.value.get(userId)
      if (user) {
        user.presence = presence
        user.lastActiveAt = new Date()
      } else {
        presenceMap.value.set(userId, {
          userId,
          username: '',
          presence,
          lastActiveAt: new Date(),
        })
      }
    }
  }

  function addTypingUser(
    userId: UserId,
    username: string,
    channelId: ChannelId,
    threadRootId?: MessageId
  ) {
    const key = `${channelId}:${threadRootId || 'root'}:${userId}`
    typingUsers.value.set(key, {
      userId,
      username,
      channelId,
      timestamp: Date.now(),
      threadRootId,
    })
  }

  function removeTypingUser(userId: UserId, channelId: ChannelId, threadRootId?: MessageId) {
    const key = `${channelId}:${threadRootId || 'root'}:${userId}`
    typingUsers.value.delete(key)
  }

  function getTypingUsersForChannel(channelId: ChannelId, threadRootId?: MessageId) {
    return computed(() => {
      const users: TypingUser[] = []
      for (const user of typingUsers.value.values()) {
        if (user.channelId === channelId) {
          if (threadRootId) {
            if (user.threadRootId === threadRootId) users.push(user)
          } else {
            if (!user.threadRootId) users.push(user)
          }
        }
      }
      return users
    })
  }

  // Check if there are any typing users in a channel (for sidebar badges)
  function hasTypingUsers(channelId: ChannelId): boolean {
    for (const user of typingUsers.value.values()) {
      if (user.channelId === channelId && !user.threadRootId) {
        return true
      }
    }
    return false
  }

  function getUserPresence(userId: UserId) {
    return computed(() => {
      if (self.value?.userId === userId) return self.value
      return presenceMap.value.get(userId)
    })
  }

  function clear() {
    self.value = null
    presenceMap.value.clear()
    typingUsers.value.clear()
  }

  // Internal method for service to access raw map
  function getTypingUsersMap() {
    return typingUsers.value
  }

  return {
    // State (readonly)
    self: readonly(self),
    presenceMap: readonly(presenceMap),
    typingUsers: readonly(typingUsers),

    // Getters
    onlineCount,

    // Actions
    setSelfPresence,
    setUserPresence,
    updatePresenceFromEvent,
    addTypingUser,
    removeTypingUser,
    getTypingUsersForChannel,
    hasTypingUsers,
    getUserPresence,
    getTypingUsersMap,
    clear,
  }
})
