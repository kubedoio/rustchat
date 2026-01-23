<script setup lang="ts">
import { ref } from 'vue';
import { X } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import BaseButton from '../atomic/BaseButton.vue';
import BaseInput from '../atomic/BaseInput.vue';

const props = defineProps<{
    show: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const teamStore = useTeamStore();

const name = ref('');
const displayName = ref('');
const description = ref('');
const loading = ref(false);
const error = ref('');

async function handleSubmit() {
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
      <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">Create Team</h2>
          <button @click="handleClose" class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
            <X class="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <!-- Form -->
        <form @submit.prevent="handleSubmit" class="p-6 space-y-4">
          <!-- Error -->
          <div v-if="error" class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-600 dark:text-red-400 text-sm">
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
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Description
            </label>
            <textarea
              v-model="description"
              rows="3"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary focus:border-transparent resize-none"
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
