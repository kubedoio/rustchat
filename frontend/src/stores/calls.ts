import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import callsApi, { type Call, type CallParticipant } from '../api/calls'
// import { useWebSocket } from '../composables/useWebSocket'

export const useCallsStore = defineStore('calls', () => {
    const activeCall = ref<Call | null>(null)
    const participants = ref<CallParticipant[]>([])
    const isExpanded = ref(false)
    const incomingCall = ref<{ channelId: string, callerId: string } | null>(null)

    // const { send } = useWebSocket() // Used for signaling later

    // Getters
    const isInCall = computed(() => !!activeCall.value)
    const currentParticipants = computed(() => participants.value.filter(p => !p.left_at))

    // Actions
    async function startCall(channelId: string, type: 'audio' | 'video' = 'audio') {
        try {
            const response = await callsApi.createCall(channelId, type)
            activeCall.value = response.data
            await joinCall(response.data.id)
            isExpanded.value = true
        } catch (error) {
            console.error('Failed to start call', error)
            throw error
        }
    }

    async function joinCall(callId: string) {
        try {
            await callsApi.joinCall(callId)
            const session = await callsApi.getCall(callId)
            activeCall.value = session.data.call
            participants.value = session.data.participants
            isExpanded.value = true
        } catch (error) {
            console.error('Failed to join call', error)
        }
    }

    async function leaveCall() {
        if (!activeCall.value) return
        try {
            await callsApi.leaveCall(activeCall.value.id)
            activeCall.value = null
            participants.value = []
            isExpanded.value = false
        } catch (error) {
            console.error('Failed to leave call', error)
        }
    }

    async function endCall() {
        if (!activeCall.value) return
        try {
            await callsApi.endCall(activeCall.value.id)
            activeCall.value = null
            participants.value = []
        } catch (error) {
            console.error('Failed to end call', error)
        }
    }

    function setIncomingCall(call: { channelId: string, callerId: string } | null) {
        incomingCall.value = call
    }

    return {
        activeCall,
        participants,
        isExpanded,
        incomingCall,
        isInCall,
        currentParticipants,
        startCall,
        joinCall,
        leaveCall,
        endCall,
        setIncomingCall
    }
})
