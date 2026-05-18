import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { preferencesApi, type Preference } from '../../../api/preferences'
import { channelsApi, type ChannelNotifyProps } from '../../../api/channels'
import { useAuthStore } from '../../../stores/auth'

export const useChannelPreferencesStore = defineStore('channelPreferences', () => {
    const favoriteChannels = ref<Set<string>>(new Set())
    const mutedChannels = ref<Set<string>>(new Set())
    const channelNotifyProps = ref<Record<string, ChannelNotifyProps>>({})
    const loading = ref(false)

    // Check if a channel is favorited
    const isFavorite = computed(() => (channelId: string) => 
        favoriteChannels.value.has(channelId)
    )

    // Check if a channel is muted
    const isMuted = computed(() => (channelId: string) => 
        mutedChannels.value.has(channelId)
    )

    // Get notify props for a channel
    const getNotifyProps = computed(() => (channelId: string) => 
        channelNotifyProps.value[channelId] || {}
    )

    // Fetch all preferences including favorites
    async function fetchPreferences() {
        const authStore = useAuthStore()
        if (!authStore.user?.id) return

        loading.value = true
        try {
            const response = await preferencesApi.getMyPreferencesMm()
            const prefs = response.data

            // Reset sets
            favoriteChannels.value.clear()
            mutedChannels.value.clear()

            // Defensive: ensure prefs is an array
            const prefsArray = Array.isArray(prefs) ? prefs : []
            
            prefsArray.forEach((pref: Preference) => {
                if (pref.category === 'favorite_channel' && pref.value === 'true') {
                    favoriteChannels.value.add(pref.name)
                }
                if (pref.category === 'channel_mute' && pref.value === 'true') {
                    mutedChannels.value.add(pref.name)
                }
            })
        } catch (error) {
            console.error('Failed to fetch channel preferences:', error)
        } finally {
            loading.value = false
        }
    }

    // Favorite a channel
    async function favoriteChannel(channelId: string) {
        const authStore = useAuthStore()
        if (!authStore.user?.id) return

        try {
            const pref: Preference = {
                user_id: authStore.user.id,
                category: 'favorite_channel',
                name: channelId,
                value: 'true'
            }
            await preferencesApi.updatePreferences(authStore.user.id, [pref])
            favoriteChannels.value.add(channelId)
        } catch (error) {
            console.error('Failed to favorite channel:', error)
            throw error
        }
    }

    // Unfavorite a channel
    async function unfavoriteChannel(channelId: string) {
        const authStore = useAuthStore()
        if (!authStore.user?.id) return

        try {
            const pref: Preference = {
                user_id: authStore.user.id,
                category: 'favorite_channel',
                name: channelId,
                value: 'true'
            }
            await preferencesApi.deletePreferences(authStore.user.id, [pref])
            favoriteChannels.value.delete(channelId)
        } catch (error) {
            console.error('Failed to unfavorite channel:', error)
            throw error
        }
    }

    // Toggle favorite status
    async function toggleFavorite(channelId: string) {
        if (favoriteChannels.value.has(channelId)) {
            await unfavoriteChannel(channelId)
        } else {
            await favoriteChannel(channelId)
        }
    }

    // Mute a channel (set notify_props.mark_unread to 'mention')
    async function muteChannel(channelId: string) {
        const authStore = useAuthStore()
        if (!authStore.user?.id) return

        try {
            const props: ChannelNotifyProps = {
                desktop: 'none',
                mobile: 'none',
                mark_unread: 'mention',
                ignore_channel_mentions: 'on'
            }
            await channelsApi.updateNotifyProps(channelId, authStore.user.id, props)
            mutedChannels.value.add(channelId)
            channelNotifyProps.value[channelId] = props
        } catch (error) {
            console.error('Failed to mute channel:', error)
            throw error
        }
    }

    // Unmute a channel (reset notify_props)
    async function unmuteChannel(channelId: string) {
        const authStore = useAuthStore()
        if (!authStore.user?.id) return

        try {
            const props: ChannelNotifyProps = {
                desktop: 'default',
                mobile: 'default',
                mark_unread: 'all',
                ignore_channel_mentions: 'off'
            }
            await channelsApi.updateNotifyProps(channelId, authStore.user.id, props)
            mutedChannels.value.delete(channelId)
            channelNotifyProps.value[channelId] = props
        } catch (error) {
            console.error('Failed to unmute channel:', error)
            throw error
        }
    }

    // Toggle mute status
    async function toggleMute(channelId: string) {
        if (mutedChannels.value.has(channelId)) {
            await unmuteChannel(channelId)
        } else {
            await muteChannel(channelId)
        }
    }

    // Fetch notify props for a specific channel
    async function fetchChannelNotifyProps(channelId: string) {
        const authStore = useAuthStore()
        if (!authStore.user?.id) return

        try {
            const response = await channelsApi.getMember(channelId, authStore.user.id)
            if (response.data?.notify_props) {
                channelNotifyProps.value[channelId] = response.data.notify_props
                // Check if muted based on props
                if (response.data.notify_props.mark_unread === 'mention') {
                    mutedChannels.value.add(channelId)
                } else {
                    mutedChannels.value.delete(channelId)
                }
            }
        } catch (error) {
            console.error('Failed to fetch channel notify props:', error)
        }
    }

    function clearState() {
        favoriteChannels.value.clear()
        mutedChannels.value.clear()
        channelNotifyProps.value = {}
    }

    return {
        favoriteChannels,
        mutedChannels,
        channelNotifyProps,
        loading,
        isFavorite,
        isMuted,
        getNotifyProps,
        fetchPreferences,
        favoriteChannel,
        unfavoriteChannel,
        toggleFavorite,
        muteChannel,
        unmuteChannel,
        toggleMute,
        fetchChannelNotifyProps,
        clearState
    }
})
