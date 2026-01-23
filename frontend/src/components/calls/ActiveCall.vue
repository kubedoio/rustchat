<script setup lang="ts">
import { useCallsStore } from '../../stores/calls'
import { useAuthStore } from '../../stores/auth'
import { computed } from 'vue'
import { Maximize2, Minimize2 } from 'lucide-vue-next'
import JitsiMeet from './JitsiMeet.vue'

const callsStore = useCallsStore()
const authStore = useAuthStore()

const activeCall = computed(() => callsStore.activeCall)
const isExpanded = computed(() => callsStore.isExpanded)

const roomName = computed(() => `rustchat-call-${activeCall.value?.id}`)
const userInfo = computed(() => ({
    displayName: authStore.user?.username || 'Guest',
    email: authStore.user?.email,
    avatarUrl: authStore.user?.avatar_url
}))

const toggleExpand = () => {
    callsStore.isExpanded = !callsStore.isExpanded
}

const handleHangup = () => {
    callsStore.leaveCall()
}
</script>

<template>
    <div v-if="activeCall" 
         class="fixed transition-all duration-300 bg-slate-900 border border-slate-700 shadow-2xl rounded-xl overflow-hidden z-50 flex flex-col"
         :class="[
             isExpanded ? 'inset-4' : 'bottom-4 right-4 w-96 h-64'
         ]">
        
        <!-- Header (Visible mainly in compact mode or for controls) -->
        <div class="flex items-center justify-between px-3 py-2 bg-slate-950/80 border-b border-white/5 backdrop-blur-sm absolute top-0 left-0 right-0 z-10 transition-opacity"
             :class="{ 'opacity-0 hover:opacity-100': isExpanded }">
            <div class="flex items-center space-x-2">
                <span class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
                <span class="text-white font-medium text-sm shadow-black drop-shadow-md">
                    {{ activeCall.type === 'video' ? 'Video Call' : 'Audio Call' }}
                </span>
            </div>
            <div class="flex items-center space-x-1">
                <button @click="toggleExpand" class="p-1.5 text-slate-300 hover:text-white rounded hover:bg-white/10 transition-colors">
                    <Maximize2 v-if="!isExpanded" class="w-4 h-4" />
                    <Minimize2 v-else class="w-4 h-4" />
                </button>
            </div>
        </div>

        <!-- Jitsi Meet Container -->
        <div class="flex-1 bg-slate-950 relative">
            <JitsiMeet 
                :roomName="roomName"
                :userInfo="userInfo"
                @readyToClose="handleHangup"
                :configOverwrite="{
                    startWithAudioMuted: false,
                    startWithVideoMuted: activeCall.type === 'audio',
                    disableDeepLinking: true,
                }"
                :interfaceConfigOverwrite="{
                    SHOW_JITSI_WATERMARK: false,
                    SHOW_WATERMARK_FOR_GUESTS: false,
                    DEFAULT_BACKGROUND: '#0f172a',
                    TOOLBAR_BUTTONS: [
                        'microphone', 'camera', 'closedcaptions', 'desktop', 'fullscreen',
                        'fodeviceselection', 'hangup', 'profile', 'chat', 'recording',
                        'livestreaming', 'etherpad', 'sharedvideo', 'settings', 'raisehand',
                        'videoquality', 'filmstrip', 'invite', 'feedback', 'stats', 'shortcuts',
                        'tileview', 'videobackgroundblur', 'download', 'help', 'mute-everyone',
                        'security'
                    ]
                }"
            />
        </div>
        
        <!-- Compact Mode Overlay (prevent interaction with Jitsi buttons if too small) -->
        <div v-if="!isExpanded" class="absolute inset-0 pointer-events-none border-t border-white/10"></div>
    </div>
</template>
