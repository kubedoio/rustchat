// Call Store - Pure state management for calls
// No business logic - just state and simple mutations

import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import type {
  CallState,
  CallConfig,
  CurrentCallSession,
  IncomingCall,
  SessionId,
  CallParticipant,
} from '../../../core/entities/Call'
import type { ChannelId } from '../../../core/entities/Channel'
import type { UserId } from '../../../core/entities/User'
import { useAuthStore } from '../../auth/stores/authStore'

export const useCallStore = defineStore('callStore', () => {
  // State
  const config = ref<CallConfig | null>(null)
  const activeCalls = ref<Map<ChannelId, CallState>>(new Map())
  const currentCall = ref<CurrentCallSession | null>(null)
  const incomingCall = ref<IncomingCall | null>(null)
  const isExpanded = ref(false)
  const isMuted = ref(true)
  const isHandRaised = ref(false)
  const isScreenSharing = ref(false)
  const preferredAudioInput = ref('')
  const preferredAudioOutput = ref('')
  const preferredVideoDevice = ref('')
  const speakingParticipants = ref<Set<SessionId>>(new Set())

  // Getters
  const isInCall = computed(() => !!currentCall.value)

  const currentUserId = computed((): UserId | null => {
    return useAuthStore().user?.id ?? null
  })

  const currentCallParticipants = computed((): CallParticipant[] => {
    if (!currentCall.value?.call) return []
    return Array.from(currentCall.value.call.participants.values())
  })

  const remoteStreams = computed(() => {
    return currentCall.value?.remoteStreams ?? new Map()
  })

  const currentChannelCall = computed(() => currentCall.value?.call ?? null)

  function getActiveCall(channelId: ChannelId): CallState | undefined {
    return activeCalls.value.get(channelId)
  }

  function getRemoteStream(id: string): MediaStream | undefined {
    return currentCall.value?.remoteStreams.get(id)
  }

  // Actions - Simple state mutations only
  function setConfig(value: CallConfig) {
    config.value = value
  }

  function setActiveCalls(calls: { channelId: ChannelId; enabled: boolean; call?: CallState }[]) {
    activeCalls.value.clear()
    for (const { channelId, call } of calls) {
      if (call) {
        activeCalls.value.set(channelId, call)
      }
    }
  }

  function updateActiveCall(channelId: ChannelId, call: CallState) {
    activeCalls.value.set(channelId, call)
  }

  function removeActiveCall(channelId: ChannelId) {
    activeCalls.value.delete(channelId)
  }

  function setCurrentCall(channelId: ChannelId, call: CallState) {
    const myUserId = currentUserId.value
    let mySessionId: SessionId = '' as SessionId

    // Find my session ID
    if (myUserId) {
      for (const [sessionId, participant] of call.participants) {
        if (participant.userId === myUserId) {
          mySessionId = sessionId
          break
        }
      }
    }

    currentCall.value = {
      callId: call.id,
      channelId,
      mySessionId,
      remoteStreams: new Map(),
    }
  }

  function updateCurrentCall(call: CallState) {
    if (!currentCall.value) return
    currentCall.value.call = call
  }

  function updateCurrentCallHost(hostId: UserId) {
    if (!currentCall.value?.call) return
    currentCall.value.call.hostId = hostId
  }

  function setMySessionId(sessionId: SessionId) {
    if (!currentCall.value) return
    currentCall.value.mySessionId = sessionId
  }

  function clearCurrentCall() {
    currentCall.value = null
    isMuted.value = true
    isHandRaised.value = false
    isScreenSharing.value = false
    isExpanded.value = false
    speakingParticipants.value.clear()
  }

  function setIncomingCall(call: IncomingCall | null) {
    incomingCall.value = call
  }

  function setIsExpanded(value: boolean) {
    isExpanded.value = value
  }

  function setIsMuted(value: boolean) {
    isMuted.value = value
  }

  function setIsHandRaised(value: boolean) {
    isHandRaised.value = value
  }

  function setIsScreenSharing(value: boolean) {
    isScreenSharing.value = value
  }

  function setPreferredAudioInput(value: string) {
    preferredAudioInput.value = value
  }

  function setPreferredAudioOutput(value: string) {
    preferredAudioOutput.value = value
  }

  function setPreferredVideoDevice(value: string) {
    preferredVideoDevice.value = value
  }

  function resetSessionState() {
    clearCurrentCall()
    activeCalls.value.clear()
    incomingCall.value = null
  }

  // WebRTC state mutations
  function setPeerConnection(pc: RTCPeerConnection) {
    if (!currentCall.value) return
    currentCall.value.peerConnection = pc
  }

  function setLocalStream(stream: MediaStream) {
    if (!currentCall.value) return
    currentCall.value.localStream = stream
  }

  function setScreenStream(stream: MediaStream | null) {
    if (!currentCall.value) return
    currentCall.value.screenStream = stream ?? undefined
  }

  function setScreenSender(sender: RTCRtpSender | null) {
    if (!currentCall.value) return
    currentCall.value.screenSender = sender ?? undefined
  }

  function addRemoteStream(id: string, stream: MediaStream) {
    if (!currentCall.value) return
    currentCall.value.remoteStreams.set(id, stream)
  }

  function removeRemoteStream(id: string) {
    if (!currentCall.value) return
    currentCall.value.remoteStreams.delete(id)
  }

  function clearRemoteStreams() {
    if (!currentCall.value) return
    currentCall.value.remoteStreams.clear()
  }

  // Speaking participants
  function addSpeakingParticipant(sessionId: SessionId) {
    speakingParticipants.value.add(sessionId)
  }

  function removeSpeakingParticipant(sessionId: SessionId) {
    speakingParticipants.value.delete(sessionId)
  }

  function clearSpeakingParticipants() {
    speakingParticipants.value.clear()
  }

  return {
    // State (readonly)
    config: readonly(config),
    activeCalls: readonly(activeCalls),
    currentCall: readonly(currentCall),
    incomingCall: readonly(incomingCall),
    isExpanded: readonly(isExpanded),
    isMuted: readonly(isMuted),
    isHandRaised: readonly(isHandRaised),
    isScreenSharing: readonly(isScreenSharing),
    preferredAudioInput,
    preferredAudioOutput,
    preferredVideoDevice,
    speakingParticipants: readonly(speakingParticipants),

    // Getters
    isInCall,
    currentUserId,
    currentCallParticipants,
    remoteStreams,
    currentChannelCall,

    // Actions
    getActiveCall,
    getRemoteStream,
    setConfig,
    setActiveCalls,
    updateActiveCall,
    removeActiveCall,
    setCurrentCall,
    updateCurrentCall,
    updateCurrentCallHost,
    setMySessionId,
    clearCurrentCall,
    setIncomingCall,
    setIsExpanded,
    setIsMuted,
    setIsHandRaised,
    setIsScreenSharing,
    setPreferredAudioInput,
    setPreferredAudioOutput,
    setPreferredVideoDevice,
    resetSessionState,
    setPeerConnection,
    setLocalStream,
    setScreenStream,
    setScreenSender,
    addRemoteStream,
    removeRemoteStream,
    clearRemoteStreams,
    addSpeakingParticipant,
    removeSpeakingParticipant,
    clearSpeakingParticipants,
  }
})

export const useCallsStore: any = useCallStore
