<script setup lang="ts">
import { useCallsStore } from '../../stores/calls'
import { computed } from 'vue'
import { Phone, PhoneOff } from 'lucide-vue-next'

const callsStore = useCallsStore()

const incomingCall = computed(() => callsStore.incomingCall)

const accept = async () => {
    if (incomingCall.value) {
        await callsStore.joinCall(incomingCall.value.channelId) // Using channelId as callId for now or fetching relevant call
        // In real flow, we'd accept via a specific call ID payload
        callsStore.setIncomingCall(null)
    }
}

const decline = () => {
    callsStore.setIncomingCall(null)
}
</script>

<template>
    <div v-if="incomingCall" class="fixed top-4 right-4 z-50 animate-slide-in">
        <div class="bg-bg-surface-1 rounded-lg shadow-xl border border-border-1 p-4 w-72">
            <div class="flex items-center space-x-3 mb-4">
                <div class="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center">
                    <Phone class="w-5 h-5 text-primary animate-pulse" />
                </div>
                <div>
                    <h3 class="font-medium text-text-1">Incoming Call</h3>
                    <p class="text-sm text-text-3">Channel Call</p>
                </div>
            </div>
            
            <div class="flex space-x-3">
                <button @click="decline" class="flex-1 flex items-center justify-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-danger hover:bg-danger/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-danger">
                    <PhoneOff class="w-4 h-4 mr-2" />
                    Decline
                </button>
                <button @click="accept" class="flex-1 flex items-center justify-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-success hover:bg-success/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-success">
                    <Phone class="w-4 h-4 mr-2" />
                    Accept
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.animate-slide-in {
    animation: slideIn 0.3s ease-out;
}
@keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
}
</style>
