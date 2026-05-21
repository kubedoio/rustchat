<script setup lang="ts">
import { log } from '@/utils/log';
import { computed } from 'vue';
import { X, Mail, MessageCircle, Briefcase } from 'lucide-vue-next';
import RcAvatar from '../ui/RcAvatar.vue';
import BaseButton from '../atomic/BaseButton.vue';
import { useChannelStore } from '@/features/channels/stores/channelStore';
import { useRouter } from 'vue-router';
import client from '../../api/client';
import { getPresencePresentation } from '../../features/presence/presencePresentation';
import { useUserSummary } from '../../composables/useUserSummary';

const props = defineProps<{
  show: boolean;
  userId: string;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const router = useRouter();
const channelStore = useChannelStore();
const { userSummary: user, isLoading: loading, error } = useUserSummary(() => (props.show ? props.userId : null));

const fullName = computed(() => {
  if (!user.value) return '';
  const first = user.value.firstName || '';
  const last = user.value.lastName || '';
  if (first || last) return `${first} ${last}`.trim();
  return user.value.displayName || user.value.nickname || user.value.username;
});

const presenceMeta = computed(() => getPresencePresentation(user.value?.presence));

async function startDirectMessage() {
  if (!user.value) return;
  try {
    const { data: channel } = await client.post('/channels/direct', {
      user_ids: [user.value.id]
    });
    if (channel) {
      channelStore.selectChannel(channel.id);
      channelStore.addChannel(channel);
      emit('close');
      router.push('/');
    }
  } catch (e) {
    log.error('Failed to start DM', e);
  }
}

function handleClose() {
  emit('close');
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-bg-app/70 backdrop-blur-sm" @click="handleClose"></div>
      
      <!-- Modal -->
      <div class="relative mx-4 w-full max-w-sm overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl">
        <!-- Header -->
        <div class="flex items-center justify-end border-b border-border-1 px-4 py-3">
          <button @click="handleClose" class="rounded-r-2 p-1 transition-standard hover:bg-bg-surface-2">
            <X class="h-5 w-5 text-text-3" />
          </button>
        </div>

        <!-- Loading State -->
        <div v-if="loading" class="p-8 flex items-center justify-center">
          <div class="h-8 w-8 animate-spin rounded-full border-2 border-brand border-t-transparent"></div>
        </div>

        <!-- Error State -->
        <div v-else-if="error" class="p-8 text-center text-danger">
          {{ error }}
        </div>

        <!-- Profile Content -->
        <div v-else-if="user" class="pb-6">
          <!-- Avatar & Name Section -->
          <div class="flex flex-col items-center px-6">
            <RcAvatar 
              :userId="user.id"
              :username="user.username"
              :src="user.avatarUrl"
              :size="96"
              :showPresence="false"
              class="shadow-lg ring-4 ring-bg-surface-1"
            />
            
            <h2 class="mt-4 text-center text-xl font-semibold text-brand">
              {{ fullName }}
            </h2>
            
            <p v-if="user.nickname && user.nickname !== fullName" class="text-sm text-text-3">
              {{ user.nickname }}
            </p>
            
            <!-- Presence Badge -->
            <div data-testid="profile-modal-status" class="mt-3 flex flex-wrap items-center justify-center gap-2">
              <span data-testid="profile-modal-presence" class="inline-flex items-center gap-1 rounded-full border px-3 py-1 text-sm font-medium" :class="presenceMeta.badgeClass">
                <component :is="presenceMeta.icon" class="h-3.5 w-3.5" />
                {{ presenceMeta.label }}
              </span>
              <span v-if="user.statusText" data-testid="profile-modal-custom-status" class="inline-flex max-w-full items-center gap-1 rounded-full border border-border-1 bg-bg-surface-2 px-3 py-1 text-sm text-text-2">
                <span v-if="user.statusEmoji">{{ user.statusEmoji }}</span>
                <span class="truncate">{{ user.statusText }}</span>
              </span>
            </div>
          </div>

          <!-- Details Section -->
          <div class="mt-6 space-y-4 px-6">
            <!-- Username -->
            <div class="flex items-center space-x-3 text-sm text-text-2">
              <span class="text-text-4">@</span>
              <span>{{ user.username }}</span>
            </div>

            <!-- Email -->
            <div class="flex items-center space-x-3 text-sm text-text-2">
              <Mail class="h-4 w-4 text-text-4" />
              <span>{{ user.email }}</span>
            </div>

            <!-- Nickname -->
            <div v-if="user.nickname" class="flex items-center space-x-3 text-sm text-text-2">
              <span class="w-16 text-xs uppercase tracking-wider text-text-4">Nickname</span>
              <span>{{ user.nickname }}</span>
            </div>

            <!-- First & Last Name -->
            <div v-if="user.firstName || user.lastName" class="flex items-center space-x-3 text-sm text-text-2">
              <span class="w-16 text-xs uppercase tracking-wider text-text-4">Full Name</span>
              <span>{{ user.firstName }} {{ user.lastName }}</span>
            </div>

            <!-- Position -->
            <div v-if="user.position" class="flex items-center space-x-3 text-sm text-text-2">
              <Briefcase class="h-4 w-4 text-text-4" />
              <span>{{ user.position }}</span>
            </div>
          </div>

          <!-- Message Button -->
          <div class="mt-6 px-6">
            <BaseButton class="w-full" @click="startDirectMessage">
              <MessageCircle class="w-4 h-4 mr-2" />
              Message
            </BaseButton>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
