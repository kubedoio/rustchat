<script setup lang="ts">
import { computed, ref } from 'vue';
import { X } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import { useAuthStore } from '../../stores/auth';
import { canCreateTeam as canCreateTeamForRole } from '../../features/permissions/capabilities';
import BaseButton from '../atomic/BaseButton.vue';
import BaseInput from '../atomic/BaseInput.vue';

const props = defineProps<{
    show: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const teamStore = useTeamStore();
const authStore = useAuthStore();

const name = ref('');
const displayName = ref('');
const description = ref('');
const loading = ref(false);
const error = ref('');
const canCreateTeam = computed(() => canCreateTeamForRole(authStore.user?.role));

async function handleSubmit() {
    if (!canCreateTeam.value) {
        error.value = 'You do not have permission to create teams';
        return;
    }

    if (!name.value.trim()) {
        error.value = 'Team name is required';
        return;
    }

    loading.value = true;
    error.value = '';

    try {
        await teamStore.createTeam({
            name: name.value.trim().toLowerCase().replace(/\s+/g, '-'),
            display_name: displayName.value.trim() || name.value.trim(),
            description: description.value.trim() || undefined,
        });
        
        // Reset and close
        name.value = '';
        displayName.value = '';
        description.value = '';
        emit('close');
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to create team';
    } finally {
        loading.value = false;
    }
}

function handleClose() {
    name.value = '';
    displayName.value = '';
    description.value = '';
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
          <h2 class="text-xl font-bold text-gray-900">Create Team</h2>
          <button @click="handleClose" class="p-1 hover:bg-gray-100 rounded-lg transition-colors">
            <X class="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <div v-if="!canCreateTeam" class="p-6 space-y-4">
          <div class="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800">
            You do not have permission to create teams.
          </div>

          <div class="flex justify-end pt-2">
            <BaseButton variant="secondary" @click="handleClose">
              Close
            </BaseButton>
          </div>
        </div>

        <!-- Form -->
        <form v-else @submit.prevent="handleSubmit" class="p-6 space-y-4">
          <!-- Error -->
          <div v-if="error" class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
            {{ error }}
          </div>

          <BaseInput
            v-model="name"
            label="Team Name"
            placeholder="e.g., engineering"
            :disabled="loading"
            required
          />

          <BaseInput
            v-model="displayName"
            label="Display Name"
            placeholder="e.g., Engineering Team"
            :disabled="loading"
          />

          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">
              Description
            </label>
            <textarea
              v-model="description"
              rows="3"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-primary focus:border-transparent resize-none"
              placeholder="What's this team about?"
              :disabled="loading"
            ></textarea>
          </div>

          <!-- Actions -->
          <div class="flex justify-end space-x-3 pt-4">
            <BaseButton variant="secondary" @click="handleClose" :disabled="loading">
              Cancel
            </BaseButton>
            <BaseButton type="submit" :loading="loading">
              Create Team
            </BaseButton>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
