// Unread Store - Backwards-compatible state management for unread counts

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '../../../api/client'
import { channelsApi } from '../../../api/channels'
import { postsApi, type ChannelUnreadAt } from '../../../api/posts'

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
    last_read_message_id: string | number | null
    first_unread_message_id: string | number | null
}

export const useUnreadStore = defineStore('unreadStore', () => {
    // State (legacy shape using Records)
    const channelUnreads = ref<Record<string, number>>({})
    const teamUnreads = ref<Record<string, number>>({})
    const channelMentions = ref<Record<string, number>>({})
    const channelReadStates = ref<Record<string, ReadState>>({})

    const loading = ref(false)

    function clearAllState() {
        channelUnreads.value = {}
        teamUnreads.value = {}
        channelMentions.value = {}
        channelReadStates.value = {}
    }

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

    async function markAsRead(channelId: string, userId: string = 'me') {
        try {
            await channelsApi.markAsRead(channelId, userId)

            // Optimistic update
            channelUnreads.value[channelId] = 0
            channelMentions.value[channelId] = 0

            // Clear the "new messages" line state locally too
            if (channelReadStates.value[channelId]) {
                channelReadStates.value[channelId] = {
                    last_read_message_id: null,
                    first_unread_message_id: null
                }
            }
        } catch (error) {
            console.error('Failed to mark channel as read:', error)
        }
    }

    async function markAsUnread(channelId: string, userId: string = 'me') {
        try {
            await channelsApi.markAsUnread(channelId, userId)

            // Optimistic update - set as having unread
            channelUnreads.value[channelId] = 1

            // Refresh overview to get accurate counts
            await fetchOverview()
        } catch (error) {
            console.error('Failed to mark channel as unread:', error)
        }
    }

    async function markAsUnreadFromPost(postId: string, userId: string = 'me') {
        try {
            const { data } = await postsApi.setUnreadFromPost(userId, postId, {
                collapsed_threads_supported: true,
            })
            applyPostUnread(data)
        } catch (error) {
            console.error('Failed to mark post as unread:', error)
            throw error
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
    }

    function applyPostUnread(data: ChannelUnreadAt) {
        channelMentions.value[data.channel_id] = Number.isFinite(data.mention_count) ? data.mention_count : 0
        // Mattermost post_unread/set_unread returns msg_count as read-position counter.
        // Keep unread counters authoritative from overview/unread events.
        void fetchOverview()
    }

    const totalUnreadCount = computed(() => Object.values(channelUnreads.value).reduce((a, b) => a + b, 0))
    const totalMentionCount = computed(() => Object.values(channelMentions.value).reduce((a, b) => a + b, 0))
    const getChannelUnreadCount = computed(() => (channelId: string) => channelUnreads.value[channelId] || 0)
    const getTeamUnreadCount = computed(() => (teamId: string) => teamUnreads.value[teamId] || 0)
    const getChannelReadState = computed(() => (channelId: string) => channelReadStates.value[channelId])

    // Feature-compatible setters (using Record-based state)
    function getChannelUnread(channelId: string): number {
        return channelUnreads.value[channelId] || 0
    }

    function getChannelMentions(channelId: string): number {
        return channelMentions.value[channelId] || 0
    }

    function getTeamUnread(teamId: string): number {
        return teamUnreads.value[teamId] || 0
    }

    function getChannelReadStateValue(channelId: string): ReadState | undefined {
        return channelReadStates.value[channelId]
    }

    function setChannelUnread(channelId: string, count: number) {
        channelUnreads.value[channelId] = count
    }

    function setChannelMentions(channelId: string, count: number) {
        channelMentions.value[channelId] = count
    }

    function setTeamUnread(teamId: string, count: number) {
        teamUnreads.value[teamId] = count
    }

    function clearChannel(channelId: string) {
        delete channelUnreads.value[channelId]
        delete channelMentions.value[channelId]
        delete channelReadStates.value[channelId]
    }

    function clearAll() {
        channelUnreads.value = {}
        channelMentions.value = {}
        teamUnreads.value = {}
        channelReadStates.value = {}
    }

    function setLoading(value: boolean) {
        loading.value = value
    }

    return {
        // State
        channelUnreads,
        teamUnreads,
        channelMentions,
        channelReadStates,
        loading,
        // Actions
        clearAllState,
        fetchOverview,
        markAsRead,
        markAsUnread,
        markAsUnreadFromPost,
        markAllAsRead,
        setReadState,
        handleUnreadUpdate,
        applyPostUnread,
        // Getters
        totalUnreadCount,
        totalMentionCount,
        getChannelUnreadCount,
        getTeamUnreadCount,
        getChannelReadState,
        // Feature-compatible setters
        getChannelUnread,
        getChannelMentions,
        getTeamUnread,
        getChannelReadStateValue,
        setChannelUnread,
        setChannelMentions,
        setTeamUnread,
        clearChannel,
        clearAll,
        setLoading
    }
})
