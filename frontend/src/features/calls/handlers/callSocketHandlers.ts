// Call WebSocket Handlers - Feature-specific call event handling
// Replaces the centralized call event handling from useWebSocket.ts

import { log } from '@/utils/log';
import { callService } from '../services/callService'
import type { ChannelId } from '../../../core/entities/Channel'
import type { UserId } from '../../../core/entities/User'
import type { SessionId } from '../../../core/entities/Call'

export interface WebSocketCallEvent {
  event: string
  data: string
  broadcast: {
    channel_id: string
    user_id: string
  }
}

export function handleCallWebSocketEvent(event: WebSocketCallEvent) {
  switch (event.event) {
    case 'custom_com.mattermost.calls_call_start':
      handleCallStart(event)
      break
    case 'custom_com.mattermost.calls_call_end':
      handleCallEnd(event)
      break
    case 'custom_com.mattermost.calls_user_joined':
      handleUserJoined(event)
      break
    case 'custom_com.mattermost.calls_user_left':
      handleUserLeft(event)
      break
    case 'custom_com.mattermost.calls_user_muted':
      handleUserMuted(event)
      break
    case 'custom_com.mattermost.calls_user_unmuted':
      handleUserUnmuted(event)
      break
    case 'custom_com.mattermost.calls_raise_hand':
      handleRaiseHand(event)
      break
    case 'custom_com.mattermost.calls_lower_hand':
      handleLowerHand(event)
      break
    case 'custom_com.mattermost.calls_user_voice_on':
      handleVoiceOn(event)
      break
    case 'custom_com.mattermost.calls_user_voice_off':
      handleVoiceOff(event)
      break
    case 'custom_com.mattermost.calls_host_mute':
      handleHostMute(event)
      break
    case 'custom_com.mattermost.calls_host_removed':
      handleHostRemoved(event)
      break
    case 'custom_com.mattermost.calls_host_changed':
      handleHostChanged(event)
      break
    case 'custom_com.mattermost.calls_ringing':
      handleRinging(event)
      break
    case 'custom_com.mattermost.calls_screen_on':
      handleScreenOn(event)
      break
    case 'custom_com.mattermost.calls_screen_off':
      handleScreenOff(event)
      break
    case 'custom_com.mattermost.calls_signal':
      handleSignal(event)
      break
  }
}

// Helper to read event data safely
function readEventData(event: WebSocketCallEvent): Record<string, unknown> {
  try {
    return JSON.parse(event.data) as Record<string, unknown>
  } catch {
    return {}
  }
}

function readEventChannelId(data: Record<string, unknown>): ChannelId | undefined {
  return ((data.channel_id_raw || data.channel_id) as string | undefined) as ChannelId | undefined
}

function readEventUserId(data: Record<string, unknown>): UserId | undefined {
  return ((data.user_id_raw || data.user_id) as string | undefined) as UserId | undefined
}

function readEventSessionId(data: Record<string, unknown>): SessionId | undefined {
  const id = (data.session_id_raw || data.session_id) as string | undefined
  return id ? (id as SessionId) : undefined
}

// Event handlers
function handleCallStart(event: WebSocketCallEvent) {
  log.debug('Call started:', event)
  // Reload calls to get updated state
  void callService.loadActiveCalls()
}

function handleCallEnd(event: WebSocketCallEvent) {
  log.debug('Call ended:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  if (channelId) {
    callService.handleCallEnded(channelId)
  }
}

function handleUserJoined(event: WebSocketCallEvent) {
  log.debug('User joined call:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  if (channelId) {
    callService.handleUserJoined(channelId)
  }
}

function handleUserLeft(event: WebSocketCallEvent) {
  log.debug('User left call:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  if (channelId) {
    callService.handleUserLeft(channelId)
  }
}

function handleUserMuted(event: WebSocketCallEvent) {
  log.debug('User muted:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)
  if (channelId && userId) {
    callService.handleUserMuted(channelId, userId, true)
  }
}

function handleUserUnmuted(event: WebSocketCallEvent) {
  log.debug('User unmuted:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)
  if (channelId && userId) {
    callService.handleUserMuted(channelId, userId, false)
  }
}

function handleRaiseHand(event: WebSocketCallEvent) {
  log.debug('Hand raised:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)
  if (channelId && userId) {
    callService.handleHandRaised(channelId, userId)
  }
}

function handleLowerHand(event: WebSocketCallEvent) {
  log.debug('Hand lowered:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)
  if (channelId && userId) {
    callService.handleHandLowered(channelId, userId)
  }
}

function handleVoiceOn(event: WebSocketCallEvent) {
  const data = readEventData(event)
  const sessionId = readEventSessionId(data)
  if (sessionId) {
    callService.handleVoiceOn(sessionId)
  }
}

function handleVoiceOff(event: WebSocketCallEvent) {
  const data = readEventData(event)
  const sessionId = readEventSessionId(data)
  if (sessionId) {
    callService.handleVoiceOff(sessionId)
  }
}

function handleHostMute(event: WebSocketCallEvent) {
  const data = readEventData(event)
  const sessionId = readEventSessionId(data)
  if (sessionId) {
    callService.handleHostMuted(sessionId)
  }
}

function handleHostRemoved(event: WebSocketCallEvent) {
  const data = readEventData(event)
  const sessionId = readEventSessionId(data)
  if (sessionId) {
    callService.handleHostRemoved(sessionId)
  }
}

function handleHostChanged(event: WebSocketCallEvent) {
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const hostId = (data.host_id || data.host_id_raw) as string | undefined
  if (channelId && hostId) {
    callService.handleHostChanged(channelId, hostId as UserId)
  }
}

function handleRinging(event: WebSocketCallEvent) {
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const callerId = (data.sender_id || data.sender_id_raw) as string | undefined as UserId | undefined
  if (channelId && callerId) {
    callService.handleIncomingCall(channelId, callerId)
  }
}

function handleScreenOn(event: WebSocketCallEvent) {
  log.debug('Screen share on:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)
  if (channelId && userId) {
    callService.handleScreenShareOn(channelId, userId)
  }
}

function handleScreenOff(event: WebSocketCallEvent) {
  log.debug('Screen share off:', event)
  const data = readEventData(event)
  const channelId = readEventChannelId(data)
  const userId = readEventUserId(data)
  if (channelId && userId) {
    callService.handleScreenShareOff(channelId, userId)
  }
}

function handleSignal(event: WebSocketCallEvent) {
  const data = readEventData(event)
  callService.handleSignalingEvent(data)
}
