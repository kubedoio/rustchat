import { defineStore } from 'pinia'
import { reactive, computed } from 'vue'

export interface UnreadState {
    channelId: string
    count: number
    mentionCount: number
    lastReadAt: string
}

export const useUnreadStore = defineStore('unreads', () => {
    // Unread counts per channel: channelId -> UnreadState
    const unreads = reactive(new Map<string, UnreadState>())

    function setUnread(channelId: string, count: number, mentionCount = 0) {
        unreads.set(channelId, {
            channelId,
            count,
            mentionCount,
            lastReadAt: new Date().toISOString(),
        })
    }

    function markAsRead(channelId: string) {
        const existing = unreads.get(channelId)
        if (existing) {
            unreads.set(channelId, {
                ...existing,
                count: 0,
                mentionCount: 0,
                lastReadAt: new Date().toISOString(),
            })
        }
    }

    function incrementUnread(channelId: string, hasMention = false) {
        const existing = unreads.get(channelId)
        if (existing) {
            unreads.set(channelId, {
                ...existing,
                count: existing.count + 1,
                mentionCount: hasMention ? existing.mentionCount + 1 : existing.mentionCount,
            })
        } else {
            unreads.set(channelId, {
                channelId,
                count: 1,
                mentionCount: hasMention ? 1 : 0,
                lastReadAt: new Date().toISOString(),
            })
        }
    }

    function getUnreadCount(channelId: string) {
        return computed(() => unreads.get(channelId)?.count || 0)
    }

    function getMentionCount(channelId: string) {
        return computed(() => unreads.get(channelId)?.mentionCount || 0)
    }

    function hasUnread(channelId: string) {
        return computed(() => (unreads.get(channelId)?.count || 0) > 0)
    }

    const totalUnreadCount = computed(() => {
        let total = 0
        for (const state of unreads.values()) {
            total += state.count
        }
        return total
    })

    const totalMentionCount = computed(() => {
        let total = 0
        for (const state of unreads.values()) {
            total += state.mentionCount
        }
        return total
    })

    function clearUnreads() {
        unreads.clear()
    }

    return {
        unreads,
        totalUnreadCount,
        totalMentionCount,
        setUnread,
        markAsRead,
        incrementUnread,
        getUnreadCount,
        getMentionCount,
        hasUnread,
        clearUnreads,
    }
})
