<script setup lang="ts">
import { ref, computed } from 'vue';
import { X } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import { useChannelStore } from '../../stores/channels';
import { useAuthStore } from '../../stores/auth';
import { canCreateChannel as canCreateChannelForRole } from '../../features/permissions/capabilities';
import BaseButton from '../atomic/BaseButton.vue';
import BaseInput from '../atomic/BaseInput.vue';

const props = defineProps<{
    show: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const teamStore = useTeamStore();
const channelStore = useChannelStore();
const authStore = useAuthStore();

const name = ref('');
const displayName = ref('');
const channelType = ref<'public' | 'private'>('public');
const purpose = ref('');
const loading = ref(false);
const error = ref('');

const currentTeam = computed(() => teamStore.currentTeam);
const canCreateStandardChannel = computed(() => canCreateChannelForRole(authStore.user?.role));

async function handleSubmit() {
    if (!canCreateStandardChannel.value) {
        error.value = 'You do not have permission to create channels';
        return;
    }

    if (!name.value.trim()) {
        error.value = 'Channel name is required';
        return;
    }

    if (!currentTeam.value) {
        error.value = 'Please select a team first';
        return;
    }

    loading.value = true;
    error.value = '';

    try {
        await channelStore.createChannel({
            team_id: currentTeam.value.id,
            name: name.value.trim().toLowerCase().replace(/\s+/g, '-'),
            display_name: displayName.value.trim() || name.value.trim(),
            channel_type: channelType.value,
            purpose: purpose.value.trim() || undefined,
        });
        
        // Reset and close
        name.value = '';
        displayName.value = '';
        channelType.value = 'public';
        purpose.value = '';
        emit('close');
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to create channel';
    } finally {
        loading.value = false;
    }
}

function handleClose() {
    name.value = '';
    displayName.value = '';
    channelType.value = 'public';
    purpose.value = '';
    error.value = '';
    emit('close');
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/50" @click="handleClose"></div>
      
      <!-- Modal -->
      <div class="relative bg-white rounded-xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <h2 class="text-xl font-bold text-gray-900">Create Channel</h2>
          <button @click="handleClose" class="p-1 hover:bg-gray-100 rounded-lg transition-colors">
            <X class="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <div v-if="!canCreateStandardChannel" class="p-6 space-y-4">
          <div class="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800">
            You do not have permission to create channels.
          </div>

          <div class="flex justify-end pt-2">
            <BaseButton variant="secondary" @click="handleClose">
              Close
            </BaseButton>
          </div>
        </div>

        <!-- Form -->
        <form v-else @submit.prevent="handleSubmit" class="p-6 space-y-4">
          <!-- No Team Warning -->
          <div v-if="!currentTeam" class="p-3 bg-yellow-50 border border-yellow-200 rounded-lg text-yellow-600 text-sm">
            Please create or select a team first.
          </div>

          <!-- Error -->
          <div v-if="error" class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
            {{ error }}
          </div>

          <!-- Channel Type -->
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Channel Type
            </label>
            <div class="flex space-x-4">
              <label class="flex items-center cursor-pointer">
                <input 
                  type="radio" 
                  v-model="channelType" 
                  value="public"
                  class="w-4 h-4 text-primary border-gray-300 focus:ring-primary"
                />
                <span class="ml-2 text-sm text-gray-700">
                  <span class="font-medium">Public</span> - Anyone can join
                </span>
              </label>
              <label class="flex items-center cursor-pointer">
                <input 
                  type="radio" 
                  v-model="channelType" 
                  value="private"
                  class="w-4 h-4 text-primary border-gray-300 focus:ring-primary"
                />
                <span class="ml-2 text-sm text-gray-700">
                  <span class="font-medium">Private</span> - Invite only
                </span>
              </label>
            </div>
          </div>

          <BaseInput
            v-model="name"
            label="Channel Name"
            placeholder="e.g., general"
            :disabled="loading || !currentTeam"
            required
          />

          <BaseInput
            v-model="displayName"
            label="Display Name"
            placeholder="e.g., General Discussion"
            :disabled="loading || !currentTeam"
          />

          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">
              Purpose
            </label>
            <textarea
              v-model="purpose"
              rows="2"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-primary focus:border-transparent resize-none"
              placeholder="What's this channel about?"
              :disabled="loading || !currentTeam"
            ></textarea>
          </div>

          <!-- Actions -->
          <div class="flex justify-end space-x-3 pt-4">
            <BaseButton variant="secondary" @click="handleClose" :disabled="loading">
              Cancel
            </BaseButton>
            <BaseButton type="submit" :loading="loading" :disabled="!currentTeam">
              Create Channel
            </BaseButton>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
