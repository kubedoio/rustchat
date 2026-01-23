import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { channelsApi, type Channel, type CreateChannelRequest } from '../api/channels'
import { useAuthStore } from './auth'

export const useChannelStore = defineStore('channels', () => {
    const channels = ref<Channel[]>([])
    const currentChannelId = ref<string | null>(null)
    const loading = ref(false)
    const error = ref<string | null>(null)

    const currentChannel = computed(() =>
        channels.value.find(c => c.id === currentChannelId.value) || null
    )

    const publicChannels = computed(() =>
        channels.value.filter(c => c.channel_type === 'public')
    )

    const privateChannels = computed(() =>
        channels.value.filter(c => c.channel_type === 'private')
    )

    const directMessages = computed(() =>
        channels.value.filter(c => c.channel_type === 'direct' || c.channel_type === 'group')
    )

    async function fetchChannels(teamId: string) {
        loading.value = true
        error.value = null
        try {
            const response = await channelsApi.list(teamId)
            channels.value = response.data
            // Auto-select general channel if none selected
            if (!currentChannelId.value && channels.value.length > 0) {
                const general = channels.value.find(c => c.name === 'general')
                currentChannelId.value = general?.id || channels.value[0]?.id || null
            }
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to fetch channels'
        } finally {
            loading.value = false
        }
    }

    async function createChannel(data: CreateChannelRequest) {
        loading.value = true
        error.value = null
        try {
            const response = await channelsApi.create(data)
            const channel = response.data

            addChannel(channel)

            return channel
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to create channel'
            throw e
        } finally {
            loading.value = false
        }
    }

    async function joinChannel(channelId: string) {
        const authStore = useAuthStore()
        if (!authStore.user?.id) {
            error.value = 'User not authenticated'
            return
        }

        try {
            await channelsApi.join(channelId, authStore.user.id)
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to join channel'
            throw e
        }
    }

    async function leaveChannel(channelId: string, userId: string) {
        try {
            await channelsApi.removeMember(channelId, userId)
            removeChannel(channelId)
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to leave channel'
            throw e
        }
    }

    function selectChannel(channelId: string) {
        currentChannelId.value = channelId
    }

    function updateChannel(updated: Channel) {
        const index = channels.value.findIndex(c => c.id === updated.id)
        if (index !== -1) {
            channels.value[index] = updated
        }
    }

    function removeChannel(channelId: string) {
        channels.value = channels.value.filter(c => c.id !== channelId)
        if (currentChannelId.value === channelId) {
            currentChannelId.value = channels.value[0]?.id || null
        }
    }

    function clearChannels() {
        channels.value = []
        currentChannelId.value = null
    }

    function incrementUnread(channelId: string) {
        const channel = channels.value.find(c => c.id === channelId)
        if (channel) {
            channel.unreadCount = (channel.unreadCount || 0) + 1
        }
    }

    function incrementMention(channelId: string) {
        const channel = channels.value.find(c => c.id === channelId)
        if (channel) {
            channel.mentionCount = (channel.mentionCount || 0) + 1
        }
    }

    function clearCounts(channelId: string) {
        const channel = channels.value.find(c => c.id === channelId)
        if (channel) {
            channel.unreadCount = 0
            channel.mentionCount = 0
        }
    }

    const joinableChannels = ref<Channel[]>([])

    async function fetchJoinableChannels(teamId: string) {
        loading.value = true
        try {
            const response = await channelsApi.listJoinable(teamId)
            joinableChannels.value = response.data
        } catch (e: any) {
            console.error('Failed to fetch joinable channels', e)
        } finally {
            loading.value = false
        }
    }

    function addChannel(channel: Channel) {
        // Deduplicate and reassign to ensure reactivity
        const channelMap = new Map(channels.value.map(c => [String(c.id), c]))
        channelMap.set(String(channel.id), channel)
        channels.value = Array.from(channelMap.values())
    }

    return {
        channels,
        joinableChannels,
        currentChannelId,
        currentChannel,
        publicChannels,
        privateChannels,
        directMessages,
        loading,
        error,
        fetchChannels,
        fetchJoinableChannels,
        createChannel,
        joinChannel,
        leaveChannel,
        selectChannel,
        updateChannel,
        removeChannel,
        clearChannels,
        incrementUnread,
        incrementMention,
        clearCounts,
        addChannel,
    }
})
