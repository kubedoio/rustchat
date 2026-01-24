import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '../api/client'

export interface ChannelUnread {
    channel_id: string
    team_id: string
    unread_count: number
    mention_count: number
}

export interface TeamUnread {
    team_id: string
    unread_count: number
}

export interface UnreadOverview {
    channels: ChannelUnread[]
    teams: TeamUnread[]
}

export interface ReadState {
    last_read_message_id: number | null
    first_unread_message_id: number | null
}

export const useUnreadStore = defineStore('unreads', () => {
    const channelUnreads = ref<Record<string, number>>({})
    const teamUnreads = ref<Record<string, number>>({})
    const channelMentions = ref<Record<string, number>>({})
    const channelReadStates = ref<Record<string, ReadState>>({})

    const loading = ref(false)

    async function fetchOverview() {
        loading.value = true
        try {
            const response = await api.get<UnreadOverview>('/unreads/overview')
            const { channels, teams } = response.data

            // Reset and populate
            channelUnreads.value = {}
            teamUnreads.value = {}

            channels.forEach(c => {
                channelUnreads.value[c.channel_id] = c.unread_count
                channelMentions.value[c.channel_id] = c.mention_count || 0
            })

            teams.forEach(t => {
                teamUnreads.value[t.team_id] = t.unread_count
            })
        } catch (error) {
            console.error('Failed to fetch unread overview:', error)
        } finally {
            loading.value = false
        }
    }

    async function markAsRead(channelId: string, targetSeq?: string | number | null) {
        try {
            await api.post(`/channels/${channelId}/read`, { target_seq: targetSeq })

            // Optimistic update for standard "mark channel as read" (no targetSeq)
            if (!targetSeq) {
                channelUnreads.value[channelId] = 0
                channelMentions.value[channelId] = 0

                // Clear the "new messages" line state locally too
                if (channelReadStates.value[channelId]) {
                    channelReadStates.value[channelId] = {
                        last_read_message_id: null,
                        first_unread_message_id: null
                    }
                }
            } else {
                // If targetSeq is provided, it's usually "Mark as unread from here"
                // We could re-calculate locally but simpler to let the WS event or next fetch handle it.
                // For now, let's just update the local state if we can.
            }

            // We might want to re-fetch overview to update team counts correctly
            // or we could try to calculate it if we have team_id mapping.
        } catch (error) {
            console.error('Failed to mark channel as read:', error)
        }
    }

    async function markAllAsRead() {
        try {
            await api.post('/unreads/mark_all_read')
            channelUnreads.value = {}
            teamUnreads.value = {}
            channelMentions.value = {}
        } catch (error) {
            console.error('Failed to mark all as read:', error)
        }
    }

    function setReadState(channelId: string, state: ReadState) {
        channelReadStates.value[channelId] = state
    }

    function handleUnreadUpdate(data: { channel_id: string; team_id: string; unread_count: number }) {
        channelUnreads.value[data.channel_id] = data.unread_count
        // Team unread count update: if we want to be accurate we should probably re-fetch or track team mappings
    }

    const totalUnreadCount = computed(() => Object.values(channelUnreads.value).reduce((a, b) => a + b, 0))
    const getChannelUnreadCount = computed(() => (channelId: string) => channelUnreads.value[channelId] || 0)
    const getTeamUnreadCount = computed(() => (teamId: string) => teamUnreads.value[teamId] || 0)
    const getChannelReadState = computed(() => (channelId: string) => channelReadStates.value[channelId])

    return {
        channelUnreads,
        teamUnreads,
        channelMentions,
        channelReadStates,
        loading,
        fetchOverview,
        markAsRead,
        markAllAsRead,
        setReadState,
        handleUnreadUpdate,
        totalUnreadCount,
        getChannelUnreadCount,
        getTeamUnreadCount,
        getChannelReadState,
    }
})
