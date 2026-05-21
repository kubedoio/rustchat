// Channel WebSocket Handlers - Feature-specific channel event handling

import { log } from '@/utils/log'
import { channelService } from '../services/channelService'
import type { Channel, ChannelId } from '../../../core/entities/Channel'
import type { UserId } from '../../../core/entities/User'

export interface WebSocketChannelEvent {
  event: string
  data: string
  broadcast: {
    channel_id: string
    user_id: string
  }
}

export function handleChannelWebSocketEvent(event: WebSocketChannelEvent) {
  switch (event.event) {
    case 'channel_created':
      handleChannelCreated(event)
      break
    case 'channel_updated':
      handleChannelUpdated(event)
      break
    case 'channel_deleted':
      handleChannelDeleted(event)
      break
    case 'user_added':
      handleUserAdded(event)
      break
    case 'user_removed':
      handleUserRemoved(event)
      break
    case 'channel_viewed':
      handleChannelViewed(event)
      break
  }
}

// Helper to read event data safely
function readEventData(event: WebSocketChannelEvent): Record<string, unknown> {
  try {
    return JSON.parse(event.data) as Record<string, unknown>
  } catch {
    return {}
  }
}

function readEventChannelId(data: Record<string, unknown>): ChannelId | undefined {
  return (data.channel_id || data.channel_id_raw) as string | undefined as ChannelId | undefined
}

function readEventUserId(data: Record<string, unknown>): UserId | undefined {
  return (data.user_id || data.user_id_raw) as string | undefined as UserId | undefined
}

// Event handlers
function handleChannelCreated(event: WebSocketChannelEvent) {
  log.debug('Channel created:', event)
  const data = readEventData(event)

  if (!data.channel_id) return

  const channel = normalizeChannel(data)
  channelService.handleChannelCreated(channel)
}

function handleChannelUpdated(event: WebSocketChannelEvent) {
  log.debug('Channel updated:', event)
  const data = readEventData(event)

  if (!data.channel_id) return

  const channel = normalizeChannel(data)
  channelService.handleChannelUpdated(channel)
}

function handleChannelDeleted(event: WebSocketChannelEvent) {
  log.debug('Channel deleted:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)

  if (channelId) {
    channelService.handleChannelDeleted(channelId)
  }
}

function handleUserAdded(event: WebSocketChannelEvent) {
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)

  if (channelId && userId) {
    channelService.handleUserJoined(channelId, userId)
    // Refresh channel to get updated member count
    if (typeof data.team_id === 'string') {
      void channelService.loadChannels(data.team_id)
    }
  }
}

function handleUserRemoved(event: WebSocketChannelEvent) {
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)

  if (channelId && userId) {
    channelService.handleUserLeft(channelId, userId)
    // Refresh channel to get updated member count
    if (typeof data.team_id === 'string') {
      void channelService.loadChannels(data.team_id)
    }
  }
}

function handleChannelViewed(event: WebSocketChannelEvent) {
  const data = readEventData(event)
  const channelId = readEventChannelId(data)

  if (channelId) {
    // Clear unread counts for this channel
    // This happens when the user views the channel on another device
    channelService.handleNewMessage(channelId, false)
  }
}

// Normalize WebSocket channel data to domain entity
function normalizeChannel(data: Record<string, unknown>): Channel {
  // Handle both full channel objects and event data with channel_id
  const channelData = (data.channel as Record<string, unknown> | undefined) || data

  return {
    id: channelData.id || channelData.channel_id || data.channel_id,
    teamId: channelData.team_id || data.team_id,
    name: channelData.name || channelData.channel_name || data.channel_name,
    displayName:
      channelData.display_name || channelData.channel_display_name || data.channel_display_name,
    type: channelData.type || channelData.channel_type || data.channel_type,
    purpose: channelData.purpose,
    header: channelData.header,
    creatorId: channelData.creator_id || data.creator_id,
    createdAt: channelData.created_at
      ? new Date(channelData.created_at as string | number)
      : channelData.create_at
        ? new Date(channelData.create_at as string | number)
        : new Date(),
    updatedAt: channelData.updated_at
      ? new Date(channelData.updated_at as string | number)
      : channelData.update_at
        ? new Date(channelData.update_at as string | number)
        : new Date(),
    isArchived: Boolean(channelData.deleted_at || channelData.delete_at),
    memberCount: channelData.member_count || data.member_count,
  } as Channel
}
