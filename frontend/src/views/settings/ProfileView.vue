<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAuthStore } from '../../stores/auth';
import { Camera, Save, Trash2, ArrowLeft } from 'lucide-vue-next';
import RcAvatar from '../../components/ui/RcAvatar.vue';
import ThemeToggle from '../../components/ui/ThemeToggle.vue';
import api from '../../api/client';

const authStore = useAuthStore();
const user = computed(() => authStore.user);

const displayName = ref(user.value?.display_name || '');
const username = ref(user.value?.username || '');
const saving = ref(false);
const uploading = ref(false);
const error = ref<string | null>(null);
const success = ref(false);

const fileInput = ref<HTMLInputElement | null>(null);

async function handleUpdateProfile() {
    if (!user.value) return;
    saving.value = true;
    error.value = null;
    success.value = false;
    try {
        await api.put(`/users/${user.value.id}`, {
            display_name: displayName.value,
            username: username.value
        });
        // Update local state is handled via WebSocket or manual refresh
        // For now, let's just refresh the user in store if needed
        await authStore.fetchMe();
        success.value = true;
        setTimeout(() => success.value = false, 3000);
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to update profile';
    } finally {
        saving.value = false;
    }
}

async function handleAvatarUpload(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file || !user.value) return;

    uploading.value = true;
    error.value = null;
    
    const formData = new FormData();
    formData.append('file', file);

    try {
        // 1. Upload file
        const uploadRes = await api.post('/files', formData);
        const avatarUrl = uploadRes.data.url;

        // 2. Update user with new avatar URL
        await api.put(`/users/${user.value.id}`, { avatar_url: avatarUrl });
        
        // 3. Refresh user
        await authStore.fetchMe();
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to upload avatar';
    } finally {
        uploading.value = false;
        if (fileInput.value) fileInput.value.value = '';
    }
}

async function removeAvatar() {
    if (!user.value) return;
    try {
        await api.put(`/users/${user.value.id}`, { avatar_url: '' });
        await authStore.fetchMe();
    } catch (e) {
        console.error('Failed to remove avatar', e);
    }
}
</script>

<template>
    <div class="min-h-screen bg-gray-50 dark:bg-gray-950 p-6">
        <div class="max-w-2xl mx-auto space-y-8">
            <!-- Header -->
            <div class="flex items-center justify-between">
                <div class="flex items-center space-x-4">
                    <router-link to="/" class="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full text-gray-500 transition-colors">
                        <ArrowLeft class="w-6 h-6" />
                    </router-link>
                    <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Profile Settings</h1>
                </div>
            </div>

            <div class="bg-white dark:bg-gray-900 rounded-xl shadow-sm border border-gray-200 dark:border-gray-800 overflow-hidden">
                <!-- Avatar Section -->
                <div class="p-8 border-b border-gray-100 dark:border-gray-800 flex flex-col items-center">
                    <div class="relative group">
                        <RcAvatar 
                            :userId="user?.id"
                            :username="user?.username"
                            :src="user?.avatar_url"
                            :size="120"
                            class="ring-4 ring-white dark:ring-gray-900 shadow-lg"
                        />
                        <button 
                            @click="fileInput?.click()"
                            class="absolute inset-0 flex items-center justify-center bg-black/40 text-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
                        >
                            <Camera class="w-8 h-8" />
                        </button>
                        <div v-if="uploading" class="absolute inset-0 flex items-center justify-center bg-black/60 text-white rounded-full z-10">
                            <div class="animate-spin w-8 h-8 border-2 border-white border-t-transparent rounded-full"></div>
                        </div>
                    </div>
                    
                    <input 
                        ref="fileInput"
                        type="file" 
                        class="hidden" 
                        accept="image/*" 
                        @change="handleAvatarUpload"
                    />

                    <div class="mt-4 flex items-center space-x-4">
                        <button 
                            @click="fileInput?.click()"
                            class="text-sm font-medium text-primary hover:text-primary-dark transition-colors"
                        >
                            Change Photo
                        </button>
                        <button 
                            v-if="user?.avatar_url"
                            @click="removeAvatar"
                            class="text-sm font-medium text-red-500 hover:text-red-600 transition-colors flex items-center"
                        >
                            <Trash2 class="w-4 h-4 mr-1" />
                            Remove
                        </button>
                    </div>
                </div>

                <!-- Form Section -->
                <div class="p-8 space-y-6">
                    <div v-if="error" class="p-4 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-lg text-sm">
                        {{ error }}
                    </div>
                    <div v-if="success" class="p-4 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 rounded-lg text-sm">
                        Profile updated successfully!
                    </div>

                    <div class="grid grid-cols-1 gap-6">
                        <div class="space-y-2">
                            <label class="text-sm font-semibold text-gray-700 dark:text-gray-300">Display Name</label>
                            <input 
                                v-model="displayName"
                                type="text" 
                                placeholder="e.g. John Doe"
                                class="w-full bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-2.5 focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none transition-all dark:text-white"
                            />
                            <p class="text-xs text-gray-500">This is how you'll appear to others in RustChat.</p>
                        </div>

                        <div class="space-y-2">
                            <label class="text-sm font-semibold text-gray-700 dark:text-gray-300">Username</label>
                            <div class="relative">
                                <span class="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400 font-medium">@</span>
                                <input 
                                    v-model="username"
                                    type="text" 
                                    class="w-full bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg pl-9 pr-4 py-2.5 focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none transition-all dark:text-white"
                                />
                            </div>
                        </div>

                        <div class="space-y-2 pt-4">
                            <label class="text-sm font-semibold text-gray-700 dark:text-gray-300">Email Address</label>
                            <input 
                                :value="user?.email"
                                type="email" 
                                disabled
                                class="w-full bg-gray-100 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-800 rounded-lg px-4 py-2.5 text-gray-500 cursor-not-allowed"
                            />
                            <p class="text-xs text-gray-500">Email address cannot be changed.</p>
                        </div>

                        <div class="space-y-4 pt-6 mt-6 border-t border-gray-100 dark:border-gray-800">
                            <label class="text-sm font-semibold text-gray-700 dark:text-gray-300">Appearance</label>
                            <div class="flex items-center justify-between">
                                <div>
                                    <p class="text-sm font-medium text-gray-900 dark:text-gray-100">Theme</p>
                                    <p class="text-xs text-gray-500">Choose how RustChat looks to you.</p>
                                </div>
                                <ThemeToggle />
                            </div>
                        </div>
                    </div>

                    <div class="pt-6 flex justify-end">
                        <button 
                            @click="handleUpdateProfile"
                            :disabled="saving"
                            class="bg-primary hover:bg-primary-dark text-white px-8 py-2.5 rounded-lg font-bold shadow-lg shadow-primary/20 transition-all flex items-center space-x-2 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <Save v-if="!saving" class="w-5 h-5" />
                            <div v-else class="animate-spin w-5 h-5 border-2 border-white border-t-transparent rounded-full"></div>
                            <span>{{ saving ? 'Saving...' : 'Save Changes' }}</span>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
