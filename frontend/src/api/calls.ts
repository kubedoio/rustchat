// Mattermost Calls Plugin API Client
// Routes are mounted under /api/v4/plugins/com.mattermost.calls

import { v4Api } from './client'

// Types from Mattermost Calls
export interface CallsConfig {
  ICEServersConfigs: RTCIceServer[]
  AllowEnableCalls: boolean
  DefaultEnabled: boolean
  NeedsTURNCredentials: boolean
  MaxCallParticipants: number
  AllowScreenSharing: boolean
  EnableSimulcast: boolean
  EnableRinging: boolean
  EnableLiveCaptions: boolean
  HostControlsAllowed: boolean
  EnableRecordings: boolean
  MaxRecordingDuration: number
  GroupCallsAllowed: boolean
}

export interface CallsVersionInfo {
  version?: string
  rtcd?: boolean
}

export interface CallState {
  id: string
  id_raw: string
  channel_id: string
  channel_id_raw: string
  start_at: number
  owner_id: string
  owner_id_raw: string
  host_id: string
  host_id_raw: string
  thread_id?: string
  screen_sharing_id?: string
  screen_sharing_id_raw?: string
  screen_sharing_session_id?: string
  recording?: CallJobState
  dismissed_notification?: Record<string, boolean>
  sessions: Record<string, CallSession>
}

export interface CallSession {
  session_id: string
  session_id_raw?: string
  user_id: string
  user_id_raw?: string
  username?: string
  display_name?: string
  unmuted: boolean
  raised_hand: number
}

export interface CallJobState {
  start_at: number
  end_at: number
}

export interface CallChannelState {
  channel_id: string
  enabled: boolean
  call?: CallState
}

export interface StartCallResponse {
  id: string
  channel_id: string
  start_at: number
  owner_id: string
  host_id: string
}

export interface ApiResp {
  message?: string
  detailed_error?: string
  status_code: number
}

export interface CreateMeetingResponse {
  meeting_url: string
  mode: 'new_tab' | 'embed_iframe'
}

const CALLS_ROUTE = '/plugins/com.mattermost.calls'

interface CallsConfigWire {
  ICEServersConfigs?: RTCIceServer[]
  ice_servers?: Array<{
    urls?: string[]
    username?: string
    credential?: string
  }>
  NeedsTURNCredentials?: boolean
}

interface CallStateWire {
  id?: string
  id_raw?: string
  channel_id?: string
  channel_id_raw?: string
  start_at?: number
  owner_id?: string
  owner_id_raw?: string
  host_id?: string
  host_id_raw?: string
  participants?: string[]
  participants_raw?: string[]
  sessions?: Record<
    string,
    {
      session_id?: string
      session_id_raw?: string
      user_id?: string
      user_id_raw?: string
      username?: string
      display_name?: string
      unmuted?: boolean
      raised_hand?: number
    }
  >
  thread_id?: string
  screen_sharing_id?: string
  screen_sharing_id_raw?: string
  screen_sharing_session_id?: string
  screen_sharing_session_id_raw?: string
}

interface ChannelStateWire {
  channel_id?: string
  channel_id_raw?: string
  enabled?: boolean
  call?: CallStateWire
  call_id?: string
  call_id_raw?: string
  has_call?: boolean
}

function normalizeIceServers(raw: CallsConfigWire): RTCIceServer[] {
  if (Array.isArray(raw.ICEServersConfigs) && raw.ICEServersConfigs.length > 0) {
    return raw.ICEServersConfigs
  }

  if (!Array.isArray(raw.ice_servers)) {
    return []
  }

  return raw.ice_servers.map(entry => ({
    urls: entry.urls || [],
    username: entry.username,
    credential: entry.credential,
  }))
}

function normalizeConfig(raw: CallsConfigWire): CallsConfig {
  return {
    ICEServersConfigs: normalizeIceServers(raw),
    AllowEnableCalls: true,
    DefaultEnabled: true,
    NeedsTURNCredentials: raw.NeedsTURNCredentials || false,
    MaxCallParticipants: 0,
    AllowScreenSharing: true,
    EnableSimulcast: false,
    EnableRinging: true,
    EnableLiveCaptions: false,
    HostControlsAllowed: true,
    EnableRecordings: false,
    MaxRecordingDuration: 0,
    GroupCallsAllowed: true,
  }
}

function normalizeCallState(channelId: string, raw: CallStateWire): CallState {
  if (raw.sessions && typeof raw.sessions === 'object') {
    const sessions: Record<string, CallSession> = {}
    for (const [key, value] of Object.entries(raw.sessions)) {
      const sessionId = value.session_id || key
      sessions[sessionId] = {
        session_id: sessionId,
        session_id_raw: value.session_id_raw,
        user_id: value.user_id || '',
        user_id_raw: value.user_id_raw,
        username: value.username,
        display_name: value.display_name,
        unmuted: value.unmuted ?? false,
        raised_hand: value.raised_hand ?? 0,
      }
    }

    return {
      id: raw.id || '',
      id_raw: raw.id_raw || raw.id || '',
      channel_id: channelId,
      channel_id_raw: raw.channel_id_raw || channelId,
      start_at: raw.start_at || Date.now(),
      owner_id: raw.owner_id_raw || raw.owner_id || '',
      owner_id_raw: raw.owner_id_raw || raw.owner_id || '',
      host_id: raw.host_id_raw || raw.host_id || '',
      host_id_raw: raw.host_id_raw || raw.host_id || '',
      thread_id: raw.thread_id,
      screen_sharing_id: raw.screen_sharing_id,
      screen_sharing_id_raw: raw.screen_sharing_id_raw,
      screen_sharing_session_id: raw.screen_sharing_session_id || raw.screen_sharing_session_id_raw,
      sessions,
    }
  }

  const participants = raw.participants_raw || raw.participants || []
  const sessions: Record<string, CallSession> = {}
  for (const participantId of participants) {
    sessions[participantId] = {
      session_id: participantId,
      user_id: participantId,
      unmuted: false,
      raised_hand: 0,
    }
  }

  return {
    id: raw.id || '',
    id_raw: raw.id_raw || raw.id || '',
    channel_id: channelId,
    channel_id_raw: raw.channel_id_raw || channelId,
    start_at: raw.start_at || Date.now(),
    owner_id: raw.owner_id_raw || raw.owner_id || '',
    owner_id_raw: raw.owner_id_raw || raw.owner_id || '',
    host_id: raw.host_id_raw || raw.host_id || '',
    host_id_raw: raw.host_id_raw || raw.host_id || '',
    thread_id: raw.thread_id,
    screen_sharing_id: raw.screen_sharing_id,
    screen_sharing_id_raw: raw.screen_sharing_id_raw,
    screen_sharing_session_id: raw.screen_sharing_session_id || raw.screen_sharing_session_id_raw,
    sessions,
  }
}

async function fetchCallForChannel(channelId: string): Promise<CallChannelState> {
  const response = await v4Api.get<CallStateWire>(`${CALLS_ROUTE}/calls/${channelId}?mobilev2=true`)
  if (!response.data) {
    return {
      channel_id: channelId,
      enabled: true,
    }
  }
  return {
    channel_id: channelId,
    enabled: true,
    call: normalizeCallState(channelId, response.data),
  }
}

export default {
  // Check if calls plugin is enabled
  async getEnabled(): Promise<boolean> {
    try {
      await v4Api.get(`${CALLS_ROUTE}/version`)
      return true
    } catch (e) {
      return false
    }
  },

  // Get calls plugin version
  getVersion() {
    return v4Api.get<CallsVersionInfo>(`${CALLS_ROUTE}/version`)
  },

  // Get calls config (ICE servers, etc)
  async getConfig() {
    const response = await v4Api.get<CallsConfigWire>(`${CALLS_ROUTE}/config`)
    return {
      ...response,
      data: normalizeConfig(response.data),
    }
  },

  // Get ephemeral TURN credentials
  async getTurnCredentials() {
    return v4Api.get<RTCIceServer[]>(`${CALLS_ROUTE}/turn-credentials`)
  },

  // Get all active calls
  async getCalls() {
    const response = await v4Api.get<ChannelStateWire[]>(`${CALLS_ROUTE}/channels?mobilev2=true`)
    const channels: CallChannelState[] = []

    for (const channel of response.data || []) {
      const channelId = channel.channel_id_raw || channel.channel_id
      if (!channelId) {
        continue
      }

      if (channel.call) {
        channels.push({
          channel_id: channelId,
          enabled: channel.enabled !== false,
          call: normalizeCallState(channelId, channel.call),
        })
        continue
      }

      if (channel.has_call || channel.call_id || channel.call_id_raw) {
        try {
          channels.push(await fetchCallForChannel(channelId))
          continue
        } catch (error) {
          // Fall back to channel-only state when call details are unavailable
        }
      }

      channels.push({
        channel_id: channelId,
        enabled: channel.enabled !== false,
      })
    }

    return {
      ...response,
      data: channels,
    }
  },

  // Get call for specific channel
  async getCallForChannel(channelId: string) {
    const response = await fetchCallForChannel(channelId)
    return { data: response }
  },

  // Start a new call in a channel
  startCall(channelId: string) {
    return v4Api.post<StartCallResponse>(`${CALLS_ROUTE}/calls/${channelId}/start`)
  },

  // Join an existing call
  joinCall(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/join`)
  },

  // Leave a call
  leaveCall(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/leave`)
  },

  // End a call (host only)
  endCall(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/end`)
  },

  // Mute self
  mute(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/mute`)
  },

  // Unmute self
  unmute(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/unmute`)
  },

  // Raise hand
  raiseHand(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/raise-hand`)
  },

  // Lower hand
  lowerHand(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/lower-hand`)
  },

  // Send reaction
  sendReaction(channelId: string, emoji: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/react`, { emoji })
  },

  // Toggle screen share
  toggleScreenShare(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/screen-share`)
  },

  // WebRTC Signaling
  sendOffer(channelId: string, sdp: string) {
    return v4Api.post<{ sdp: string; type_: string }>(`${CALLS_ROUTE}/calls/${channelId}/offer`, {
      sdp,
    })
  },

  sendIceCandidate(channelId: string, candidate: string, sdpMid?: string, sdpMLineIndex?: number) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/ice`, {
      candidate,
      sdp_mid: sdpMid,
      sdp_mline_index: sdpMLineIndex,
    })
  },

  // Host controls
  hostMakeHost(channelId: string, newHostId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/make`, {
      new_host_id: newHostId,
    })
  },

  hostMute(channelId: string, sessionId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/mute`, {
      session_id: sessionId,
    })
  },

  hostMuteOthers(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/mute-others`)
  },

  hostScreenOff(channelId: string, sessionId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/screen-off`, {
      session_id: sessionId,
    })
  },

  hostLowerHand(channelId: string, sessionId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/lower-hand`, {
      session_id: sessionId,
    })
  },

  hostRemove(channelId: string, sessionId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/remove`, {
      session_id: sessionId,
    })
  },

  // Ringing
  ringUsers(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/ring`)
  },

  dismissNotification(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/dismiss-notification`)
  },

  // Recording
  startRecording(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/recording/start`)
  },

  stopRecording(channelId: string) {
    return v4Api.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/recording/stop`)
  },

  // Enable/disable calls in channel (admin)
  enableChannelCalls(channelId: string, enable: boolean) {
    return v4Api.post<CallChannelState>(`${CALLS_ROUTE}/${channelId}`, { enabled: enable })
  },
}
