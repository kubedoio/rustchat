<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { usePresenceStore } from '../../features/presence';
import { getPresencePresentation, normalizePresenceStatus } from '../../features/presence/presencePresentation';
import { useTeamStore } from '../../stores/teams';
import type { PresenceStatus } from '../../core/entities/User';

interface Props {
  userId?: string;
  username?: string;
  src?: string;
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | number;
  showPresence?: boolean;
  avatarVersion?: number | string;
}

const props = withDefaults(defineProps<Props>(), {
  showPresence: true,
  size: 'md'
});

const presenceStore = usePresenceStore();
const teamStore = useTeamStore();

const userPresence = computed(() => {
  if (!props.userId) return null;
  return presenceStore.getUserPresence(props.userId).value;
});

const fallbackTeamPresence = computed<PresenceStatus | null>(() => {
  if (!props.userId) return null;
  const member = teamStore.members.find((m) => m.user_id === props.userId);
  return (member?.presence?.toLowerCase() as PresenceStatus) || null;
});

const currentPresence = computed<PresenceStatus>(() => {
  return normalizePresenceStatus(userPresence.value?.presence || fallbackTeamPresence.value);
});

const initials = computed(() => {
  const name = props.username;
  if (!name) return '?';
  const parts = name.split(/[\s._-]/).filter(p => p.length > 0);
  if (parts.length >= 2 && parts[0]?.[0] && parts[1]?.[0]) {
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }
  return name.slice(0, 2).toUpperCase() || '?';
});

const bgColor = computed(() => {
  const name = props.username;
  if (!name) return 'bg-slate-400';
  const colors = [
    'bg-blue-500', 'bg-green-500', 'bg-red-500', 'bg-yellow-500', 
    'bg-purple-500', 'bg-pink-500', 'bg-indigo-500', 'bg-teal-500',
    'bg-orange-500', 'bg-cyan-500'
  ];
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return colors[Math.abs(hash) % colors.length];
});

const sizeClasses = computed(() => {
  const size = props.size;
  if (typeof size === 'number') return '';
  switch (size) {
    case 'xs': return 'w-5 h-5 text-[10px]';
    case 'sm': return 'w-6 h-6 text-xs';
    case 'md': return 'w-9 h-9 text-sm';
    case 'lg': return 'w-12 h-12 text-lg';
    case 'xl': return 'w-24 h-24 text-3xl';
    default: return 'w-9 h-9 text-sm';
  }
});

const customSizeStyle = computed(() => {
  const size = props.size;
  if (typeof size === 'number') {
    return {
      width: `${size}px`,
      height: `${size}px`,
      fontSize: `${Math.max(size / 2.5, 10)}px`
    };
  }
  return {};
});

const avatarUrl = computed(() => {
  if (!props.src) return null;
  if (!props.avatarVersion) return props.src;
  const separator = props.src.includes('?') ? '&' : '?';
  return `${props.src}${separator}v=${props.avatarVersion}`;
});

// Track image loading errors
const imageError = ref(false);

// Reset error when src changes
watch(() => props.src, () => {
  imageError.value = false;
});

function handleImageError() {
  imageError.value = true;
}

const presenceSizeClass = computed(() => {
  const size = props.size;
  if (typeof size === 'number') {
    return size > 40 ? 'w-4 h-4 border-2' : 'w-3 h-3 border-2';
  }
  switch (size) {
    case 'xs': return 'w-1.5 h-1.5 border';
    case 'sm': return 'w-3 h-3 border-2';
    case 'md': return 'w-3.5 h-3.5 border-2';
    case 'lg': return 'w-4 h-4 border-2';
    case 'xl': return 'w-7 h-7 border-4';
    default: return 'w-3.5 h-3.5 border-2';
  }
});

const presenceMeta = computed(() => getPresencePresentation(currentPresence.value));

const showPresenceIcon = computed(() => {
  const size = props.size;
  if (typeof size === 'number') {
    return size >= 24;
  }
  return size !== 'xs';
});

const presenceIconClass = computed(() => {
  const size = props.size;
  if (typeof size === 'number') {
    return size >= 40 ? 'w-2 h-2' : 'w-1.5 h-1.5';
  }
  switch (size) {
    case 'sm': return 'w-1.5 h-1.5';
    case 'md': return 'w-2 h-2';
    case 'lg': return 'w-2 h-2';
    case 'xl': return 'w-3.5 h-3.5';
    default: return 'w-2 h-2';
  }
});
</script>

<template>
  <div 
    class="relative inline-flex items-center justify-center rounded-full shrink-0 select-none overflow-visible"
    :class="[sizeClasses, !avatarUrl || imageError ? bgColor : '']"
    :style="customSizeStyle"
  >
    <!-- Avatar Image -->
    <div class="w-full h-full rounded-full overflow-hidden flex items-center justify-center">
      <img 
        v-if="avatarUrl && !imageError" 
        :src="avatarUrl" 
        :alt="username" 
        class="w-full h-full object-cover"
        crossorigin="anonymous"
        @error="handleImageError"
      />
      <!-- Fallback Initials -->
      <span v-else class="text-white font-bold tracking-tighter">
        {{ initials }}
      </span>
    </div>

    <!-- Presence Dot -->
    <div 
      v-if="showPresence"
      class="absolute bottom-0 right-0 flex items-center justify-center rounded-full shadow-sm"
      :class="[presenceSizeClass, presenceMeta.badgeClass]"
      :title="presenceMeta.label"
    >
      <component
        :is="presenceMeta.icon"
        v-if="presenceMeta.icon && showPresenceIcon"
        :class="presenceIconClass"
        stroke-width="2.5"
      />
    </div>
  </div>
</template>
