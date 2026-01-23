<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { X, User, Camera } from 'lucide-vue-next';
import { useAuthStore } from '../../stores/auth';
import { usersApi } from '../../api/users';
import BaseButton from '../atomic/BaseButton.vue';
import BaseInput from '../atomic/BaseInput.vue';

const props = defineProps<{
    show: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const authStore = useAuthStore();

const username = ref('');
const displayName = ref('');
const avatarUrl = ref('');
const loading = ref(false);
const error = ref('');
const success = ref('');

const currentUser = computed(() => authStore.user);

// Populate form when modal opens
watch(() => props.show, (isOpen) => {
    if (isOpen && currentUser.value) {
        username.value = currentUser.value.username || '';
        displayName.value = currentUser.value.display_name || '';
        avatarUrl.value = currentUser.value.avatar_url || '';
        error.value = '';
        success.value = '';
    }
});

async function handleSubmit() {
    if (!currentUser.value) return;

    loading.value = true;
    error.value = '';
    success.value = '';

    try {
        const response = await usersApi.update(currentUser.value.id, {
            username: username.value.trim() || undefined,
            display_name: displayName.value.trim() || undefined,
            avatar_url: avatarUrl.value.trim() || undefined,
        });
        
        // Update local user state
        authStore.user = response.data;
        success.value = 'Profile updated successfully!';
        
        setTimeout(() => {
            emit('close');
        }, 1000);
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to update profile';
    } finally {
        loading.value = false;
    }
}

function handleClose() {
    error.value = '';
    success.value = '';
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
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">Edit Profile</h2>
          <button @click="handleClose" class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
            <X class="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <!-- Form -->
        <form @submit.prevent="handleSubmit" class="p-6 space-y-5">
          <!-- Avatar Preview -->
          <div class="flex justify-center">
            <div class="relative">
              <div class="w-24 h-24 rounded-full bg-primary flex items-center justify-center text-white text-3xl font-bold overflow-hidden">
                <img v-if="avatarUrl" :src="avatarUrl" alt="Avatar" class="w-full h-full object-cover" />
                <User v-else class="w-12 h-12" />
              </div>
              <button 
                type="button"
                class="absolute bottom-0 right-0 w-8 h-8 bg-gray-800 dark:bg-gray-600 rounded-full flex items-center justify-center border-2 border-white dark:border-gray-800"
              >
                <Camera class="w-4 h-4 text-white" />
              </button>
            </div>
          </div>

          <!-- Error/Success Messages -->
          <div v-if="error" class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-600 dark:text-red-400 text-sm">
            {{ error }}
          </div>
          <div v-if="success" class="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-green-600 dark:text-green-400 text-sm">
            {{ success }}
          </div>

          <BaseInput
            v-model="username"
            label="Username"
            placeholder="your_username"
            :disabled="loading"
          />

          <BaseInput
            v-model="displayName"
            label="Display Name"
            placeholder="Your Name"
            :disabled="loading"
          />

          <BaseInput
            v-model="avatarUrl"
            label="Avatar URL"
            placeholder="https://example.com/avatar.jpg"
            :disabled="loading"
          />

          <!-- Email (read-only) -->
          <div class="space-y-1">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Email
            </label>
            <div class="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg text-gray-600 dark:text-gray-400 text-sm">
              {{ currentUser?.email }}
            </div>
          </div>

          <!-- Actions -->
          <div class="flex justify-end space-x-3 pt-4">
            <BaseButton variant="secondary" @click="handleClose" :disabled="loading">
              Cancel
            </BaseButton>
            <BaseButton type="submit" :loading="loading">
              Save Changes
            </BaseButton>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
