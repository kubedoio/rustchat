<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { X, Search, User } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import { useChannelStore } from '../../stores/channels';
import { useAuthStore } from '../../stores/auth';
import BaseButton from '../atomic/BaseButton.vue';

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
      <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" @click="handleClose"></div>
      
      <!-- Modal -->
      <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-md mx-4 overflow-hidden flex flex-col max-h-[80vh]">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">Direct Messages</h2>
          <button @click="handleClose" class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
            <X class="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <!-- Search -->
        <div class="px-6 py-4 border-b border-gray-100 dark:border-gray-700">
          <div class="relative">
            <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
            <input 
              v-model="search"
              type="text"
              placeholder="Search for a team member..."
              class="w-full pl-10 pr-4 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent outline-none transition-all"
              autofocus
            />
          </div>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-2 custom-scrollbar min-h-[200px]">
          <!-- Error -->
          <div v-if="error" class="m-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-600 dark:text-red-400 text-sm">
            {{ error }}
          </div>

          <!-- Loading -->
          <div v-if="teamStore.loading && teamStore.members.length === 0" class="flex flex-col items-center justify-center py-12 text-gray-500">
            <div class="w-8 h-8 border-4 border-indigo-500/20 border-t-indigo-500 rounded-full animate-spin mb-4"></div>
            <p class="text-sm">Loading members...</p>
          </div>

          <!-- Members List -->
          <div v-else-if="filteredMembers.length > 0" class="space-y-1">
            <button
              v-for="member in filteredMembers"
              :key="member.user_id"
              @click="startDM(member)"
              :disabled="loading"
              class="w-full flex items-center px-4 py-3 rounded-lg hover:bg-indigo-50 dark:hover:bg-indigo-900/30 transition-colors group text-left"
            >
              <!-- Avatar -->
              <div class="relative mr-4">
                <img 
                  v-if="member.avatar_url"
                  :src="member.avatar_url"
                  class="w-10 h-10 rounded-full object-cover ring-2 ring-white dark:ring-gray-800 shadow-sm"
                />
                <div v-else class="w-10 h-10 rounded-full bg-indigo-100 dark:bg-indigo-900/50 flex items-center justify-center ring-2 ring-white dark:ring-gray-800 shadow-sm">
                  <User class="w-6 h-6 text-indigo-500 dark:text-indigo-400" />
                </div>
              </div>

              <!-- Name -->
              <div class="flex-1 min-w-0">
                <p class="text-sm font-semibold text-gray-900 dark:text-white truncate">
                  {{ member.display_name || member.username }}
                </p>
                <p class="text-xs text-gray-500 dark:text-gray-400 truncate">
                  @{{ member.username }}
                </p>
              </div>
            </button>
          </div>

          <!-- Empty State -->
          <div v-else class="flex flex-col items-center justify-center py-12 text-gray-500 px-6 text-center">
            <div class="w-12 h-12 rounded-full bg-gray-100 dark:bg-gray-800 flex items-center justify-center mb-4">
              <Search class="w-6 h-6 text-gray-400" />
            </div>
            <p class="text-sm font-medium">No team members found</p>
            <p class="text-xs mt-1">Try a different search or invite someone to the team!</p>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 flex justify-end">
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
  background: #cbd5e1;
  border-radius: 4px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: #475569;
}
</style>
