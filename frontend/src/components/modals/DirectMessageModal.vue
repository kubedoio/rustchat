<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { X, Search } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import { useChannelStore } from '../../stores/channels';
import { useAuthStore } from '../../stores/auth';
import BaseButton from '../atomic/BaseButton.vue';
import RcAvatar from '../ui/RcAvatar.vue';

const props = defineProps<{
    show: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const teamStore = useTeamStore();
const channelStore = useChannelStore();
const authStore = useAuthStore();

const search = ref('');
const loading = ref(false);
const error = ref('');

const currentTeamId = computed(() => teamStore.currentTeamId);

// Fetch members when modal opens
watch(() => props.show, (isShown) => {
    if (isShown && currentTeamId.value) {
        teamStore.fetchMembers(currentTeamId.value);
    }
});

import { type TeamMember } from '../../api/teams';

const filteredMembers = computed((): TeamMember[] => {
    if (!teamStore.members) return [];
    
    return (teamStore.members as TeamMember[]).filter((m: TeamMember) => {
        // Don't show current user
        if (m.user_id === authStore.user?.id) return false;
        
        const searchLower = search.value.toLowerCase();
        return (
            m.username.toLowerCase().includes(searchLower) ||
            (m.display_name && m.display_name.toLowerCase().includes(searchLower))
        );
    });
});

async function startDM(member: any) {
    if (!currentTeamId.value) return;

    loading.value = true;
    error.value = '';

    try {
        const channel = await channelStore.createChannel({
            team_id: currentTeamId.value,
            name: `dm_${member.user_id}`, // Backend will handle deterministic naming
            display_name: member.display_name || member.username,
            channel_type: 'direct',
            target_user_id: member.user_id,
        });
        
        // Select the new/existing channel
        channelStore.selectChannel(channel.id);
        handleClose();
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to start direct message';
    } finally {
        loading.value = false;
    }
}

function handleClose() {
    search.value = '';
    error.value = '';
    emit('close');
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-bg-app/70 backdrop-blur-sm" @click="handleClose"></div>
      
      <!-- Modal -->
      <div class="relative mx-4 flex max-h-[80vh] w-full max-w-md flex-col overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl">
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-border-1 px-6 py-4">
          <div>
            <h2 class="text-xl font-semibold text-brand">Direct Messages</h2>
            <p class="text-xs text-text-3">Start a private conversation in the current workspace.</p>
          </div>
          <button @click="handleClose" class="rounded-r-2 p-1 transition-standard hover:bg-bg-surface-2">
            <X class="h-5 w-5 text-text-3" />
          </button>
        </div>

        <!-- Search -->
        <div class="border-b border-border-1 px-6 py-4 bg-bg-surface-2/45">
          <div class="relative">
            <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-3" />
            <input 
              v-model="search"
              type="text"
              placeholder="Search for a team member..."
              class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 py-2 pl-10 pr-4 text-sm text-text-1 outline-none transition-standard focus:border-brand/50 focus:ring-2 focus:ring-brand/20"
              autofocus
            />
          </div>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-2 custom-scrollbar min-h-[200px]">
          <!-- Error -->
          <div v-if="error" class="m-4 rounded-r-2 border border-danger/20 bg-danger/5 p-3 text-sm text-danger">
            {{ error }}
          </div>

          <!-- Loading -->
          <div v-if="teamStore.loading && teamStore.members.length === 0" class="flex flex-col items-center justify-center py-12 text-text-3">
            <div class="mb-4 h-8 w-8 animate-spin rounded-full border-4 border-brand/20 border-t-brand"></div>
            <p class="text-sm">Loading members...</p>
          </div>

          <!-- Members List -->
          <div v-else-if="filteredMembers.length > 0" class="space-y-1">
            <button
              v-for="member in filteredMembers"
              :key="member.user_id"
              @click="startDM(member)"
              :disabled="loading"
              class="group flex w-full items-center rounded-r-2 border border-transparent px-4 py-3 text-left transition-standard hover:border-border-1 hover:bg-bg-surface-2/70"
            >
              <!-- Avatar -->
              <RcAvatar 
                :userId="member.user_id"
                :username="member.username"
                :src="member.avatar_url"
                size="md"
                class="mr-4"
              />

              <!-- Name -->
              <div class="flex-1 min-w-0">
                <p class="truncate text-sm font-semibold text-text-1 transition-standard group-hover:text-brand">
                  {{ member.display_name || member.username }}
                </p>
                <p class="truncate text-xs text-text-3">
                  @{{ member.username }}
                </p>
              </div>
            </button>
          </div>

          <!-- Empty State -->
          <div v-else class="flex flex-col items-center justify-center px-6 py-12 text-center text-text-3">
            <div class="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-bg-surface-2">
              <Search class="h-6 w-6 text-text-4" />
            </div>
            <p class="text-sm font-medium">No team members found</p>
            <p class="text-xs mt-1">Try a different search or invite someone to the team!</p>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex justify-end border-t border-border-1 bg-bg-surface-2/45 px-6 py-4">
          <BaseButton variant="secondary" @click="handleClose" :disabled="loading">
            Cancel
          </BaseButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--color-border-1);
  border-radius: 4px;
}
</style>
