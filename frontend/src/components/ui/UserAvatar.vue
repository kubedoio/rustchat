<script setup lang="ts">
import { computed } from 'vue';
import md5 from 'md5';

interface Props {
  src?: string;
  email?: string; // For Gravatar
  username?: string; // For GitHub fallback & Initials
  alt?: string;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  presence?: 'online' | 'away' | 'dnd' | 'offline';
}

const props = defineProps<Props>();

const gravatarUrl = computed(() => {
  if (!props.email) return null;
  const hash = md5(props.email.trim().toLowerCase());
  return `https://www.gravatar.com/avatar/${hash}?d=404`; 
});

const githubAvatarUrl = computed(() => {
  if (!props.username) return null;
  return `https://github.com/${props.username}.png`;
});

const initials = computed(() => {
  if (!props.username) return '?';
  return props.username.slice(0, 2).toUpperCase();
});

const computedSize = computed(() => {
    switch(props.size) {
        case 'sm': return 'w-6 h-6 text-xs';
        case 'md': return 'w-8 h-8 text-sm';
        case 'lg': return 'w-10 h-10 text-base';
        case 'xl': return 'w-20 h-20 text-2xl';
        default: return 'w-8 h-8 text-sm'; // Default md
    }
});

// Handling image load priority is complex in pure CSS/Vue
// We'll use a layered approach or simply try primary src, then gravatar, then github
// A better simple approach is to rely on the `src` being passed correctly from the parent if they have an upload
// And if that is empty, we fall back here.
// But to fail gracefully (e.g. 404 on Gravatar), we might need JS event handling.
// For MVP, we can iterate:
// If props.src exists, show it.
// Else if props.email exists, show Gravatar.
// Else if props.username exists, show GitHub.
// Else show initials.
// Note: This naive logic doesn't handle 404s (e.g. user has email but no Gravatar).
// To robustness, we can update to use event listeners or a composable.
// Let's implement a simple `onError` handler.

import { ref, watch } from 'vue';

const currentSrc = ref<string | null>(null);
const triedGravatar = ref(false);
const triedGithub = ref(false);
const showInitials = ref(false);

function resetState() {
    triedGravatar.value = false;
    triedGithub.value = false;
    showInitials.value = false;
    determineSrc();
}

watch(() => [props.src, props.email, props.username], () => {
    resetState();
}, { immediate: true });

function determineSrc() {
    if (props.src) {
        currentSrc.value = props.src;
        return;
    }
    tryNext();
}

function tryNext() {
    if (!triedGravatar.value && props.email) {
        triedGravatar.value = true;
        currentSrc.value = gravatarUrl.value;
    } else if (!triedGithub.value && props.username) {
        triedGithub.value = true;
        currentSrc.value = githubAvatarUrl.value;
    } else {
        showInitials.value = true;
        currentSrc.value = null;
    }
}

function handleError() {
    // If current source failed to load, try next strategy
    tryNext();
}

</script>

<template>
  <div 
    class="relative inline-flex items-center justify-center overflow-hidden rounded-full bg-slate-100 text-slate-500 font-medium"
    :class="computedSize"
  >
    <img 
        v-if="!showInitials && currentSrc" 
        :src="currentSrc" 
        :alt="alt || username" 
        class="w-full h-full object-cover"
        @error="handleError"
    />
    <span v-else>{{ initials }}</span>

    <!-- Presence Badge -->
    <div 
      v-if="presence && presence !== 'offline' && size !== 'sm'"
      class="absolute bottom-0 right-0 rounded-full border-2 border-white dark:border-gray-800"
      :class="[
        size === 'xl' ? 'w-5 h-5 border-[3px]' : 'w-3 h-3',
        presence === 'online' ? 'bg-green-500' : 
        presence === 'away' ? 'bg-amber-500' : 
        presence === 'dnd' ? 'bg-red-500' : 'bg-gray-400'
      ]"
    ></div>
  </div>
</template>
