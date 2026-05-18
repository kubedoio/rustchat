import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import callsApi, { type CallState, type CallsConfig } from '../api/calls'
import { useWebSocket } from '../composables/useWebSocket'
import { useToast } from '../composables/useToast'
import { useAuthStore } from './auth'
import { getErrorMessage, isNotFoundError } from '../core/errors/errorUtils'

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

export const useCallsStore = defineStore('calls', () => {
    const { onEvent } = useWebSocket()
    const toast = useToast()
    const authStore = useAuthStore()

    // State
    const callsConfig = ref<CallsConfig | null>(null)
    const activeCalls = ref<Map<string, CallState>>(new Map())
    const currentCall = ref<CurrentCall | null>(null)
    const isExpanded = ref(false)
    const incomingCall = ref<{ channelId: string, callerId: string } | null>(null)
    const isMuted = ref(true)
    const isHandRaised = ref(false)
    const isScreenSharing = ref(false)
    const speakingParticipants = ref<Set<string>>(new Set())
    
    // Device preferences (stored in localStorage)
    const preferredAudioInput = ref<string>(localStorage.getItem('calls_preferred_audio_input') || '')
    const preferredAudioOutput = ref<string>(localStorage.getItem('calls_preferred_audio_output') || '')
    const preferredVideoDevice = ref<string>(localStorage.getItem('calls_preferred_video_device') || '')

    // Getters
    const isInCall = computed(() => !!currentCall.value)
    const currentCallParticipants = computed(() => {
        if (!currentCall.value) return []
        return Object.values(currentCall.value.call.sessions || {})
    })

    const currentChannelCall = computed(() => (channelId: string) => {
        return activeCalls.value.get(channelId)
    })

    function eventPayload(data: unknown): Record<string, unknown> {
        return data && typeof data === 'object' ? data as Record<string, unknown> : {}
    }

    function readEventChannelId(data: unknown): string | undefined {
        const payload = eventPayload(data)
        return (payload.channel_id_raw || payload.channel_id) as string | undefined
    }

    function readEventUserId(data: unknown): string | undefined {
        const payload = eventPayload(data)
        return (payload.user_id_raw || payload.user_id) as string | undefined
    }

    function readEventSessionId(data: unknown): string | undefined {
        const payload = eventPayload(data)
        return (payload.session_id_raw || payload.session_id) as string | undefined
    }

    function readEventBoolean(data: unknown, key: string): boolean | undefined {
        const value = eventPayload(data)[key]
        return typeof value === 'boolean' ? value : undefined
    }

    function readEventString(data: unknown, ...keys: string[]): string | undefined {
        const payload = eventPayload(data)
        for (const key of keys) {
            const value = payload[key]
            if (typeof value === 'string' && value.length > 0) {
                return value
            }
        }
        return undefined
    }

    function findMySessionId(call: CallState): string {
        const myUserId = authStore.user?.id
        if (!myUserId) {
            return ''
        }

        const selfSession = Object.values(call.sessions || {}).find(
            (session) => (session.user_id_raw || session.user_id) === myUserId
        )
        return selfSession?.session_id || ''
    }

    function syncSelfCallFlags(call: CallState) {
        const mySessionId = findMySessionId(call)
        const mySession = mySessionId ? call.sessions?.[mySessionId] : undefined
        isMuted.value = mySession ? !mySession.unmuted : true
        isHandRaised.value = !!mySession && mySession.raised_hand > 0
        isScreenSharing.value = (call.screen_sharing_id_raw || call.screen_sharing_id) === authStore.user?.id
    }

    // WebSocket Event Listeners for Call Events
    onEvent('custom_com.mattermost.calls_call_start', (data) => {
        console.log('Call started:', data)
        // Reload calls to get updated state
        loadCalls()
    })

    onEvent('custom_com.mattermost.calls_call_end', (data) => {
        console.log('Call ended:', data)
        const eventChannelId = readEventChannelId(data)
        if (eventChannelId && currentCall.value?.channelId === eventChannelId) {
            leaveCall()
        }
        if (eventChannelId) {
            activeCalls.value.delete(eventChannelId)
        }
    })

    onEvent('custom_com.mattermost.calls_user_joined', (data) => {
        console.log('User joined call:', data)
        const eventChannelId = readEventChannelId(data)
        if (eventChannelId) {
            loadCallForChannel(eventChannelId)
        }
    })

    onEvent('custom_com.mattermost.calls_user_left', (data) => {
        console.log('User left call:', data)
        const eventChannelId = readEventChannelId(data)
        if (eventChannelId) {
            loadCallForChannel(eventChannelId)
        }
    })

    onEvent('custom_com.mattermost.calls_user_muted', (data) => {
        console.log('User muted:', data)
        const eventChannelId = readEventChannelId(data)
        if (eventChannelId && currentCall.value?.channelId === eventChannelId) {
            const userId = readEventUserId(data)
            if (userId === authStore.user?.id) {
                const muted = readEventBoolean(data, 'muted') ?? true
                isMuted.value = muted
                // Update local tracks
                const active = currentCall.value
                if (active) {
                    active.localStream?.getAudioTracks().forEach(track => {
                        track.enabled = !muted
                    })
                }
            }
            loadCallForChannel(eventChannelId)
        }
    })

    onEvent('custom_com.mattermost.calls_user_unmuted', (data) => {
        console.log('User unmuted:', data)
        const eventChannelId = readEventChannelId(data)
        if (!eventChannelId || currentCall.value?.channelId !== eventChannelId) {
            return
        }

        const userId = readEventUserId(data)
        if (userId === authStore.user?.id) {
            isMuted.value = false
            currentCall.value.localStream?.getAudioTracks().forEach(track => {
                track.enabled = true
            })
        }
        loadCallForChannel(eventChannelId)
    })

    onEvent('custom_com.mattermost.calls_raise_hand', (data) => {
        console.log('Hand raised:', data)
        const eventChannelId = readEventChannelId(data)
        if (!eventChannelId || currentCall.value?.channelId !== eventChannelId) {
            return
        }

        const userId = readEventUserId(data)
        if (userId === authStore.user?.id) {
            isHandRaised.value = true
        }
        loadCallForChannel(eventChannelId)
    })

    onEvent('custom_com.mattermost.calls_lower_hand', (data) => {
        console.log('Hand lowered:', data)
        const eventChannelId = readEventChannelId(data)
        if (eventChannelId && currentCall.value?.channelId === eventChannelId) {
            const userId = readEventUserId(data)
            if (userId === authStore.user?.id) {
                isHandRaised.value = false
            }
            loadCallForChannel(eventChannelId)
        }
    })

    onEvent('custom_com.mattermost.calls_user_voice_on', (data) => {
        const sessionId = readEventSessionId(data)
        if (sessionId) {
            speakingParticipants.value.add(sessionId)
        }
    })

    onEvent('custom_com.mattermost.calls_user_voice_off', (data) => {
        const sessionId = readEventSessionId(data)
        if (sessionId) {
            speakingParticipants.value.delete(sessionId)
        }
    })

    onEvent('custom_com.mattermost.calls_host_mute', (data) => {
        if (!currentCall.value) return
        const sessionId = readEventSessionId(data)
        if (sessionId === currentCall.value?.mySessionId) {
            isMuted.value = true
            const active = currentCall.value
            if (active) {
                active.localStream?.getAudioTracks().forEach(track => {
                    track.enabled = false
                })
            }
            toast.info('Host muted you', 'Your microphone has been disabled by the host')
        }
    })

    onEvent('custom_com.mattermost.calls_host_removed', (data) => {
        if (!currentCall.value) return
        const sessionId = readEventSessionId(data)
        if (sessionId === currentCall.value?.mySessionId) {
            leaveCall()
            toast.error('Removed from call', 'You have been removed from the call by the host')
        }
    })

    onEvent('custom_com.mattermost.calls_host_changed', (data) => {
        const payload = eventPayload(data)
        const eventChannelId = readEventChannelId(payload)
        if (currentCall.value && currentCall.value.channelId === eventChannelId) {
            currentCall.value.call.host_id = readEventString(payload, 'host_id', 'host_id_raw') || ''
        }
    })

    onEvent('custom_com.mattermost.calls_ringing', (data) => {
        if (isInCall.value) return
        const eventChannelId = readEventChannelId(data)
        const callerId = readEventString(data, 'sender_id', 'sender_id_raw')
        if (eventChannelId && callerId) {
            setIncomingCall({ channelId: eventChannelId, callerId })
        }
    })

    onEvent('custom_com.mattermost.calls_screen_on', (data) => {
        console.log('Screen share on:', data)
        const eventChannelId = readEventChannelId(data)
        if (currentCall.value && currentCall.value.channelId === eventChannelId) {
            const userId = readEventUserId(data)
            if (userId === authStore.user?.id) {
                isScreenSharing.value = true
            }
            loadCallForChannel(eventChannelId)
        }
    })

    onEvent('custom_com.mattermost.calls_screen_off', (data) => {
        console.log('Screen share off:', data)
        const eventChannelId = readEventChannelId(data)
        if (currentCall.value && currentCall.value.channelId === eventChannelId) {
            const userId = readEventUserId(data)
            if (userId === authStore.user?.id) {
                isScreenSharing.value = false
            }
            loadCallForChannel(eventChannelId)
        }
    })

    onEvent('custom_com.mattermost.calls_signal', async (data) => {
        const active = currentCall.value
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
                    sdpMLineIndex: (signal.sdp_mline_index ?? null) as number | null
                })
            }
        } catch (error) {
            console.error('Failed to handle signaling event', error)
        }
    })

    // Actions
    async function loadConfig() {
        try {
            const { data } = await callsApi.getConfig()

            // If TURN is enabled but credentials are not in the main config, fetch them
            if (data.NeedsTURNCredentials) {
                try {
                    const { data: turnServers } = await callsApi.getTurnCredentials()
                    // Merge TURN servers into ICE config
                    data.ICEServersConfigs = [
                        ...data.ICEServersConfigs.filter(s => {
                            const urls = Array.isArray(s.urls) ? s.urls : [s.urls]
                            return !urls.some(url => url.toString().startsWith('turn:'))
                        }),
                        ...turnServers
                    ]
                } catch (error) {
                    console.error('Failed to fetch TURN credentials', error)
                }
            }

            callsConfig.value = data
            return data
        } catch (error) {
            console.error('Failed to load calls config', error)
            return null
        }
    }


    async function loadCalls() {
        try {
            const { data } = await callsApi.getCalls()
            activeCalls.value.clear()
            for (const channel of data) {
                if (channel.call) {
                    activeCalls.value.set(channel.channel_id, channel.call)
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
            if (!data) {
                return null
            }
            if (data.call) {
                activeCalls.value.set(channelId, data.call)
                if (currentCall.value?.channelId === channelId) {
                    currentCall.value.call = data.call
                    const mySessionId = findMySessionId(data.call)
                    if (mySessionId) {
                        currentCall.value.mySessionId = mySessionId
                    }
                    syncSelfCallFlags(data.call)
                }
            } else {
                activeCalls.value.delete(channelId)
            }
            return data
        } catch (error: unknown) {
            // Silently handle 404s as they just mean there's no active call in the channel
            if (!isNotFoundError(error)) {
                console.error('Failed to load call for channel', error)
            }
            return null
        }
    }

    async function startCall(channelId: string) {
        try {
            // First get config for ICE servers
            const config = await loadConfig()
            if (!config) {
                throw new Error('Calls plugin not available')
            }

            // Start the call on the server
            const { data: callData } = await callsApi.startCall(channelId)
            const channelState = await loadCallForChannel(channelId)
            if (!channelState?.call) {
                throw new Error('Call started but state could not be loaded')
            }

            const mySessionId = findMySessionId(channelState.call)
            if (!mySessionId) {
                throw new Error('Could not resolve your call session')
            }

            currentCall.value = {
                channelId,
                call: channelState.call,
                mySessionId,
                peerConnection: null,
                screenSender: null,
                localStream: null,
                screenStream: null,
                remoteStreams: new Map()
            }
            syncSelfCallFlags(channelState.call)

            // Initialize WebRTC
            const rtc = await initializeWebRTC(channelId, config.ICEServersConfigs)
            if (currentCall.value) {
                currentCall.value.peerConnection = rtc.pc
                currentCall.value.localStream = rtc.stream
            }

            isExpanded.value = true
            toast.success('Call started', 'You are now in a call')

            return callData
        } catch (error: unknown) {
            cleanupWebRTC()
            currentCall.value = null
            console.error('Failed to start call', error)
            toast.error('Failed to start call', getErrorMessage(error))
            throw error
        }
    }

    async function joinCall(channelId: string) {
        try {
            // Check if there's an active call
            const channelState = await loadCallForChannel(channelId)
            if (!channelState?.call) {
                throw new Error('No active call in this channel')
            }

            // Get config for ICE servers
            const config = await loadConfig()
            if (!config) {
                throw new Error('Calls plugin not available')
            }

            // Join the call on the server
            await callsApi.joinCall(channelId)

            const refreshedState = await loadCallForChannel(channelId)
            const callState = refreshedState?.call || channelState.call
            const mySessionId = findMySessionId(callState)
            if (!mySessionId) {
                throw new Error('Could not resolve your call session')
            }

            currentCall.value = {
                channelId,
                call: callState,
                mySessionId,
                peerConnection: null,
                screenSender: null,
                localStream: null,
                screenStream: null,
                remoteStreams: new Map()
            }
            syncSelfCallFlags(callState)

            // Initialize WebRTC
            const rtc = await initializeWebRTC(channelId, config.ICEServersConfigs)
            if (currentCall.value) {
                currentCall.value.peerConnection = rtc.pc
                currentCall.value.localStream = rtc.stream
            }

            isExpanded.value = true
            toast.success('Joined call', 'You are now in the call')

        } catch (error: unknown) {
            cleanupWebRTC()
            currentCall.value = null
            console.error('Failed to join call', error)
            toast.error('Failed to join call', getErrorMessage(error))
            throw error
        }
    }

    async function leaveCall() {
        if (!currentCall.value) return

        const channelId = currentCall.value.channelId

        try {
            // Clean up WebRTC
            cleanupWebRTC()

            // Leave on server
            await callsApi.leaveCall(channelId)

            currentCall.value = null
            isMuted.value = true
            isHandRaised.value = false
            isScreenSharing.value = false
            isExpanded.value = false
            speakingParticipants.value.clear()

        } catch (error) {
            console.error('Failed to leave call', error)
        }
    }

    async function endCall() {
        if (!currentCall.value) return

        const channelId = currentCall.value.channelId

        try {
            // Clean up WebRTC
            cleanupWebRTC()

            // End on server
            await callsApi.endCall(channelId)

            currentCall.value = null
            isMuted.value = true
            isHandRaised.value = false
            isScreenSharing.value = false
            isExpanded.value = false
            speakingParticipants.value.clear()

        } catch (error) {
            console.error('Failed to end call', error)
            toast.error('Failed to end call', 'Only the host can end the call')
        }
    }

    function shouldUseSimulcast(): boolean {
        return callsConfig.value?.EnableSimulcast === true
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

        // Preserve browser-generated SDP when nothing was removed to avoid
        // unnecessary rewrites that can break strict parsers.
        if (!removedAny) {
            return sdp
        }

        return `${filtered.join('\r\n')}\r\n`
    }

    function prepareOfferSdp(sdp: string): string {
        if (shouldUseSimulcast()) {
            return sdp
        }

        return stripSimulcastFromSdp(sdp)
    }

    function applyVideoCodecPreferences(pc: RTCPeerConnection) {
        // Keep video negotiation on broadly supported codecs for mobile interoperability.
        if (shouldUseSimulcast()) {
            return
        }

        const capabilities = RTCRtpSender.getCapabilities?.('video')
        const codecs = capabilities?.codecs || []
        if (codecs.length === 0) {
            return
        }

        const primary = codecs.filter((codec) => {
            const mime = codec.mimeType.toLowerCase()
            return mime === 'video/vp8' || mime === 'video/h264'
        })
        if (primary.length === 0) {
            return
        }

        const repair = codecs.filter((codec) => {
            const mime = codec.mimeType.toLowerCase()
            return mime === 'video/rtx' || mime === 'video/red' || mime === 'video/ulpfec'
        })
        const preferred = [...primary, ...repair]

        for (const transceiver of pc.getTransceivers()) {
            const senderKind = transceiver.sender?.track?.kind
            const receiverKind = transceiver.receiver?.track?.kind
            if (senderKind !== 'video' && receiverKind !== 'video') {
                continue
            }

            if (typeof transceiver.setCodecPreferences !== 'function') {
                continue
            }

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
            await pc.setLocalDescription({
                type: 'offer',
                sdp: preparedSdp
            })
        } catch (error) {
            // Brave can reject aggressively munged SDP. Fall back to the
            // browser-generated offer so call setup still succeeds.
            console.warn('Prepared SDP rejected by browser, retrying with original SDP', error)
            selectedSdp = rawSdp
            await pc.setLocalDescription({
                type: 'offer',
                sdp: rawSdp
            })
        }

        return callsApi.sendOffer(channelId, selectedSdp)
    }

    // WebRTC
    async function initializeWebRTC(channelId: string, iceServers: RTCIceServer[]) {
        try {
            // Get user media
            const stream = await navigator.mediaDevices.getUserMedia({
                audio: true,
                video: false // Audio only for now
            })
            // Calls start muted server-side, keep local tracks consistent.
            stream.getAudioTracks().forEach(track => {
                track.enabled = false
            })

            // Create peer connection
            const pc = new RTCPeerConnection({
                iceServers: (iceServers || []).length > 0 ? iceServers : [
                    { urls: 'stun:stun.l.google.com:19302' }
                ]
            })

            // Add local stream tracks
            stream.getTracks().forEach(track => {
                pc.addTrack(track, stream)
            })

            // Handle incoming tracks
            pc.ontrack = (event) => {
                console.log('Received remote track:', event.track.kind, event.streams)
                const active = currentCall.value
                if (!active) {
                    return
                }

                if (event.streams && event.streams[0]) {
                    const remoteStream = event.streams[0]
                    active.remoteStreams.set(remoteStream.id, remoteStream)
                } else {
                    // Some browsers can emit ontrack without an attached stream.
                    // Build a synthetic stream so screen-share video is still renderable.
                    const syntheticStreamId = `track-${event.track.id}`
                    const existing = active.remoteStreams.get(syntheticStreamId)
                    const synthetic = existing || new MediaStream()
                    const hasTrack = synthetic
                        .getTracks()
                        .some((track) => track.id === event.track.id)
                    if (!hasTrack) {
                        synthetic.addTrack(event.track)
                    }
                    active.remoteStreams.set(syntheticStreamId, synthetic)
                }

                event.track.onended = () => {
                    const call = currentCall.value
                    if (!call) {
                        return
                    }

                    for (const [key, stream] of call.remoteStreams.entries()) {
                        const remainingTracks = stream
                            .getTracks()
                            .filter((track) => track.id !== event.track.id)
                        if (remainingTracks.length === stream.getTracks().length) {
                            continue
                        }
                        if (remainingTracks.length === 0) {
                            call.remoteStreams.delete(key)
                            continue
                        }
                        const replacement = new MediaStream()
                        remainingTracks.forEach((track) => replacement.addTrack(track))
                        call.remoteStreams.set(key, replacement)
                    }
                }
            }

            // Handle ICE candidates
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

            // Set remote description
            await pc.setRemoteDescription(new RTCSessionDescription({
                type: 'answer',
                sdp: answer.sdp
            }))

            return {
                pc,
                stream
            }

        } catch (error) {
            console.error('WebRTC initialization failed', error)
            throw error
        }
    }

    function cleanupWebRTC() {
        if (currentCall.value?.peerConnection) {
            currentCall.value.peerConnection.close()
        }
        if (currentCall.value) {
            currentCall.value.screenSender = null
        }
        if (currentCall.value?.localStream) {
            currentCall.value.localStream.getTracks().forEach(track => track.stop())
        }
        if (currentCall.value?.screenStream) {
            currentCall.value.screenStream.getTracks().forEach(track => track.stop())
            currentCall.value.screenStream = null
        }
        currentCall.value?.remoteStreams.clear()
    }

    // Call controls
    async function toggleMute() {
        if (!currentCall.value) return

        const channelId = currentCall.value.channelId
        try {
            if (isMuted.value) {
                await callsApi.unmute(channelId)
                // Enable audio tracks
                currentCall.value.localStream?.getAudioTracks().forEach(track => {
                    track.enabled = true
                })
            } else {
                await callsApi.mute(channelId)
                // Disable audio tracks
                currentCall.value.localStream?.getAudioTracks().forEach(track => {
                    track.enabled = false
                })
            }
            isMuted.value = !isMuted.value
        } catch (error) {
            console.error('Failed to toggle mute', error)
        }
    }

    async function toggleHand() {
        if (!currentCall.value) return

        const channelId = currentCall.value.channelId
        try {
            if (isHandRaised.value) {
                await callsApi.lowerHand(channelId)
            } else {
                await callsApi.raiseHand(channelId)
            }
            isHandRaised.value = !isHandRaised.value
        } catch (error) {
            console.error('Failed to toggle hand', error)
        }
    }

    async function stopLocalScreenShare() {
        if (!currentCall.value?.screenStream) {
            return
        }

        currentCall.value.screenStream.getTracks().forEach(track => {
            track.onended = null
            track.stop()
        })
        currentCall.value.screenStream = null

        const sender = currentCall.value.screenSender
        if (sender) {
            await sender.replaceTrack(null)
        }
    }

    async function toggleScreenShare() {
        if (!currentCall.value || !currentCall.value.peerConnection) return

        const channelId = currentCall.value.channelId
        const pc = currentCall.value.peerConnection

        try {
            if (currentCall.value.screenStream) {
                // Stop screen sharing
                await stopLocalScreenShare()
                await callsApi.toggleScreenShare(channelId)
                await renegotiate(channelId, pc)
                isScreenSharing.value = false
            } else {
                // Start screen sharing
                const stream = await navigator.mediaDevices.getDisplayMedia({
                    video: true,
                    audio: false
                })

                currentCall.value.screenStream = stream
                const [videoTrack] = stream.getVideoTracks()
                if (videoTrack) {
                    videoTrack.contentHint = 'detail'
                    videoTrack.onended = () => {
                        if (currentCall.value?.screenStream) {
                            void toggleScreenShare()
                        }
                    }

                    if (currentCall.value.screenSender) {
                        await currentCall.value.screenSender.replaceTrack(videoTrack)
                    } else {
                        currentCall.value.screenSender = pc.addTrack(videoTrack, stream)
                    }
                }

                await callsApi.toggleScreenShare(channelId)
                await renegotiate(channelId, pc)
                isScreenSharing.value = true
            }
        } catch (error) {
            console.error('Failed to toggle screen share', error)
            await stopLocalScreenShare()
            isScreenSharing.value = false
        }
    }

    async function renegotiate(channelId: string, pc: RTCPeerConnection) {
        const { data: answer } = await createAndSendOffer(channelId, pc)
        await pc.setRemoteDescription(new RTCSessionDescription({
            type: 'answer',
            sdp: answer.sdp
        }))
    }

    async function sendReaction(emoji: string) {
        if (!currentCall.value) return

        try {
            await callsApi.sendReaction(currentCall.value.channelId, emoji)
        } catch (error) {
            console.error('Failed to send reaction', error)
        }
    }

    async function ring(channelId: string) {
        try {
            await callsApi.ringUsers(channelId)
            toast.success('Ringing participants', 'Other channel members have been notified')
        } catch (error) {
            console.error('Failed to ring users', error)
        }
    }

    async function hostMute(sessionId: string) {
        if (!currentCall.value) return
        try {
            await callsApi.hostMute(currentCall.value.channelId, sessionId)
        } catch (error) {
            console.error('Failed to host mute', error)
        }
    }

    async function hostMuteOthers() {
        if (!currentCall.value) return
        try {
            await callsApi.hostMuteOthers(currentCall.value.channelId)
            toast.success('Muted all', 'All other participants have been muted')
        } catch (error) {
            console.error('Failed to host mute others', error)
        }
    }

    async function hostRemove(sessionId: string) {
        if (!currentCall.value) return
        try {
            await callsApi.hostRemove(currentCall.value.channelId, sessionId)
        } catch (error) {
            console.error('Failed to host remove', error)
        }
    }

    // Set state
    function setIncomingCall(call: { channelId: string, callerId: string } | null) {
        incomingCall.value = call
    }

    function toggleExpanded() {
        isExpanded.value = !isExpanded.value
    }

    // Device preference actions
    async function setPreferredAudioInput(deviceId: string) {
        preferredAudioInput.value = deviceId
        localStorage.setItem('calls_preferred_audio_input', deviceId)
    }

    async function setPreferredAudioOutput(deviceId: string) {
        preferredAudioOutput.value = deviceId
        localStorage.setItem('calls_preferred_audio_output', deviceId)
    }

    async function setPreferredVideoDevice(deviceId: string) {
        preferredVideoDevice.value = deviceId
        localStorage.setItem('calls_preferred_video_device', deviceId)
    }

    function resetSessionState() {
        cleanupWebRTC()
        activeCalls.value.clear()
        currentCall.value = null
        incomingCall.value = null
        isExpanded.value = false
        isMuted.value = true
        isHandRaised.value = false
        isScreenSharing.value = false
        speakingParticipants.value.clear()
    }

    return {
        // State
        callsConfig,
        activeCalls,
        currentCall,
        isExpanded,
        incomingCall,
        isMuted,
        isHandRaised,
        isScreenSharing,
        speakingParticipants,
        preferredAudioInput,
        preferredAudioOutput,
        preferredVideoDevice,

        // Getters
        isInCall,
        currentCallParticipants,
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
        setPreferredVideoDevice
    }
})
