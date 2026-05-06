<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue';

const props = defineProps<{
    domain?: string;
    roomName: string;
    width?: string | number;
    height?: string | number;
    configOverwrite?: object;
    interfaceConfigOverwrite?: object;
    userInfo?: {
        displayName?: string;
        email?: string;
        avatarUrl?: string;
    };
    jwt?: string;
}>();

const emit = defineEmits<{
    (e: 'videoConferenceJoined', payload: any): void;
    (e: 'videoConferenceLeft', payload: any): void;
    (e: 'readyToClose'): void;
    (e: 'participantJoined', payload: any): void;
    (e: 'participantLeft', payload: any): void;
    (e: 'error', payload: any): void;
}>();

const jitsiContainer = ref<HTMLElement | null>(null);
let api: any = null;

onMounted(() => {
    loadJitsiScript();
});

onUnmounted(() => {
    disposeApi();
});

function loadJitsiScript() {
    if (window.JitsiMeetExternalAPI) {
        initJitsi();
        return;
    }
    
    // Fallback if script not in index.html (though we plan to add it there)
    const script = document.createElement('script');
    script.src = `https://${props.domain || 'meet.jit.si'}/external_api.js`;
    script.async = true;
    script.onload = () => initJitsi();
    script.onerror = (err) => emit('error', err);
    document.body.appendChild(script);
}

function initJitsi() {
    if (!jitsiContainer.value) return;

    const options = {
        roomName: props.roomName,
        width: props.width || '100%',
        height: props.height || '100%',
        parentNode: jitsiContainer.value,
        configOverwrite: {
            startWithAudioMuted: false,
            startWithVideoMuted: false,
            prejoinPageEnabled: false,
            ...props.configOverwrite
        },
        interfaceConfigOverwrite: {
            TOOLBAR_BUTTONS: [
                'microphone', 'camera', 'closedcaptions', 'desktop', 'fullscreen',
                'fodeviceselection', 'hangup', 'profile', 'chat', 'recording',
                'livestreaming', 'etherpad', 'sharedvideo', 'settings', 'raisehand',
                'videoquality', 'filmstrip', 'invite', 'feedback', 'stats', 'shortcuts',
                'tileview', 'videobackgroundblur', 'download', 'help', 'mute-everyone',
                'security'
            ],
            ...props.interfaceConfigOverwrite
        },
        userInfo: props.userInfo,
        jwt: props.jwt
    };

    // @ts-ignore
    api = new window.JitsiMeetExternalAPI(props.domain || 'meet.jit.si', options);

    api.addEventListeners({
        videoConferenceJoined: (e: any) => emit('videoConferenceJoined', e),
        videoConferenceLeft: (e: any) => emit('videoConferenceLeft', e),
        readyToClose: () => emit('readyToClose'),
        participantJoined: (e: any) => emit('participantJoined', e),
        participantLeft: (e: any) => emit('participantLeft', e),
    });
}

function disposeApi() {
    if (api) {
        api.dispose();
        api = null;
    }
}

// Watch for room changes to re-init if needed
watch(() => props.roomName, () => {
    disposeApi();
    initJitsi();
});
</script>

<template>
    <div ref="jitsiContainer" class="w-full h-full bg-slate-900"></div>
</template>
