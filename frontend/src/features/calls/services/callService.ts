// Call Service - Business logic for calls
// Handles WebRTC, state management, and orchestration

import { log } from '@/utils/log'
import { callRepository } from '../repositories/callRepository'
import type { CallState, CallConfig, SessionId } from '../../../core/entities/Call'
import type { ChannelId } from '../../../core/entities/Channel'
import type { UserId } from '../../../core/entities/User'
import { useCallStore } from '../stores/callStore'
import { AppError } from '../../../core/errors/AppError'
import { useToast } from '../../../composables/useToast'

class CallService {
  private get store() {
    return useCallStore()
  }

  private get toast() {
    return useToast()
  }

  // Configuration
  async loadConfig(): Promise<CallConfig | null> {
    try {
      const config = await callRepository.getConfig()
      this.store.setConfig(config)
      return config
    } catch (error) {
      log.error('Failed to load calls config', error)
      return null
    }
  }

  // Active calls
  async loadActiveCalls(): Promise<void> {
    try {
      const calls = await callRepository.getActiveCalls()
      this.store.setActiveCalls(calls)
    } catch (error) {
      log.error('Failed to load active calls', error)
    }
  }

  async loadCallForChannel(channelId: ChannelId): Promise<CallState | null> {
    try {
      const channelState = await callRepository.getCallForChannel(channelId)
      if (channelState?.call) {
        this.store.updateActiveCall(channelId, channelState.call)

        // If we're in this call, update current call state
        if (this.store.currentCall?.channelId === channelId) {
          this.store.updateCurrentCall(channelState.call)
          this.syncSelfFlags(channelState.call)
        }

        return channelState.call
      } else {
        this.store.removeActiveCall(channelId)
        return null
      }
    } catch (error) {
      log.error('Failed to load call for channel', error)
      return null
    }
  }

  // Call lifecycle
  async startCall(channelId: ChannelId): Promise<void> {
    const config = await this.loadConfig()
    if (!config) {
      throw new AppError('Calls plugin not available')
    }

    try {
      const call = await callRepository.startCall(channelId)
      this.store.setCurrentCall(channelId, call)

      // Initialize WebRTC
      await this.initializeWebRTC(channelId, config.iceServers)

      this.store.setIsExpanded(true)
      this.toast.success('Call started', 'You are now in a call')
    } catch (error) {
      this.cleanupWebRTC()
      this.store.clearCurrentCall()
      throw error
    }
  }

  async joinCall(channelId: ChannelId): Promise<void> {
    // Check if there's an active call
    const channelState = await callRepository.getCallForChannel(channelId)
    if (!channelState?.call) {
      throw new AppError('No active call in this channel')
    }

    const config = await this.loadConfig()
    if (!config) {
      throw new AppError('Calls plugin not available')
    }

    try {
      await callRepository.joinCall(channelId)

      // Refresh call state after joining
      await this.loadCallForChannel(channelId)
      const call = this.store.getActiveCall(channelId)
      if (!call) {
        throw new AppError('Could not resolve your call session')
      }

      this.store.setCurrentCall(channelId, call)

      // Initialize WebRTC
      await this.initializeWebRTC(channelId, config.iceServers)

      this.store.setIsExpanded(true)
      this.toast.success('Joined call', 'You are now in the call')
    } catch (error) {
      this.cleanupWebRTC()
      this.store.clearCurrentCall()
      throw error
    }
  }

  async leaveCall(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    const channelId = currentCall.channelId

    try {
      this.cleanupWebRTC()
      await callRepository.leaveCall(channelId)
    } catch (error) {
      log.error('Failed to leave call', error)
    } finally {
      this.store.clearCurrentCall()
    }
  }

  async endCall(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    const channelId = currentCall.channelId

    try {
      this.cleanupWebRTC()
      await callRepository.endCall(channelId)
      this.store.clearCurrentCall()
    } catch (error) {
      this.toast.error('Failed to end call', 'Only the host can end the call')
      throw error
    }
  }

  // Media controls
  async toggleMute(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    const channelId = currentCall.channelId

    try {
      if (this.store.isMuted) {
        await callRepository.unmute(channelId)
        // Enable audio tracks
        currentCall.localStream?.getAudioTracks().forEach(track => {
          track.enabled = true
        })
      } else {
        await callRepository.mute(channelId)
        // Disable audio tracks
        currentCall.localStream?.getAudioTracks().forEach(track => {
          track.enabled = false
        })
      }
      this.store.setIsMuted(!this.store.isMuted)
    } catch (error) {
      log.error('Failed to toggle mute', error)
    }
  }

  async toggleHand(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    const channelId = currentCall.channelId

    try {
      if (this.store.isHandRaised) {
        await callRepository.lowerHand(channelId)
      } else {
        await callRepository.raiseHand(channelId)
      }
      this.store.setIsHandRaised(!this.store.isHandRaised)
    } catch (error) {
      log.error('Failed to toggle hand', error)
    }
  }

  async toggleScreenShare(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall?.peerConnection) return

    const channelId = currentCall.channelId
    const pc = currentCall.peerConnection

    try {
      if (currentCall.screenStream) {
        // Stop screen sharing
        await this.stopLocalScreenShare()
        await callRepository.toggleScreenShare(channelId)
        await this.renegotiate(channelId, pc)
        this.store.setIsScreenSharing(false)
      } else {
        // Start screen sharing
        const stream = await navigator.mediaDevices.getDisplayMedia({
          video: true,
          audio: false,
        })

        this.store.setScreenStream(stream)
        const [videoTrack] = stream.getVideoTracks()

        if (videoTrack) {
          videoTrack.contentHint = 'detail'
          videoTrack.onended = () => {
            if (this.store.currentCall?.screenStream) {
              void this.toggleScreenShare()
            }
          }

          if (currentCall.screenSender) {
            await currentCall.screenSender.replaceTrack(videoTrack)
          } else {
            const sender = pc.addTrack(videoTrack, stream)
            this.store.setScreenSender(sender)
          }
        }

        await callRepository.toggleScreenShare(channelId)
        await this.renegotiate(channelId, pc)
        this.store.setIsScreenSharing(true)
      }
    } catch (error) {
      log.error('Failed to toggle screen share', error)
      await this.stopLocalScreenShare()
      this.store.setIsScreenSharing(false)
    }
  }

  async sendReaction(emoji: string): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    try {
      await callRepository.sendReaction(currentCall.channelId, emoji)
    } catch (error) {
      log.error('Failed to send reaction', error)
    }
  }

  // Host controls
  async hostMute(sessionId: SessionId): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    try {
      await callRepository.hostMute(currentCall.channelId, sessionId)
    } catch (error) {
      log.error('Failed to host mute', error)
    }
  }

  async hostMuteOthers(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    try {
      await callRepository.hostMuteOthers(currentCall.channelId)
      this.toast.success('Muted all', 'All other participants have been muted')
    } catch (error) {
      log.error('Failed to host mute others', error)
    }
  }

  async hostRemove(sessionId: SessionId): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    try {
      await callRepository.hostRemove(currentCall.channelId, sessionId)
    } catch (error) {
      log.error('Failed to host remove', error)
    }
  }

  async ringUsers(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    try {
      await callRepository.ringUsers(currentCall.channelId)
      this.toast.success('Ringing participants', 'Other channel members have been notified')
    } catch (error) {
      log.error('Failed to ring users', error)
    }
  }

  // WebSocket event handlers
  handleUserJoined(channelId: ChannelId): void {
    void this.loadCallForChannel(channelId)
  }

  handleUserLeft(channelId: ChannelId): void {
    void this.loadCallForChannel(channelId)
  }

  handleUserMuted(channelId: ChannelId, userId: UserId, muted: boolean): void {
    if (this.store.currentCall?.channelId === channelId) {
      // Check if it's us
      const myUserId = this.store.currentUserId
      if (userId === myUserId) {
        this.store.setIsMuted(muted)
        // Update local tracks
        this.store.currentCall.localStream?.getAudioTracks().forEach(track => {
          track.enabled = !muted
        })
      }
      void this.loadCallForChannel(channelId)
    }
  }

  handleHandRaised(channelId: ChannelId, userId: UserId): void {
    if (this.store.currentCall?.channelId === channelId) {
      const myUserId = this.store.currentUserId
      if (userId === myUserId) {
        this.store.setIsHandRaised(true)
      }
      void this.loadCallForChannel(channelId)
    }
  }

  handleHandLowered(channelId: ChannelId, userId: UserId): void {
    if (this.store.currentCall?.channelId === channelId) {
      const myUserId = this.store.currentUserId
      if (userId === myUserId) {
        this.store.setIsHandRaised(false)
      }
      void this.loadCallForChannel(channelId)
    }
  }

  handleScreenShareOn(channelId: ChannelId, userId: UserId): void {
    if (this.store.currentCall?.channelId === channelId) {
      const myUserId = this.store.currentUserId
      if (userId === myUserId) {
        this.store.setIsScreenSharing(true)
      }
      void this.loadCallForChannel(channelId)
    }
  }

  handleScreenShareOff(channelId: ChannelId, userId: UserId): void {
    if (this.store.currentCall?.channelId === channelId) {
      const myUserId = this.store.currentUserId
      if (userId === myUserId) {
        this.store.setIsScreenSharing(false)
      }
      void this.loadCallForChannel(channelId)
    }
  }

  handleHostMuted(sessionId: SessionId): void {
    if (!this.store.currentCall) return
    if (sessionId === this.store.currentCall.mySessionId) {
      this.store.setIsMuted(true)
      this.store.currentCall.localStream?.getAudioTracks().forEach(track => {
        track.enabled = false
      })
      this.toast.info('Host muted you', 'Your microphone has been disabled by the host')
    }
  }

  handleHostRemoved(sessionId: SessionId): void {
    if (!this.store.currentCall) return
    if (sessionId === this.store.currentCall.mySessionId) {
      void this.leaveCall()
      this.toast.error('Removed from call', 'You have been removed from the call by the host')
    }
  }

  handleHostChanged(channelId: ChannelId, hostId: UserId): void {
    if (this.store.currentCall?.channelId === channelId) {
      this.store.updateCurrentCallHost(hostId)
    }
  }

  handleIncomingCall(channelId: ChannelId, callerId: UserId): void {
    if (this.store.isInCall) return
    this.store.setIncomingCall({ channelId, callerId })
  }

  handleCallEnded(channelId: ChannelId): void {
    if (this.store.currentCall?.channelId === channelId) {
      void this.leaveCall()
    }
    this.store.removeActiveCall(channelId)
  }

  handleVoiceOn(sessionId: SessionId): void {
    this.store.addSpeakingParticipant(sessionId)
  }

  handleVoiceOff(sessionId: SessionId): void {
    this.store.removeSpeakingParticipant(sessionId)
  }

  handleSignalingEvent(data: Record<string, unknown>): void {
    const currentCall = this.store.currentCall
    if (!currentCall?.peerConnection) return

    const channelId = (data.channel_id_raw || data.channel_id) as string | undefined
    if (channelId !== currentCall.channelId) return

    const signal = data.signal as Record<string, unknown> | undefined
    if (!signal?.type) return

    const pc = currentCall.peerConnection

    if (signal.type === 'ice-candidate' && signal.candidate) {
      void pc
        .addIceCandidate({
          candidate: signal.candidate as string,
          sdpMid: (signal.sdp_mid ?? null) as string | null,
          sdpMLineIndex: (signal.sdp_mline_index ?? null) as number | null,
        })
        .catch(error => {
          log.error('Failed to handle signaling event', error)
        })
    }
  }

  dismissIncomingCall(): void {
    this.store.setIncomingCall(null)
  }

  toggleExpanded(): void {
    this.store.setIsExpanded(!this.store.isExpanded)
  }

  // Private helpers
  private async initializeWebRTC(channelId: ChannelId, iceServers: RTCIceServer[]): Promise<void> {
    try {
      // Get user media
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: true,
        video: false, // Audio only for now
      })

      // Calls start muted server-side, keep local tracks consistent
      stream.getAudioTracks().forEach(track => {
        track.enabled = false
      })

      this.store.setLocalStream(stream)

      // Create peer connection
      const pc = new RTCPeerConnection({
        iceServers: iceServers.length > 0 ? iceServers : [{ urls: 'stun:stun.l.google.com:19302' }],
      })

      this.store.setPeerConnection(pc)

      // Add local stream tracks
      stream.getTracks().forEach(track => {
        pc.addTrack(track, stream)
      })

      // Handle incoming tracks
      pc.ontrack = event => {
        this.handleRemoteTrack(event)
      }

      // Handle ICE candidates
      pc.onicecandidate = async event => {
        if (event.candidate) {
          await callRepository.sendIceCandidate(
            channelId,
            JSON.stringify(event.candidate),
            event.candidate.sdpMid || undefined,
            event.candidate.sdpMLineIndex || undefined
          )
        }
      }

      // Create and send offer
      await this.createAndSendOffer(channelId, pc)
    } catch (error) {
      log.error('WebRTC initialization failed', error)
      throw error
    }
  }

  private async createAndSendOffer(channelId: ChannelId, pc: RTCPeerConnection): Promise<void> {
    this.applyVideoCodecPreferences(pc)

    const offer = await pc.createOffer()
    const rawSdp = offer.sdp || ''
    const preparedSdp = this.prepareOfferSdp(rawSdp)

    let selectedSdp = preparedSdp
    try {
      await pc.setLocalDescription({
        type: 'offer',
        sdp: preparedSdp,
      })
    } catch (error) {
      // Brave can reject aggressively munged SDP. Fall back to the
      // browser-generated offer so call setup still succeeds.
      log.warn('Prepared SDP rejected by browser, retrying with original SDP', error)
      selectedSdp = rawSdp
      await pc.setLocalDescription({
        type: 'offer',
        sdp: rawSdp,
      })
    }

    const answer = await callRepository.sendOffer(channelId, selectedSdp)

    await pc.setRemoteDescription(
      new RTCSessionDescription({
        type: 'answer',
        sdp: answer.sdp,
      })
    )
  }

  private handleRemoteTrack(event: RTCTrackEvent): void {
    log.debug('Received remote track:', event.track.kind, event.streams)

    if (event.streams && event.streams[0]) {
      const remoteStream = event.streams[0]
      this.store.addRemoteStream(remoteStream.id, remoteStream)
    } else {
      // Some browsers can emit ontrack without an attached stream.
      // Build a synthetic stream so screen-share video is still renderable.
      const syntheticStreamId = `track-${event.track.id}`
      const existing = this.store.getRemoteStream(syntheticStreamId)
      const synthetic = existing || new MediaStream()
      const hasTrack = synthetic.getTracks().some(t => t.id === event.track.id)

      if (!hasTrack) {
        synthetic.addTrack(event.track)
      }

      this.store.addRemoteStream(syntheticStreamId, synthetic)
    }

    // Handle track ended
    event.track.onended = () => {
      this.handleTrackEnded(event.track.id)
    }
  }

  private handleTrackEnded(trackId: string): void {
    for (const [key, stream] of this.store.remoteStreams) {
      const remainingTracks = stream.getTracks().filter((t: MediaStreamTrack) => t.id !== trackId)

      if (remainingTracks.length === stream.getTracks().length) {
        continue // Track not in this stream
      }

      if (remainingTracks.length === 0) {
        this.store.removeRemoteStream(key)
      } else {
        const replacement = new MediaStream()
        remainingTracks.forEach((t: MediaStreamTrack) => replacement.addTrack(t))
        this.store.addRemoteStream(key, replacement)
      }
    }
  }

  private cleanupWebRTC(): void {
    const currentCall = this.store.currentCall
    if (!currentCall) return

    if (currentCall.peerConnection) {
      currentCall.peerConnection.close()
    }

    if (currentCall.localStream) {
      currentCall.localStream.getTracks().forEach(track => track.stop())
    }

    if (currentCall.screenStream) {
      currentCall.screenStream.getTracks().forEach(track => track.stop())
    }

    this.store.clearRemoteStreams()
    this.store.clearSpeakingParticipants()
  }

  private async stopLocalScreenShare(): Promise<void> {
    const currentCall = this.store.currentCall
    if (!currentCall?.screenStream) return

    currentCall.screenStream.getTracks().forEach(track => {
      track.onended = null
      track.stop()
    })

    this.store.setScreenStream(null)

    const sender = currentCall.screenSender
    if (sender) {
      await sender.replaceTrack(null)
    }
  }

  private async renegotiate(channelId: ChannelId, pc: RTCPeerConnection): Promise<void> {
    await this.createAndSendOffer(channelId, pc)
  }

  private syncSelfFlags(call: CallState): void {
    const myUserId = this.store.currentUserId
    if (!myUserId) return

    // Find my session
    let mySession: { sessionId: SessionId; isMuted: boolean; raisedHandAt: number } | null = null
    for (const [sessionId, participant] of call.participants) {
      if (participant.userId === myUserId) {
        mySession = {
          sessionId,
          isMuted: participant.isMuted,
          raisedHandAt: participant.raisedHandAt,
        }
        break
      }
    }

    if (mySession) {
      this.store.setIsMuted(mySession.isMuted)
      this.store.setIsHandRaised(mySession.raisedHandAt > 0)
      this.store.setMySessionId(mySession.sessionId)
    }
  }

  private shouldUseSimulcast(): boolean {
    return this.store.config?.enableSimulcast === true
  }

  private prepareOfferSdp(sdp: string): string {
    if (this.shouldUseSimulcast()) {
      return sdp
    }
    return this.stripSimulcastFromSdp(sdp)
  }

  private stripSimulcastFromSdp(sdp: string): string {
    if (!sdp) return sdp

    const lines = sdp
      .split(/\r\n|\n/)
      .map(line => line.trim())
      .filter(line => line.length > 0)

    let removedAny = false
    const filtered = lines.filter(line => {
      const lower = line.toLowerCase()
      if (lower.startsWith('a=simulcast:')) {
        removedAny = true
        return false
      }
      if (lower.startsWith('a=rid:')) {
        removedAny = true
        return false
      }
      if (lower.includes('urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id')) {
        removedAny = true
        return false
      }
      if (lower.includes('urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id')) {
        removedAny = true
        return false
      }
      return true
    })

    // Preserve browser-generated SDP when nothing was removed
    if (!removedAny) {
      return sdp
    }

    return `${filtered.join('\r\n')}\r\n`
  }

  private applyVideoCodecPreferences(pc: RTCPeerConnection): void {
    if (this.shouldUseSimulcast()) return

    const capabilities = RTCRtpSender.getCapabilities?.('video')
    const codecs = capabilities?.codecs || []
    if (codecs.length === 0) return

    const primary = codecs.filter(codec => {
      const mime = codec.mimeType.toLowerCase()
      return mime === 'video/vp8' || mime === 'video/h264'
    })
    if (primary.length === 0) return

    const repair = codecs.filter(codec => {
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
        log.debug('Failed to set codec preferences on transceiver', error)
      }
    }
  }
}

export const callService = new CallService()
