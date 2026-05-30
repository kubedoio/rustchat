import type { UserId } from './User'
import type { ChannelId } from './Channel'

export type CallId = string & { __brand: 'CallId' }
export type SessionId = string & { __brand: 'SessionId' }

export function createCallId(id: string): CallId {
  return id as CallId
}

export function createSessionId(id: string): SessionId {
  return id as SessionId
}

export interface CallParticipant {
  sessionId: SessionId
  userId: UserId
  username: string
  displayName?: string
  isMuted: boolean
  isSpeaking: boolean
  isScreenSharing: boolean
  raisedHandAt: number // timestamp, 0 if not raised
  joinedAt: Date
}

// Export Call as alias for CallState for compatibility
export type Call = CallState

export interface CallState {
  id: CallId
  channelId: ChannelId
  startedAt: Date
  startedBy: UserId
  hostId: UserId
  participants: Map<SessionId, CallParticipant>
  screenSharingSessionId?: SessionId
  threadId?: string
  recording?: {
    isRecording: boolean
    startedBy?: UserId
    startedAt?: Date
  }
}

export interface CallConfig {
  iceServers: RTCIceServer[]
  allowEnableCalls: boolean
  defaultEnabled: boolean
  needsTURNCredentials: boolean
  maxParticipants: number
  allowScreenSharing: boolean
  enableSimulcast: boolean
  enableRinging: boolean
  enableLiveCaptions: boolean
  hostControlsAllowed: boolean
  enableRecordings: boolean
  maxRecordingDuration: number
  groupCallsAllowed: boolean
}

export interface CurrentCallSession {
  callId: CallId
  channelId: ChannelId
  mySessionId: SessionId
  call?: CallState // The actual call state
  peerConnection?: RTCPeerConnection
  screenSender?: RTCRtpSender | null
  localStream?: MediaStream
  screenStream?: MediaStream
  remoteStreams: Map<string, MediaStream>
}

export interface IncomingCall {
  channelId: ChannelId
  callerId: UserId
}

// Helper functions
export function isHost(call: CallState, userId: UserId): boolean {
  return call.hostId === userId
}

export function isParticipant(call: CallState, userId: UserId): boolean {
  for (const participant of call.participants.values()) {
    if (participant.userId === userId) return true
  }
  return false
}

export function getParticipantCount(call: CallState): number {
  return call.participants.size
}
