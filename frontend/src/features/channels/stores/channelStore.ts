import { log } from '@/utils/log';
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useStorage } from '@vueuse/core'
import { channelsApi, type CreateChannelRequest, type ChannelNotifyProps } from '../../../api/channels'
import type { ChannelId } from '../../../core/entities/Channel'
import { getApiErrorMessage } from '../../../core/errors/errorUtils'

export const useChannelStore = defineStore('channelStore', () => {
  // Internal Map for feature architecture compatibility
  const _channelsMap = ref<Map<ChannelId, any>>(new Map())
  const joinableChannels = ref<any[]>([])
  const currentChannelId = ref<ChannelId | null>(null)
  const lastChannelByTeam = useStorage<Record<string, string>>('last_channel_by_team', {})
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Legacy-compatible computed: channels as Array
  const channels = computed(() => Array.from(_channelsMap.value.values()))

  const currentChannel = computed(() =>
    channels.value.find(c => c.id === currentChannelId.value) || null
  )

  const publicChannels = computed(() =>
    channels.value.filter(c => c.channel_type === 'public' || (c as any).type === 'public')
  )

  const privateChannels = computed(() =>
    channels.value.filter(c => c.channel_type === 'private' || (c as any).type === 'private')
  )

  const directMessages = computed(() =>
    channels.value.filter(c =>
      c.channel_type === 'direct' || c.channel_type === 'group' ||
      (c as any).type === 'direct' || (c as any).type === 'group'
    )
  )

  // Feature actions
  function setChannels(items: any[]) {
    _channelsMap.value.clear()
    for (const channel of items) {
      _channelsMap.value.set(channel.id, channel)
    }
  }

  function addChannel(channel: any) {
    _channelsMap.value.set(channel.id, channel)
  }

  function updateChannel(channel: any) {
    const existing = _channelsMap.value.get(channel.id)
    if (existing) {
      _channelsMap.value.set(channel.id, { ...existing, ...channel })
    }
  }

  function removeChannel(channelId: ChannelId) {
    _channelsMap.value.delete(channelId)
    if (currentChannelId.value === channelId) {
      currentChannelId.value = channels.value[0]?.id || null
    }
  }

  function setCurrentChannelId(channelId: ChannelId | null) {
    currentChannelId.value = channelId
  }

  function setJoinableChannels(items: any[]) {
    joinableChannels.value = items
  }

  function setUnreadCounts(counts: { channelId: ChannelId; unreadCount: number; mentionCount: number }[]) {
    for (const { channelId, unreadCount, mentionCount } of counts) {
      const channel = _channelsMap.value.get(channelId)
      if (channel) {
        channel.unreadCount = unreadCount
        channel.mentionCount = mentionCount
      }
    }
  }

  function incrementUnread(channelId: ChannelId) {
    const channel = _channelsMap.value.get(channelId)
    if (channel) {
      channel.unreadCount = (channel.unreadCount || 0) + 1
    }
  }

  function incrementMention(channelId: ChannelId) {
    const channel = _channelsMap.value.get(channelId)
    if (channel) {
      channel.mentionCount = (channel.mentionCount || 0) + 1
    }
  }

  function clearCounts(channelId: ChannelId) {
    const channel = _channelsMap.value.get(channelId)
    if (channel) {
      channel.unreadCount = 0
      channel.mentionCount = 0
    }
  }

  function setLoading(value: boolean) {
    loading.value = value
  }

  function setError(err: string | null) {
    error.value = err
  }

  function clearError() {
    error.value = null
  }

  function clearChannels() {
    _channelsMap.value.clear()
    currentChannelId.value = null
    joinableChannels.value = []
  }

  function getChannelById(channelId: ChannelId): any | undefined {
    return _channelsMap.value.get(channelId)
  }

  // Legacy async methods
  async function fetchChannels(teamId: string) {
    loading.value = true
    error.value = null
    try {
      const response = await channelsApi.list(teamId)
      _channelsMap.value.clear()
      for (const channel of response.data) {
        _channelsMap.value.set(channel.id, channel)
      }

      // Try to restore last selected channel for this team
      const lastId = lastChannelByTeam.value[teamId]
      if (lastId && channels.value.some(c => c.id === lastId)) {
        currentChannelId.value = lastId
      } else {
        // Auto-select general channel if none selected or last not found
        const general = channels.value.find(c => c.name === 'general')
        currentChannelId.value = general?.id || channels.value[0]?.id || null

        // Save this default selection
        if (currentChannelId.value) {
          lastChannelByTeam.value[teamId] = currentChannelId.value
        }
      }
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to fetch channels'
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
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to create channel'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function joinChannel(channelId: string) {
    // Use a simple user id approach - legacy didn't pass userId here consistently
    try {
      await channelsApi.join(channelId, 'me')
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to join channel'
      throw e
    }
  }

  async function leaveChannel(channelId: string, userId: string) {
    try {
      await channelsApi.removeMember(channelId, userId)
      removeChannel(channelId)
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to leave channel'
      throw e
    }
  }

  function selectChannel(channelId: string) {
    currentChannelId.value = channelId
    const channel = _channelsMap.value.get(channelId)
    if (channel && (channel as any).team_id) {
      lastChannelByTeam.value[(channel as any).team_id] = channelId
    }
  }

  async function fetchJoinableChannels(teamId: string) {
    loading.value = true
    try {
      const response = await channelsApi.listJoinable(teamId)
      joinableChannels.value = response.data
    } catch (e: unknown) {
      log.error('Failed to fetch joinable channels', e)
    } finally {
      loading.value = false
    }
  }

  async function updateNotifyProps(channelId: string, _userId: string, props: ChannelNotifyProps) {
    try {
      await channelsApi.updateNotifyProps(channelId, _userId, props)
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to update notification settings'
      throw e
    }
  }

  return {
    // State
    channels,
    joinableChannels,
    currentChannelId,
    currentChannel,
    publicChannels,
    privateChannels,
    directMessages,
    loading,
    error,

    // Feature actions
    setChannels,
    addChannel,
    updateChannel,
    removeChannel,
    setCurrentChannelId,
    setJoinableChannels,
    setUnreadCounts,
    incrementUnread,
    incrementMention,
    clearCounts,
    setLoading,
    setError,
    clearError,
    clearChannels,
    getChannelById,

    // Legacy actions
    fetchChannels,
    fetchJoinableChannels,
    createChannel,
    joinChannel,
    leaveChannel,
    selectChannel,
    updateNotifyProps
  }
})
