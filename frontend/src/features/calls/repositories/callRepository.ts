// Call Repository - Data access for calls
// Maps API responses to domain entities

import callsApi from '../../../api/calls'
import type { 
  CallState, 
  CallConfig, 
  CallParticipant, 
  SessionId
} from '../../../core/entities/Call'
import { createCallId, createSessionId } from '../../../core/entities/Call'
import type { ChannelId } from '../../../core/entities/Channel'
import type { UserId } from '../../../core/entities/User'
import { withRetry } from '../../../core/services/retry'
import { AppError } from '../../../core/errors/AppError'
import { isNotFoundError } from '../../../core/errors/errorUtils'


export interface CallChannelState {
  channelId: ChannelId
  enabled: boolean
  call?: CallState
}

export const callRepository = {
  // Config
  async getConfig(): Promise<CallConfig> {
    return withRetry(async () => {
      const response = await callsApi.getConfig()
      const data = response.data
      
      // If TURN is needed, fetch credentials
      let iceServers = data.ICEServersConfigs
      if (data.NeedsTURNCredentials) {
        try {
          const turnResponse = await callsApi.getTurnCredentials()
          iceServers = [
            ...data.ICEServersConfigs.filter(s => {
              const urls = Array.isArray(s.urls) ? s.urls : [s.urls]
              return !urls.some(url => url.toString().startsWith('turn:'))
            }),
            ...turnResponse.data
          ]
        } catch (error) {
          console.error('Failed to fetch TURN credentials', error)
        }
      }

      return {
        iceServers,
        allowEnableCalls: data.AllowEnableCalls,
        defaultEnabled: data.DefaultEnabled,
        needsTURNCredentials: data.NeedsTURNCredentials,
        maxParticipants: data.MaxCallParticipants,
        allowScreenSharing: data.AllowScreenSharing,
        enableSimulcast: data.EnableSimulcast,
        enableRinging: data.EnableRinging,
        enableLiveCaptions: data.EnableLiveCaptions,
        hostControlsAllowed: data.HostControlsAllowed,
        enableRecordings: data.EnableRecordings,
        maxRecordingDuration: data.MaxRecordingDuration,
        groupCallsAllowed: data.GroupCallsAllowed
      }
    })
  },

  // Active calls
  async getActiveCalls(): Promise<CallChannelState[]> {
    return withRetry(async () => {
      const response = await callsApi.getCalls()
      return response.data.map(channelState => ({
        channelId: channelState.channel_id as ChannelId,
        enabled: channelState.enabled,
        call: channelState.call ? normalizeCallState(channelState.call) : undefined
      }))
    })
  },

  async getCallForChannel(channelId: ChannelId): Promise<CallChannelState | null> {
    return withRetry(async () => {
      try {
        const response = await callsApi.getCallForChannel(channelId)
        if (!response.data) return null
        
        return {
          channelId: response.data.channel_id as ChannelId,
          enabled: response.data.enabled,
          call: response.data.call ? normalizeCallState(response.data.call) : undefined
        }
      } catch (error: unknown) {
        if (isNotFoundError(error)) {
          return null
        }
        throw error
      }
    })
  },

  // Call lifecycle
  async startCall(channelId: ChannelId): Promise<CallState> {
    return withRetry(async () => {
      await callsApi.startCall(channelId)
      
      // Fetch the full call state after starting
      const channelState = await this.getCallForChannel(channelId)
      if (!channelState?.call) {
        throw new AppError('Call started but state could not be loaded')
      }
      
      return channelState.call
    }, { maxAttempts: 2 })
  },

  async joinCall(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.joinCall(channelId))
  },

  async leaveCall(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.leaveCall(channelId))
  },

  async endCall(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.endCall(channelId))
  },

  // Media controls
  async mute(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.mute(channelId))
  },

  async unmute(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.unmute(channelId))
  },

  async raiseHand(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.raiseHand(channelId))
  },

  async lowerHand(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.lowerHand(channelId))
  },

  async toggleScreenShare(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.toggleScreenShare(channelId))
  },

  async sendReaction(channelId: ChannelId, emoji: string): Promise<void> {
    await withRetry(() => callsApi.sendReaction(channelId, emoji))
  },

  // WebRTC Signaling
  async sendOffer(channelId: ChannelId, sdp: string): Promise<{ sdp: string; type: string }> {
    const response = await withRetry(() => callsApi.sendOffer(channelId, sdp))
    return {
      sdp: response.data.sdp,
      type: response.data.type_
    }
  },

  async sendIceCandidate(
    channelId: ChannelId, 
    candidate: string, 
    sdpMid?: string, 
    sdpMLineIndex?: number
  ): Promise<void> {
    await withRetry(() => callsApi.sendIceCandidate(channelId, candidate, sdpMid, sdpMLineIndex))
  },

  // Host controls
  async hostMute(channelId: ChannelId, sessionId: SessionId): Promise<void> {
    await withRetry(() => callsApi.hostMute(channelId, sessionId))
  },

  async hostMuteOthers(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.hostMuteOthers(channelId))
  },

  async hostRemove(channelId: ChannelId, sessionId: SessionId): Promise<void> {
    await withRetry(() => callsApi.hostRemove(channelId, sessionId))
  },

  async hostMakeHost(channelId: ChannelId, newHostId: UserId): Promise<void> {
    await withRetry(() => callsApi.hostMakeHost(channelId, newHostId))
  },

  // Ringing
  async ringUsers(channelId: ChannelId): Promise<void> {
    await withRetry(() => callsApi.ringUsers(channelId))
  }
}

// Normalize API CallState to domain entity
function normalizeCallState(raw: unknown): CallState {
  const r = raw as Record<string, unknown>
  const participants = new Map<SessionId, CallParticipant>()
  
  if (raw.sessions) {
    for (const [key, session] of Object.entries(raw.sessions)) {
      const s = session as Record<string, unknown>
      const sessionId = createSessionId(s.session_id || key)
      participants.set(sessionId, {
        sessionId,
        userId: (s.user_id_raw || s.user_id) as UserId,
        username: s.username || '',
        displayName: s.display_name,
        isMuted: !s.unmuted,
        isSpeaking: false, // Will be updated by WebSocket events
        isScreenSharing: false, // Will be updated by WebSocket events
        raisedHandAt: s.raised_hand || 0,
        joinedAt: new Date(raw.start_at || Date.now())
      })
    }
  }

  return {
    id: createCallId((r.id || r.id_raw || '') as string),
    channelId: ((r.channel_id_raw || r.channel_id) as string) as ChannelId,
    startedAt: new Date((r.start_at || Date.now()) as string | number),
    startedBy: ((r.owner_id_raw || r.owner_id) as string) as UserId,
    hostId: ((r.host_id_raw || r.host_id) as string) as UserId,
    participants,
    screenSharingSessionId: r.screen_sharing_session_id 
      ? createSessionId(r.screen_sharing_session_id as string)
      : undefined,
    threadId: r.thread_id as string | undefined,
    recording: r.recording ? {
      isRecording: true,
      startedAt: new Date((r.recording as Record<string, unknown>).start_at as string | number),
      startedBy: r.owner_id as UserId
    } : undefined
  }
}
