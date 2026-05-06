<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { X, User, Camera } from 'lucide-vue-next';
import { useAuthStore } from '../../stores/auth';
import type { AuthUser } from '../../core/entities/Auth';
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
const firstName = ref('');
const lastName = ref('');
const displayName = ref('');
const nickname = ref('');
const position = ref('');
const avatarUrl = ref('');
const loading = ref(false);
const error = ref('');
const success = ref('');

const currentUser = computed(() => authStore.user);

// Full name preview
const fullNamePreview = computed(() => {
  const first = firstName.value.trim();
  const last = lastName.value.trim();
  if (first || last) return `${first} ${last}`.trim();
  return displayName.value.trim() || nickname.value.trim() || username.value;
});

// Populate form when modal opens
watch(() => props.show, (isOpen) => {
    if (isOpen && currentUser.value) {
        username.value = currentUser.value.username || '';
        firstName.value = currentUser.value.firstName || '';
        lastName.value = currentUser.value.lastName || '';
        displayName.value = currentUser.value.displayName || '';
        nickname.value = currentUser.value.nickname || '';
        position.value = currentUser.value.position || '';
        avatarUrl.value = currentUser.value.avatarUrl || '';
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
            first_name: firstName.value.trim() || undefined,
            last_name: lastName.value.trim() || undefined,
            display_name: displayName.value.trim() || undefined,
            nickname: nickname.value.trim() || undefined,
            position: position.value.trim() || undefined,
            avatar_url: avatarUrl.value.trim() || undefined,
        });
        
        // Update local user state
        authStore.user = response.data as unknown as AuthUser;
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
      <div class="relative bg-white rounded-xl shadow-2xl w-full max-w-md mx-4 overflow-hidden max-h-[90vh] overflow-y-auto">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <h2 class="text-xl font-bold text-gray-900">Edit Profile</h2>
          <button @click="handleClose" class="p-1 hover:bg-gray-100 rounded-lg transition-colors">
            <X class="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <!-- Form -->
        <form @submit.prevent="handleSubmit" class="p-6 space-y-5">
          <!-- Avatar Preview -->
          <div class="flex justify-center">
            <div class="relative">
              <div class="flex h-24 w-24 items-center justify-center overflow-hidden rounded-full bg-primary text-3xl font-bold text-brand-foreground">
                <img v-if="avatarUrl" :src="avatarUrl" alt="Avatar" class="w-full h-full object-cover" />
                <User v-else class="w-12 h-12" />
              </div>
              <button 
                type="button"
                class="absolute bottom-0 right-0 w-8 h-8 bg-gray-800 rounded-full flex items-center justify-center border-2 border-white"
              >
                <Camera class="w-4 h-4 text-white" />
              </button>
            </div>
          </div>

          <!-- Full Name Preview -->
          <div class="text-center">
            <p class="text-lg font-semibold text-gray-900">{{ fullNamePreview }}</p>
            <p class="text-sm text-gray-500">@{{ username }}</p>
          </div>

          <!-- Error/Success Messages -->
          <div v-if="error" class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
            {{ error }}
          </div>
          <div v-if="success" class="p-3 bg-green-50 border border-green-200 rounded-lg text-green-600 text-sm">
            {{ success }}
          </div>

          <!-- Name Fields -->
          <div class="space-y-1">
            <label class="block text-xs font-semibold text-gray-500 uppercase tracking-wider">Full Name</label>
            <div class="grid grid-cols-2 gap-3">
              <BaseInput
                v-model="firstName"
                label="First Name"
                placeholder="John"
                :disabled="loading"
              />
              <BaseInput
                v-model="lastName"
                label="Last Name"
                placeholder="Doe"
                :disabled="loading"
              />
            </div>
          </div>

          <BaseInput
            v-model="displayName"
            label="Display Name"
            placeholder="How you want to be called"
            :disabled="loading"
          />

          <BaseInput
            v-model="nickname"
            label="Nickname"
            placeholder="Your preferred name"
            :disabled="loading"
          />

          <BaseInput
            v-model="position"
            label="Position / Job Title"
            placeholder="e.g. Software Engineer"
            :disabled="loading"
          />

          <BaseInput
            v-model="username"
            label="Username"
            placeholder="your_username"
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
            <label class="block text-sm font-medium text-gray-700">
              Email
            </label>
            <div class="px-3 py-2 bg-gray-100 rounded-lg text-gray-600 text-sm">
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
