import { get } from 'svelte/store'
import { svelteApi, SvelteHttpError } from './http'
import { authStore } from './auth'
import { onWebSocketEvent } from './websocket'
import type { CallState, CallSession, CallsConfig, CallChannelState, StartCallResponse, ApiResp } from '../../api/calls'

const CALLS_ROUTE = '/plugins/com.mattermost.calls'
const API_BASE = '/api/v4'

export interface CurrentCall {
  channelId: string
  call: CallState
  mySessionId: string
  peerConnection: RTCPeerConnection | null
  screenSender: RTCRtpSender | null
  localStream: MediaStream | null
  screenStream: MediaStream | null
  remoteStreams: Map<string, MediaStream>
}

// ─── Internal reactive state ───
let callsConfig = $state<CallsConfig | null>(null)
let activeCalls = $state<Map<string, CallState>>(new Map())
let currentCall = $state<CurrentCall | null>(null)
let isExpanded = $state(false)
let incomingCall = $state<{ channelId: string; callerId: string } | null>(null)
let isMuted = $state(true)
let isHandRaised = $state(false)
let isScreenSharing = $state(false)
let speakingParticipants = $state<Set<string>>(new Set())

// Device preferences
let preferredAudioInput = $state<string>(localStorage.getItem('calls_preferred_audio_input') || '')
let preferredAudioOutput = $state<string>(localStorage.getItem('calls_preferred_audio_output') || '')
let preferredVideoDevice = $state<string>(localStorage.getItem('calls_preferred_video_device') || '')

// ─── Derived state ───
let isInCall = $derived(!!currentCall)
let currentCallParticipants = $derived(currentCall ? Object.values(currentCall.call.sessions || {}) : [])

function currentChannelCall(channelId: string): CallState | undefined {
  return activeCalls.get(channelId)
}

// ─── Event field helpers ───
function readEventChannelId(data: any): string | undefined {
  return data?.channel_id_raw || data?.channel_id
}

function readEventUserId(data: any): string | undefined {
  return data?.user_id_raw || data?.user_id
}

function readEventSessionId(data: any): string | undefined {
  return data?.session_id_raw || data?.session_id
}

function getMyUserId(): string | undefined {
  return get(authStore).user?.id
}

function findMySessionId(call: CallState): string {
  const myUserId = getMyUserId()
  if (!myUserId) return ''
  const selfSession = Object.values(call.sessions || {}).find(
    (session) => (session.user_id_raw || session.user_id) === myUserId
  )
  return selfSession?.session_id || ''
}

function syncSelfCallFlags(call: CallState) {
  const mySessionId = findMySessionId(call)
  const mySession = mySessionId ? call.sessions?.[mySessionId] : undefined
  isMuted = mySession ? !mySession.unmuted : true
  isHandRaised = !!mySession && mySession.raised_hand > 0
  isScreenSharing = (call.screen_sharing_id_raw || call.screen_sharing_id) === getMyUserId()
}

// ─── Inline API layer using svelteApi ───

interface CallsConfigWire {
  ICEServersConfigs?: RTCIceServer[]
  ice_servers?: Array<{ urls?: string[]; username?: string; credential?: string }>
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
  sessions?: Record<string, {
    session_id?: string
    session_id_raw?: string
    user_id?: string
    user_id_raw?: string
    username?: string
    display_name?: string
    unmuted?: boolean
    raised_hand?: number
  }>
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
  if (!Array.isArray(raw.ice_servers)) return []
  return raw.ice_servers.map((entry) => ({
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
  const response = await svelteApi.get<CallStateWire>(`${CALLS_ROUTE}/calls/${channelId}?mobilev2=true`, { baseURL: API_BASE })
  if (!response.data) {
    return { channel_id: channelId, enabled: true }
  }
  return {
    channel_id: channelId,
    enabled: true,
    call: normalizeCallState(channelId, response.data),
  }
}

const callsApi = {
  async getConfig() {
    const response = await svelteApi.get<CallsConfigWire>(`${CALLS_ROUTE}/config`, { baseURL: API_BASE })
    return { ...response, data: normalizeConfig(response.data) }
  },
  getTurnCredentials() {
    return svelteApi.get<RTCIceServer[]>(`${CALLS_ROUTE}/turn-credentials`, { baseURL: API_BASE })
  },
  async getCalls() {
    const response = await svelteApi.get<ChannelStateWire[]>(`${CALLS_ROUTE}/channels?mobilev2=true`, { baseURL: API_BASE })
    const channels: CallChannelState[] = []
    for (const channel of response.data || []) {
      const channelId = channel.channel_id_raw || channel.channel_id
      if (!channelId) continue
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
        } catch {
          // Fall back to channel-only state
        }
      }
      channels.push({ channel_id: channelId, enabled: channel.enabled !== false })
    }
    return { ...response, data: channels }
  },
  async getCallForChannel(channelId: string) {
    const response = await fetchCallForChannel(channelId)
    return { data: response }
  },
  startCall(channelId: string) {
    return svelteApi.post<StartCallResponse>(`${CALLS_ROUTE}/calls/${channelId}/start`, undefined, { baseURL: API_BASE })
  },
  joinCall(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/join`, undefined, { baseURL: API_BASE })
  },
  leaveCall(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/leave`, undefined, { baseURL: API_BASE })
  },
  endCall(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/end`, undefined, { baseURL: API_BASE })
  },
  mute(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/mute`, undefined, { baseURL: API_BASE })
  },
  unmute(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/unmute`, undefined, { baseURL: API_BASE })
  },
  raiseHand(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/raise-hand`, undefined, { baseURL: API_BASE })
  },
  lowerHand(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/lower-hand`, undefined, { baseURL: API_BASE })
  },
  sendReaction(channelId: string, emoji: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/react`, { emoji }, { baseURL: API_BASE })
  },
  toggleScreenShare(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/screen-share`, undefined, { baseURL: API_BASE })
  },
  sendOffer(channelId: string, sdp: string) {
    return svelteApi.post<{ sdp: string; type_: string }>(`${CALLS_ROUTE}/calls/${channelId}/offer`, { sdp }, { baseURL: API_BASE })
  },
  sendIceCandidate(channelId: string, candidate: string, sdpMid?: string, sdpMLineIndex?: number) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/ice`, { candidate, sdp_mid: sdpMid, sdp_mline_index: sdpMLineIndex }, { baseURL: API_BASE })
  },
  hostMute(channelId: string, sessionId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/mute`, { session_id: sessionId }, { baseURL: API_BASE })
  },
  hostMuteOthers(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/mute-others`, undefined, { baseURL: API_BASE })
  },
  hostRemove(channelId: string, sessionId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/host/remove`, { session_id: sessionId }, { baseURL: API_BASE })
  },
  ringUsers(channelId: string) {
    return svelteApi.post<ApiResp>(`${CALLS_ROUTE}/calls/${channelId}/ring`, undefined, { baseURL: API_BASE })
  },
}

// ─── Core actions ───

async function loadConfig(): Promise<CallsConfig | null> {
  try {
    const { data } = await callsApi.getConfig()
    if (data.NeedsTURNCredentials) {
      try {
        const { data: turnServers } = await callsApi.getTurnCredentials()
        data.ICEServersConfigs = [
          ...data.ICEServersConfigs.filter((s) => {
            const urls = Array.isArray(s.urls) ? s.urls : [s.urls]
            return !urls.some((url) => url.toString().startsWith('turn:'))
          }),
          ...turnServers,
        ]
      } catch (error) {
        console.error('Failed to fetch TURN credentials', error)
      }
    }
    callsConfig = data
    return data
  } catch (error) {
    console.error('Failed to load calls config', error)
    return null
  }
}

async function loadCalls() {
  try {
    const { data } = await callsApi.getCalls()
    activeCalls.clear()
    for (const channel of data) {
      if (channel.call) {
        activeCalls.set(channel.channel_id, channel.call)
      }
    }
    return data
  } catch (error) {
    console.error('Failed to load calls', error)
    return []
  }
}

async function loadCallForChannel(channelId: string) {
  try {
    const { data } = await callsApi.getCallForChannel(channelId)
    if (!data) return null
    if (data.call) {
      activeCalls.set(channelId, data.call)
      if (currentCall?.channelId === channelId) {
        currentCall.call = data.call
        const mySessionId = findMySessionId(data.call)
        if (mySessionId) {
          currentCall.mySessionId = mySessionId
        }
        syncSelfCallFlags(data.call)
      }
    } else {
      activeCalls.delete(channelId)
    }
    return data
  } catch (error) {
    if (error instanceof SvelteHttpError && error.status !== 404) {
      console.error('Failed to load call for channel', error)
    }
    return null
  }
}

async function startCall(channelId: string) {
  try {
    const config = await loadConfig()
    if (!config) throw new Error('Calls plugin not available')

    const { data: callData } = await callsApi.startCall(channelId)
    const channelState = await loadCallForChannel(channelId)
    if (!channelState?.call) throw new Error('Call started but state could not be loaded')

    const mySessionId = findMySessionId(channelState.call)
    if (!mySessionId) throw new Error('Could not resolve your call session')

    currentCall = {
      channelId,
      call: channelState.call,
      mySessionId,
      peerConnection: null,
      screenSender: null,
      localStream: null,
      screenStream: null,
      remoteStreams: new Map(),
    }
    syncSelfCallFlags(channelState.call)

    const rtc = await initializeWebRTC(channelId, config.ICEServersConfigs)
    if (currentCall) {
      currentCall.peerConnection = rtc.pc
      currentCall.localStream = rtc.stream
    }

    isExpanded = true
    console.warn('[toast] Call started: You are now in a call')
    return callData
  } catch (error: any) {
    cleanupWebRTC()
    currentCall = null
    console.error('Failed to start call', error)
    console.warn('[toast] Failed to start call:', error.message || 'Unknown error')
    throw error
  }
}

async function joinCall(channelId: string) {
  try {
    const channelState = await loadCallForChannel(channelId)
    if (!channelState?.call) throw new Error('No active call in this channel')

    const config = await loadConfig()
    if (!config) throw new Error('Calls plugin not available')

    await callsApi.joinCall(channelId)

    const refreshedState = await loadCallForChannel(channelId)
    const callState = refreshedState?.call || channelState.call
    const mySessionId = findMySessionId(callState)
    if (!mySessionId) throw new Error('Could not resolve your call session')

    currentCall = {
      channelId,
      call: callState,
      mySessionId,
      peerConnection: null,
      screenSender: null,
      localStream: null,
      screenStream: null,
      remoteStreams: new Map(),
    }
    syncSelfCallFlags(callState)

    const rtc = await initializeWebRTC(channelId, config.ICEServersConfigs)
    if (currentCall) {
      currentCall.peerConnection = rtc.pc
      currentCall.localStream = rtc.stream
    }

    isExpanded = true
    console.warn('[toast] Joined call: You are now in the call')
  } catch (error: any) {
    cleanupWebRTC()
    currentCall = null
    console.error('Failed to join call', error)
    console.warn('[toast] Failed to join call:', error.message || 'Unknown error')
    throw error
  }
}

async function leaveCall() {
  if (!currentCall) return
  const channelId = currentCall.channelId
  try {
    cleanupWebRTC()
    await callsApi.leaveCall(channelId)
    currentCall = null
    isMuted = true
    isHandRaised = false
    isScreenSharing = false
    isExpanded = false
    speakingParticipants.clear()
  } catch (error) {
    console.error('Failed to leave call', error)
  }
}

async function endCall() {
  if (!currentCall) return
  const channelId = currentCall.channelId
  try {
    cleanupWebRTC()
    await callsApi.endCall(channelId)
    currentCall = null
    isMuted = true
    isHandRaised = false
    isScreenSharing = false
    isExpanded = false
    speakingParticipants.clear()
  } catch (error) {
    console.error('Failed to end call', error)
    console.warn('[toast] Failed to end call: Only the host can end the call')
  }
}

// ─── WebRTC helpers ───

function shouldUseSimulcast(): boolean {
  return callsConfig?.EnableSimulcast === true
}

function stripSimulcastFromSdp(sdp: string): string {
  if (!sdp) return sdp
  const lines = sdp
    .split(/\r\n|\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)

  let removedAny = false
  const filtered = lines.filter((line) => {
    const lower = line.toLowerCase()
    if (lower.startsWith('a=simulcast:')) { removedAny = true; return false }
    if (lower.startsWith('a=rid:')) { removedAny = true; return false }
    if (lower.includes('urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id')) { removedAny = true; return false }
    if (lower.includes('urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id')) { removedAny = true; return false }
    return true
  })

  if (!removedAny) return sdp
  return `${filtered.join('\r\n')}\r\n`
}

function prepareOfferSdp(sdp: string): string {
  if (shouldUseSimulcast()) return sdp
  return stripSimulcastFromSdp(sdp)
}

function applyVideoCodecPreferences(pc: RTCPeerConnection) {
  if (shouldUseSimulcast()) return
  const capabilities = RTCRtpSender.getCapabilities?.('video')
  const codecs = capabilities?.codecs || []
  if (codecs.length === 0) return

  const primary = codecs.filter((codec) => {
    const mime = codec.mimeType.toLowerCase()
    return mime === 'video/vp8' || mime === 'video/h264'
  })
  if (primary.length === 0) return

  const repair = codecs.filter((codec) => {
    const mime = codec.mimeType.toLowerCase()
    return mime === 'video/rtx' || mime === 'video/red' || mime === 'video/ulpfec'
  })
  const preferred = [...primary, ...repair]

  for (const transceiver of pc.getTransceivers()) {
    const senderKind = transceiver.sender?.track?.kind
    const receiverKind = transceiver.receiver?.track?.kind
    if (senderKind !== 'video' && receiverKind !== 'video') continue
    if (typeof transceiver.setCodecPreferences !== 'function') continue
    try {
      transceiver.setCodecPreferences(preferred)
    } catch (error) {
      console.debug('Failed to set codec preferences on transceiver', error)
    }
  }
}

async function createAndSendOffer(channelId: string, pc: RTCPeerConnection) {
  applyVideoCodecPreferences(pc)
  const offer = await pc.createOffer()
  const rawSdp = offer.sdp || ''
  const preparedSdp = prepareOfferSdp(rawSdp)

  let selectedSdp = preparedSdp
  try {
    await pc.setLocalDescription({ type: 'offer', sdp: preparedSdp })
  } catch (error) {
    console.warn('Prepared SDP rejected by browser, retrying with original SDP', error)
    selectedSdp = rawSdp
    await pc.setLocalDescription({ type: 'offer', sdp: rawSdp })
  }

  return callsApi.sendOffer(channelId, selectedSdp)
}

async function initializeWebRTC(channelId: string, iceServers: RTCIceServer[]) {
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false })
    stream.getAudioTracks().forEach((track) => { track.enabled = false })

    const pc = new RTCPeerConnection({
      iceServers: (iceServers || []).length > 0 ? iceServers : [{ urls: 'stun:stun.l.google.com:19302' }],
    })

    stream.getTracks().forEach((track) => pc.addTrack(track, stream))

    pc.ontrack = (event) => {
      console.log('Received remote track:', event.track.kind, event.streams)
      const active = currentCall
      if (!active) return

      if (event.streams && event.streams[0]) {
        const remoteStream = event.streams[0]
        active.remoteStreams.set(remoteStream.id, remoteStream)
      } else {
        const syntheticStreamId = `track-${event.track.id}`
        const existing = active.remoteStreams.get(syntheticStreamId)
        const synthetic = existing || new MediaStream()
        const hasTrack = synthetic.getTracks().some((t) => t.id === event.track.id)
        if (!hasTrack) synthetic.addTrack(event.track)
        active.remoteStreams.set(syntheticStreamId, synthetic)
      }

      event.track.onended = () => {
        const call = currentCall
        if (!call) return
        for (const [key, stream] of call.remoteStreams.entries()) {
          const remainingTracks = stream.getTracks().filter((t) => t.id !== event.track.id)
          if (remainingTracks.length === stream.getTracks().length) continue
          if (remainingTracks.length === 0) {
            call.remoteStreams.delete(key)
            continue
          }
          const replacement = new MediaStream()
          remainingTracks.forEach((t) => replacement.addTrack(t))
          call.remoteStreams.set(key, replacement)
        }
      }
    }

    pc.onicecandidate = async (event) => {
      if (event.candidate) {
        await callsApi.sendIceCandidate(
          channelId,
          JSON.stringify(event.candidate),
          event.candidate.sdpMid || undefined,
          event.candidate.sdpMLineIndex || undefined
        )
      }
    }

    const { data: answer } = await createAndSendOffer(channelId, pc)
    await pc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: answer.sdp }))

    return { pc, stream }
  } catch (error) {
    console.error('WebRTC initialization failed', error)
    throw error
  }
}

function cleanupWebRTC() {
  if (currentCall?.peerConnection) {
    currentCall.peerConnection.close()
  }
  if (currentCall) {
    currentCall.screenSender = null
  }
  if (currentCall?.localStream) {
    currentCall.localStream.getTracks().forEach((track) => track.stop())
  }
  if (currentCall?.screenStream) {
    currentCall.screenStream.getTracks().forEach((track) => track.stop())
    currentCall.screenStream = null
  }
  currentCall?.remoteStreams.clear()
}

async function renegotiate(channelId: string, pc: RTCPeerConnection) {
  const { data: answer } = await createAndSendOffer(channelId, pc)
  await pc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: answer.sdp }))
}

// ─── Call controls ───

async function toggleMute() {
  if (!currentCall) return
  const channelId = currentCall.channelId
  try {
    if (isMuted) {
      await callsApi.unmute(channelId)
      currentCall.localStream?.getAudioTracks().forEach((track) => { track.enabled = true })
    } else {
      await callsApi.mute(channelId)
      currentCall.localStream?.getAudioTracks().forEach((track) => { track.enabled = false })
    }
    isMuted = !isMuted
  } catch (error) {
    console.error('Failed to toggle mute', error)
  }
}

async function toggleHand() {
  if (!currentCall) return
  const channelId = currentCall.channelId
  try {
    if (isHandRaised) {
      await callsApi.lowerHand(channelId)
    } else {
      await callsApi.raiseHand(channelId)
    }
    isHandRaised = !isHandRaised
  } catch (error) {
    console.error('Failed to toggle hand', error)
  }
}

async function stopLocalScreenShare() {
  if (!currentCall?.screenStream) return
  currentCall.screenStream.getTracks().forEach((track) => {
    track.onended = null
    track.stop()
  })
  currentCall.screenStream = null
  const sender = currentCall.screenSender
  if (sender) {
    await sender.replaceTrack(null)
  }
}

async function toggleScreenShare() {
  if (!currentCall || !currentCall.peerConnection) return
  const channelId = currentCall.channelId
  const pc = currentCall.peerConnection
  try {
    if (currentCall.screenStream) {
      await stopLocalScreenShare()
      await callsApi.toggleScreenShare(channelId)
      await renegotiate(channelId, pc)
      isScreenSharing = false
    } else {
      const stream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false })
      currentCall.screenStream = stream
      const [videoTrack] = stream.getVideoTracks()
      if (videoTrack) {
        videoTrack.contentHint = 'detail'
        videoTrack.onended = () => {
          if (currentCall?.screenStream) {
            void toggleScreenShare()
          }
        }
        if (currentCall.screenSender) {
          await currentCall.screenSender.replaceTrack(videoTrack)
        } else {
          currentCall.screenSender = pc.addTrack(videoTrack, stream)
        }
      }
      await callsApi.toggleScreenShare(channelId)
      await renegotiate(channelId, pc)
      isScreenSharing = true
    }
  } catch (error) {
    console.error('Failed to toggle screen share', error)
    await stopLocalScreenShare()
    isScreenSharing = false
  }
}

async function sendReaction(emoji: string) {
  if (!currentCall) return
  try {
    await callsApi.sendReaction(currentCall.channelId, emoji)
  } catch (error) {
    console.error('Failed to send reaction', error)
  }
}

async function ring(channelId: string) {
  try {
    await callsApi.ringUsers(channelId)
    console.warn('[toast] Ringing participants: Other channel members have been notified')
  } catch (error) {
    console.error('Failed to ring users', error)
  }
}

async function hostMute(sessionId: string) {
  if (!currentCall) return
  try {
    await callsApi.hostMute(currentCall.channelId, sessionId)
  } catch (error) {
    console.error('Failed to host mute', error)
  }
}

async function hostMuteOthers() {
  if (!currentCall) return
  try {
    await callsApi.hostMuteOthers(currentCall.channelId)
    console.warn('[toast] Muted all: All other participants have been muted')
  } catch (error) {
    console.error('Failed to host mute others', error)
  }
}

async function hostRemove(sessionId: string) {
  if (!currentCall) return
  try {
    await callsApi.hostRemove(currentCall.channelId, sessionId)
  } catch (error) {
    console.error('Failed to host remove', error)
  }
}

// ─── State setters ───

function setIncomingCall(call: { channelId: string; callerId: string } | null) {
  incomingCall = call
}

function toggleExpanded() {
  isExpanded = !isExpanded
}

// ─── Device preferences ───

function setPreferredAudioInput(deviceId: string) {
  preferredAudioInput = deviceId
  localStorage.setItem('calls_preferred_audio_input', deviceId)
}

function setPreferredAudioOutput(deviceId: string) {
  preferredAudioOutput = deviceId
  localStorage.setItem('calls_preferred_audio_output', deviceId)
}

function setPreferredVideoDevice(deviceId: string) {
  preferredVideoDevice = deviceId
  localStorage.setItem('calls_preferred_video_device', deviceId)
}

function resetSessionState() {
  cleanupWebRTC()
  activeCalls.clear()
  currentCall = null
  incomingCall = null
  isExpanded = false
  isMuted = true
  isHandRaised = false
  isScreenSharing = false
  speakingParticipants.clear()
}

// ─── WebSocket handler registration ───

export function registerCallWebSocketHandlers() {
  onWebSocketEvent('custom_com.mattermost.calls_call_start', (data) => {
    console.log('Call started:', data)
    void loadCalls()
  })

  onWebSocketEvent('custom_com.mattermost.calls_call_end', (data) => {
    console.log('Call ended:', data)
    const eventChannelId = readEventChannelId(data)
    if (eventChannelId && currentCall?.channelId === eventChannelId) {
      void leaveCall()
    }
    if (eventChannelId) {
      activeCalls.delete(eventChannelId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_user_joined', (data) => {
    console.log('User joined call:', data)
    const eventChannelId = readEventChannelId(data)
    if (eventChannelId) {
      void loadCallForChannel(eventChannelId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_user_left', (data) => {
    console.log('User left call:', data)
    const eventChannelId = readEventChannelId(data)
    if (eventChannelId) {
      void loadCallForChannel(eventChannelId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_user_muted', (data) => {
    console.log('User muted:', data)
    const d = data as Record<string, unknown>
    const eventChannelId = readEventChannelId(d)
    if (eventChannelId && currentCall?.channelId === eventChannelId) {
      const userId = readEventUserId(d)
      if (userId === getMyUserId()) {
        const muted = d.muted as boolean
        isMuted = muted
        const active = currentCall
        if (active) {
          active.localStream?.getAudioTracks().forEach((track) => { track.enabled = !muted })
        }
      }
      void loadCallForChannel(eventChannelId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_user_unmuted', (data) => {
    console.log('User unmuted:', data)
    const eventChannelId = readEventChannelId(data)
    if (!eventChannelId || currentCall?.channelId !== eventChannelId) return
    const userId = readEventUserId(data)
    if (userId === getMyUserId()) {
      isMuted = false
      currentCall?.localStream?.getAudioTracks().forEach((track) => { track.enabled = true })
    }
    void loadCallForChannel(eventChannelId)
  })

  onWebSocketEvent('custom_com.mattermost.calls_raise_hand', (data) => {
    console.log('Hand raised:', data)
    const eventChannelId = readEventChannelId(data)
    if (!eventChannelId || currentCall?.channelId !== eventChannelId) return
    const userId = readEventUserId(data)
    if (userId === getMyUserId()) {
      isHandRaised = true
    }
    void loadCallForChannel(eventChannelId)
  })

  onWebSocketEvent('custom_com.mattermost.calls_lower_hand', (data) => {
    console.log('Hand lowered:', data)
    const eventChannelId = readEventChannelId(data)
    if (eventChannelId && currentCall?.channelId === eventChannelId) {
      const userId = readEventUserId(data)
      if (userId === getMyUserId()) {
        isHandRaised = false
      }
      void loadCallForChannel(eventChannelId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_user_voice_on', (data) => {
    const sessionId = readEventSessionId(data)
    if (sessionId) {
      speakingParticipants.add(sessionId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_user_voice_off', (data) => {
    const sessionId = readEventSessionId(data)
    if (sessionId) {
      speakingParticipants.delete(sessionId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_host_mute', (data) => {
    if (!currentCall) return
    const sessionId = readEventSessionId(data)
    if (sessionId === currentCall?.mySessionId) {
      isMuted = true
      const active = currentCall
      if (active) {
        active.localStream?.getAudioTracks().forEach((track) => { track.enabled = false })
      }
      console.warn('[toast] Host muted you: Your microphone has been disabled by the host')
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_host_removed', (data) => {
    if (!currentCall) return
    const sessionId = readEventSessionId(data)
    if (sessionId === currentCall?.mySessionId) {
      void leaveCall()
      console.warn('[toast] Removed from call: You have been removed from the call by the host')
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_host_changed', (data) => {
    const d = data as Record<string, unknown>
    const eventChannelId = readEventChannelId(d)
    if (currentCall && currentCall.channelId === eventChannelId) {
      currentCall.call.host_id = (d.host_id || d.host_id_raw) as string
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_ringing', (data) => {
    if (isInCall) return
    const d = data as Record<string, unknown>
    const eventChannelId = readEventChannelId(d)
    const callerId = (d.sender_id || d.sender_id_raw) as string | undefined
    if (eventChannelId && callerId) {
      setIncomingCall({ channelId: eventChannelId, callerId })
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_screen_on', (data) => {
    console.log('Screen share on:', data)
    const eventChannelId = readEventChannelId(data)
    if (currentCall && currentCall.channelId === eventChannelId) {
      const userId = readEventUserId(data)
      if (userId === getMyUserId()) {
        isScreenSharing = true
      }
      void loadCallForChannel(eventChannelId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_screen_off', (data) => {
    console.log('Screen share off:', data)
    const eventChannelId = readEventChannelId(data)
    if (currentCall && currentCall.channelId === eventChannelId) {
      const userId = readEventUserId(data)
      if (userId === getMyUserId()) {
        isScreenSharing = false
      }
      void loadCallForChannel(eventChannelId)
    }
  })

  onWebSocketEvent('custom_com.mattermost.calls_signal', async (data) => {
    const active = currentCall
    if (!active?.peerConnection) return
    const d = data as Record<string, unknown>
    const eventChannelId = readEventChannelId(d)
    if (eventChannelId !== active.channelId) return
    const signal = d.signal as Record<string, unknown>
    if (!signal?.type) return
    try {
      if (signal.type === 'ice-candidate' && signal.candidate) {
        await active.peerConnection.addIceCandidate({
          candidate: signal.candidate as string,
          sdpMid: (signal.sdp_mid ?? null) as string | null,
          sdpMLineIndex: (signal.sdp_mline_index ?? null) as number | null,
        })
      }
    } catch (error) {
      console.error('Failed to handle signaling event', error)
    }
  })
}

// ─── Exported singleton ───

export const callsStore = {
  // State getters
  get callsConfig() { return callsConfig },
  get activeCalls() { return activeCalls },
  get currentCall() { return currentCall },
  get isExpanded() { return isExpanded },
  get incomingCall() { return incomingCall },
  get isMuted() { return isMuted },
  get isHandRaised() { return isHandRaised },
  get isScreenSharing() { return isScreenSharing },
  get speakingParticipants() { return speakingParticipants },
  get preferredAudioInput() { return preferredAudioInput },
  get preferredAudioOutput() { return preferredAudioOutput },
  get preferredVideoDevice() { return preferredVideoDevice },

  // Derived getters
  get isInCall() { return isInCall },
  get currentCallParticipants() { return currentCallParticipants },
  currentChannelCall,

  // Actions
  loadConfig,
  loadCalls,
  loadCallForChannel,
  startCall,
  joinCall,
  leaveCall,
  endCall,
  toggleMute,
  toggleHand,
  toggleScreenShare,
  sendReaction,
  ring,
  hostMute,
  hostMuteOthers,
  hostRemove,
  setIncomingCall,
  toggleExpanded,
  initializeWebRTC,
  cleanupWebRTC,
  resetSessionState,
  setPreferredAudioInput,
  setPreferredAudioOutput,
  setPreferredVideoDevice,
}
