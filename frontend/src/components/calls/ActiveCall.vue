<script setup lang="ts">
import { useCallsStore } from '../../stores/calls'
import { useAuthStore } from '../../stores/auth'
import { useChannelStore } from '../../stores/channels'
import { computed, ref, watchEffect } from 'vue'
import { 
    Maximize2, 
    Minimize2, 
    Mic, 
    MicOff, 
    PhoneOff, 
    Hand, 
    Monitor,
    MoreVertical,
    Users,
    Bell,
    Trash2,
    Shield
} from 'lucide-vue-next'

const callsStore = useCallsStore()
const authStore = useAuthStore()
const channelStore = useChannelStore()

const activeCall = computed(() => callsStore.currentCall)
const isExpanded = computed(() => callsStore.isExpanded)
const isMuted = computed(() => callsStore.isMuted)
const isHandRaised = computed(() => callsStore.isHandRaised)
const isScreenSharing = computed(() => callsStore.isScreenSharing)
const participants = computed(() => callsStore.currentCallParticipants)

const showParticipants = ref(false)
const showMenu = ref(false)
const participantMenuOpen = ref<string | null>(null) // session_id for which menu is open

const speakingParticipants = computed(() => callsStore.speakingParticipants)
const screenVideoRef = ref<HTMLVideoElement | null>(null)

const screenShareStream = computed(() => {
    if (!activeCall.value) return null
    
    // If local user is sharing, use local screen stream
    if (activeCall.value.screenStream) return activeCall.value.screenStream
    
    // Prefer the stream that matches the call state's screen sharing session.
    const remoteStreams = Array.from(activeCall.value.remoteStreams.values())
    const screenSessionId = activeCall.value.call.screen_sharing_session_id
    if (screenSessionId) {
        const matched = remoteStreams.find((stream) => {
            if (!stream.getVideoTracks().length) return false
            if (stream.id.includes(screenSessionId)) return true
            return stream.getVideoTracks().some((track) => track.id.includes(screenSessionId))
        })
        if (matched) return matched
    }

    // Fallback: in this audio-call UI, any remote video stream is likely screen share.
    return remoteStreams.find((stream) => stream.getVideoTracks().length > 0) || null
})

watchEffect(() => {
    if (screenVideoRef.value && screenShareStream.value) {
        screenVideoRef.value.srcObject = screenShareStream.value
    }
})

const channelName = computed(() => {
    if (!activeCall.value) return ''
    const channel = channelStore.channels.find(c => c.id === activeCall.value?.channelId)
    return channel?.name || 'Unknown Channel'
})

const isHost = computed(() => {
    if (!activeCall.value || !authStore.user) return false
    return activeCall.value.call.host_id === authStore.user.id ||
           activeCall.value.call.owner_id === authStore.user.id
})

const toggleExpand = () => {
    callsStore.toggleExpanded()
}

const handleHangup = () => {
    callsStore.leaveCall()
}

const handleEndCall = () => {
    callsStore.endCall()
}

const toggleMute = () => {
    callsStore.toggleMute()
}

const toggleHand = () => {
    callsStore.toggleHand()
}

const toggleScreenShare = () => {
    callsStore.toggleScreenShare()
}

const handleRingAll = () => {
    if (activeCall.value) {
        callsStore.ring(activeCall.value.channelId)
    }
}

const handleMuteAll = () => {
    callsStore.hostMuteOthers()
}

const handleHostMute = (sessionId: string) => {
    callsStore.hostMute(sessionId)
    participantMenuOpen.value = null
}

const handleHostRemove = (sessionId: string) => {
    callsStore.hostRemove(sessionId)
    participantMenuOpen.value = null
}

const formatDuration = (startAt: number) => {
    const elapsed = Math.floor((Date.now() - startAt) / 1000)
    const minutes = Math.floor(elapsed / 60)
    const seconds = elapsed % 60
    return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
}
</script>

<template>
    <div v-if="activeCall" 
         class="fixed transition-all duration-300 bg-bg-surface-1 border border-border-1 shadow-2xl rounded-xl overflow-hidden z-50 flex flex-col"
         :class="[
             isExpanded ? 'inset-4' : 'bottom-4 right-4 w-80'
         ]">
        
        <!-- Header -->
        <div class="flex items-center justify-between px-4 py-3 bg-app border-b border-border-1 shrink-0">
            <div class="flex items-center space-x-3 min-w-0">
                <span class="w-2.5 h-2.5 rounded-full bg-success animate-pulse shrink-0"></span>
                <div class="min-w-0">
                    <h3 class="text-text-1 font-medium text-sm truncate">{{ channelName }}</h3>
                    <p class="text-xs text-text-3">
                        {{ participants.length }} participant{{ participants.length !== 1 ? 's' : '' }}
                        • {{ formatDuration(activeCall.call.start_at) }}
                    </p>
                </div>
            </div>
            <div class="flex items-center space-x-1 shrink-0">
                <button 
                    @click="toggleExpand" 
                    class="p-1.5 text-text-3 hover:text-text-1 rounded hover:bg-bg-surface-1 transition-colors"
                    :title="isExpanded ? 'Minimize' : 'Maximize'"
                >
                    <Minimize2 v-if="isExpanded" class="w-4 h-4" />
                    <Maximize2 v-else class="w-4 h-4" />
                </button>
            </div>
        </div>

        <!-- Participants List (Expanded) -->
        <div v-if="isExpanded" class="flex-1 overflow-hidden flex">
            <!-- Main Area - Could show active speaker or screen share here -->
            <div class="flex-1 bg-app flex items-center justify-center relative overflow-hidden">
                <div v-if="screenShareStream" class="absolute inset-0 flex items-center justify-center bg-app">
                    <video 
                        ref="screenVideoRef" 
                        autoplay 
                        playsinline 
                        class="max-w-full max-h-full object-contain"
                    ></video>
                    <div v-if="activeCall.screenStream" class="absolute top-4 left-4 bg-brand px-3 py-1 rounded text-xs font-medium text-text-1 shadow-lg">
                        You are sharing your screen
                    </div>
                </div>
                <div v-else class="text-center">
                    <div class="w-24 h-24 rounded-full bg-bg-surface-2 flex items-center justify-center mb-4 mx-auto">
                        <Users class="w-12 h-12 text-text-3" />
                    </div>
                    <p class="text-text-3 text-sm">Audio Call in Progress</p>
                    <p class="text-text-3 text-xs mt-1">{{ participants.length }} participants</p>
                </div>
            </div>
            
            <!-- Participants Sidebar -->
            <div v-if="showParticipants" class="w-64 bg-bg-surface-1 border-l border-border-1 overflow-y-auto">
                <div class="p-3 border-b border-border-1">
                    <h4 class="text-text-1 font-medium text-sm">Participants</h4>
                </div>
                <div class="p-2 space-y-1">
                    <div v-for="participant in participants" :key="participant.session_id"
                         class="flex items-center space-x-2 p-2 rounded hover:bg-bg-surface-2 relative group">
                        <div class="w-8 h-8 rounded-full flex items-center justify-center transition-all duration-300"
                             :class="[
                                 speakingParticipants.has(participant.session_id) 
                                     ? 'bg-success/20 ring-2 ring-success' 
                                     : 'bg-brand/20'
                             ]">
                            <span class="text-brand text-xs font-medium">
                                {{ participant.user_id.slice(0, 2).toUpperCase() }}
                            </span>
                        </div>
                        <div class="flex-1 min-w-0">
                            <div class="flex items-center space-x-1">
                                <p class="text-text-1 text-sm truncate">
                                    {{ (participant.user_id_raw || participant.user_id) === authStore.user?.id ? 'You' : (participant.display_name || participant.username || participant.user_id.slice(0, 8)) }}
                                </p>
                                <Shield v-if="(participant.user_id_raw || participant.user_id) === activeCall.call.host_id_raw || participant.user_id === activeCall.call.host_id" 
                                        class="w-3 h-3 text-brand" title="Host" />
                            </div>
                        </div>
                        <div class="flex items-center space-x-1">
                            <MicOff v-if="!participant.unmuted" class="w-3.5 h-3.5 text-text-3" />
                            <Hand v-if="participant.raised_hand > 0" class="w-3.5 h-3.5 text-warning" />
                            
                            <!-- Participant Moderation Menu -->
                            <div v-if="isHost && participant.user_id !== authStore.user?.id" class="relative ml-1">
                                <button @click="participantMenuOpen = participantMenuOpen === participant.session_id ? null : participant.session_id"
                                        class="p-1 text-text-3 hover:text-text-1 rounded hover:bg-bg-surface-1 opacity-0 group-hover:opacity-100 transition-opacity">
                                    <MoreVertical class="w-3.5 h-3.5" />
                                </button>
                                
                                <div v-if="participantMenuOpen === participant.session_id" 
                                     class="absolute right-0 top-full mt-1 w-32 bg-bg-surface-2 border border-border-1 rounded shadow-xl z-50 py-1">
                                    <button @click="handleHostMute(participant.session_id)" 
                                            class="w-full px-3 py-1.5 text-left text-xs text-text-2 hover:bg-bg-surface-2 flex items-center">
                                        <MicOff class="w-3 h-3 mr-2" />
                                        Mute
                                    </button>
                                    <button @click="handleHostRemove(participant.session_id)" 
                                            class="w-full px-3 py-1.5 text-left text-xs text-danger hover:bg-bg-surface-2 flex items-center">
                                        <Trash2 class="w-3 h-3 mr-2" />
                                        Remove
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Compact Mode - Participants Preview -->
        <div v-else class="flex-1 bg-app p-3 overflow-hidden">
            <div class="flex items-center space-x-2">
                <div v-for="(participant, idx) in participants.slice(0, 5)" :key="participant.session_id"
                     class="w-10 h-10 rounded-full flex items-center justify-center shrink-0 transition-all duration-300"
                     :class="[
                         speakingParticipants.has(participant.session_id) 
                             ? 'ring-2 ring-success bg-success/20 z-20' 
                             : 'bg-brand/20'
                     ]"
                     :style="{ marginLeft: idx > 0 ? '-0.5rem' : '0', zIndex: speakingParticipants.has(participant.session_id) ? 30 : (10 - idx) }">
                    <span class="text-brand text-xs font-medium">
                        {{ participant.user_id.slice(0, 2).toUpperCase() }}
                    </span>
                </div>
                <div v-if="participants.length > 5" 
                     class="w-10 h-10 rounded-full bg-bg-surface-2 flex items-center justify-center shrink-0 -ml-2">
                    <span class="text-text-3 text-xs">+{{ participants.length - 5 }}</span>
                </div>
            </div>
            <div class="mt-2 text-xs text-text-3">
                {{ isMuted ? 'Muted' : 'Unmuted' }}
                <span v-if="isHandRaised" class="ml-2 text-warning">Hand raised</span>
            </div>
        </div>

        <!-- Controls -->
        <div class="flex items-center justify-center space-x-3 px-4 py-3 bg-app border-t border-border-1 shrink-0">
            <!-- Mute/Unmute -->
            <button 
                @click="toggleMute"
                :class="[
                    'w-12 h-12 rounded-full flex items-center justify-center transition-all',
                    isMuted 
                        ? 'bg-danger/20 text-danger hover:bg-danger/30' 
                        : 'bg-bg-surface-2 text-text-1 hover:bg-bg-surface-1'
                ]"
                :title="isMuted ? 'Unmute' : 'Mute'"
            >
                <MicOff v-if="isMuted" class="w-5 h-5" />
                <Mic v-else class="w-5 h-5" />
            </button>

            <!-- Raise Hand -->
            <button 
                @click="toggleHand"
                :class="[
                    'w-10 h-10 rounded-full flex items-center justify-center transition-all',
                    isHandRaised 
                        ? 'bg-warning/20 text-warning' 
                        : 'bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1'
                ]"
                :title="isHandRaised ? 'Lower hand' : 'Raise hand'"
            >
                <Hand class="w-4 h-4" />
            </button>

            <!-- Screen Share -->
            <button 
                @click="toggleScreenShare"
                :class="[
                    'w-10 h-10 rounded-full flex items-center justify-center transition-all',
                    isScreenSharing 
                        ? 'bg-success/20 text-success' 
                        : 'bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1'
                ]"
                :title="isScreenSharing ? 'Stop sharing' : 'Share screen'"
            >
                <Monitor class="w-4 h-4" />
            </button>

            <!-- Participants Toggle (Expanded mode only) -->
            <button 
                v-if="isExpanded"
                @click="showParticipants = !showParticipants"
                :class="[
                    'w-10 h-10 rounded-full flex items-center justify-center transition-all',
                    showParticipants 
                        ? 'bg-brand/20 text-brand' 
                        : 'bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1'
                ]"
                :title="showParticipants ? 'Hide participants' : 'Show participants'"
            >
                <Users class="w-4 h-4" />
            </button>

            <!-- More Options -->
            <div class="relative">
                <button 
                    @click="showMenu = !showMenu"
                    class="w-10 h-10 rounded-full flex items-center justify-center bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1 transition-all"
                    title="More options"
                >
                    <MoreVertical class="w-4 h-4" />
                </button>
                
                <div v-if="showMenu" 
                     class="absolute bottom-full mb-2 right-0 w-48 bg-bg-surface-2 border border-border-1 rounded-lg shadow-xl py-1 z-50">
                    <div class="fixed inset-0 z-[-1]" @click="showMenu = false"></div>
                    
                    <template v-if="isHost">
                        <button 
                            @click="handleMuteAll(); showMenu = false"
                            class="w-full px-4 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center"
                        >
                            <MicOff class="w-4 h-4 mr-2" />
                            Mute All
                        </button>
                        <button 
                            @click="handleRingAll(); showMenu = false"
                            class="w-full px-4 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center"
                        >
                            <Bell class="w-4 h-4 mr-2" />
                            Ring Everyone
                        </button>
                        <div class="my-1 border-t border-border-1"></div>
                        <button 
                            @click="handleEndCall(); showMenu = false"
                            class="w-full px-4 py-2 text-left text-sm text-danger hover:bg-bg-surface-2 flex items-center"
                        >
                            <PhoneOff class="w-4 h-4 mr-2" />
                            End Call for Everyone
                        </button>
                    </template>
                    <template v-else>
                        <p class="px-4 py-2 text-xs text-text-3 italic text-center">No host options available</p>
                    </template>
                </div>
            </div>

            <!-- Hangup -->
            <button 
                @click="handleHangup"
                class="w-12 h-12 rounded-full bg-danger hover:bg-danger/90 text-text-1 flex items-center justify-center transition-all"
                title="Leave call"
            >
                <PhoneOff class="w-5 h-5" />
            </button>
        </div>
    </div>
</template>
