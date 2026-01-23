import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export type Presence = 'online' | 'away' | 'dnd' | 'offline';

export interface PresenceUser {
    userId: string
    username: string
    presence: Presence
    lastActiveAt?: string
}

export interface TypingUser {
    userId: string
    username: string
    channelId: string
    timestamp: number
    threadRootId?: string
}

export const usePresenceStore = defineStore('presence', () => {
    // Current user's presence
    const self = ref<PresenceUser | null>(null)

    // Teammates presence map: userId -> PresenceUser
    const presenceMap = ref<Map<string, PresenceUser>>(new Map())

    // Typing users map: `${channelId}:${threadRootId || 'root'}:${userId}` -> TypingUser
    const typingUsers = ref<Map<string, TypingUser>>(new Map())

    // Clean up stale typing indicators every 3 seconds
    setInterval(() => {
        const now = Date.now()
        for (const [key, user] of typingUsers.value.entries()) {
            if (now - user.timestamp > 5000) {
                typingUsers.value.delete(key)
            }
        }
    }, 3000)

    function setSelfPresence(userData: Partial<PresenceUser>) {
        if (!self.value && userData.userId) {
            self.value = {
                userId: userData.userId,
                username: userData.username || '',
                presence: userData.presence || 'online',
                lastActiveAt: userData.lastActiveAt
            }
        } else if (self.value) {
            Object.assign(self.value, userData)
        }
    }

    function setUserPresence(userId: string, username: string, presence: Presence) {
        presenceMap.value.set(userId, {
            userId,
            username,
            presence,
            lastActiveAt: new Date().toISOString()
        })
    }

    function updatePresenceFromEvent(userId: string, presence: Presence) {
        if (self.value?.userId === userId) {
            self.value.presence = presence
        } else {
            const user = presenceMap.value.get(userId)
            if (user) {
                user.presence = presence
                user.lastActiveAt = new Date().toISOString()
            } else {
                // If we don't have the user yet, we might want to fetch or just ignore until they appear in a list
                presenceMap.value.set(userId, {
                    userId,
                    username: '', // Unknown for now
                    presence,
                    lastActiveAt: new Date().toISOString()
                })
            }
        }
    }

    function addTypingUser(userId: string, username: string, channelId: string, threadRootId?: string) {
        const key = `${channelId}:${threadRootId || 'root'}:${userId}`
        typingUsers.value.set(key, {
            userId,
            username,
            channelId,
            timestamp: Date.now(),
            threadRootId
        })
    }

    function removeTypingUser(userId: string, channelId: string, threadRootId?: string) {
        const key = `${channelId}:${threadRootId || 'root'}:${userId}`
        typingUsers.value.delete(key)
    }

    // Get typing users for main channel or specific thread
    function getTypingUsersForChannel(channelId: string, threadRootId?: string) {
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

    const getUserPresence = (userId: string) => {
        return computed(() => {
            if (self.value?.userId === userId) return self.value
            return presenceMap.value.get(userId)
        })
    }

    const onlineCount = computed(() => {
        let count = 0
        for (const user of presenceMap.value.values()) {
            if (user.presence === 'online') count++
        }
        if (self.value?.presence === 'online') count++
        return count
    })

    return {
        self,
        presenceMap,
        typingUsers,
        onlineCount,
        setSelfPresence,
        setUserPresence,
        updatePresenceFromEvent,
        addTypingUser,
        removeTypingUser,
        getTypingUsersForChannel,
        getUserPresence,
    }
})
