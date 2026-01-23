<script setup lang="ts">
import { ref, watch } from 'vue';
import { X, UserCog } from 'lucide-vue-next';
import { useAdminStore } from '../../stores/admin';
import type { AdminUser } from '../../api/admin';

const props = defineProps<{
    open: boolean
    user: AdminUser | null
}>();

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'updated'): void
}>();

const adminStore = useAdminStore();

const form = ref({
    display_name: '',
    role: '',
});

const submitting = ref(false);
const error = ref('');

const roles = [
    { value: 'member', label: 'Member' },
    { value: 'team_admin', label: 'Team Admin' },
    { value: 'org_admin', label: 'Org Admin' },
    { value: 'system_admin', label: 'System Admin' },
];

watch(() => props.user, (newUser) => {
    if (newUser) {
        form.value = {
            display_name: newUser.display_name || '',
            role: newUser.role,
        };
    }
}, { immediate: true });

async function submit() {
    if (!props.user) return;
    
    submitting.value = true;
    error.value = '';
    
    try {
        await adminStore.updateUser(props.user.id, {
            role: form.value.role,
            display_name: form.value.display_name || undefined,
        });
        
        emit('updated');
        emit('close');
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to update user';
    } finally {
        submitting.value = false;
    }
}

function close() {
    error.value = '';
    emit('close');
}
</script>

<template>
    <Teleport to="body">
        <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center">
            <!-- Backdrop -->
            <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="close"></div>
            
            <!-- Modal -->
            <div class="relative bg-white dark:bg-slate-900 rounded-xl shadow-2xl border border-gray-200 dark:border-slate-700 w-full max-w-md overflow-hidden">
                <!-- Header -->
                <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-slate-700">
                    <div class="flex items-center space-x-3">
                        <UserCog class="w-5 h-5 text-indigo-500" />
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Edit User</h2>
                    </div>
                    <button @click="close" class="p-1 hover:bg-gray-100 dark:hover:bg-slate-800 rounded-lg transition-colors">
                        <X class="w-5 h-5 text-gray-400" />
                    </button>
                </div>

                <!-- Form -->
                <form @submit.prevent="submit" class="p-6 space-y-4">
                    <!-- Error -->
                    <div v-if="error" class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-700 dark:text-red-400 text-sm">
                        {{ error }}
                    </div>

                    <div v-if="user" class="mb-4 p-3 bg-gray-50 dark:bg-slate-800/50 rounded-lg flex items-center space-x-3">
                        <div class="w-10 h-10 rounded-full bg-indigo-100 dark:bg-indigo-900/30 flex items-center justify-center text-indigo-600 dark:text-indigo-400 font-bold shrink-0">
                            {{ user.username.charAt(0).toUpperCase() }}
                        </div>
                        <div class="min-w-0">
                            <p class="text-sm font-medium text-gray-900 dark:text-white truncate">{{ user.username }}</p>
                            <p class="text-xs text-gray-500 truncate">{{ user.email }}</p>
                        </div>
                    </div>

                    <!-- Display Name -->
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Display Name</label>
                        <input 
                            v-model="form.display_name"
                            type="text"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500"
                            placeholder="John Doe"
                        />
                    </div>

                    <!-- Role -->
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Role</label>
                        <select 
                            v-model="form.role"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500"
                        >
                            <option v-for="role in roles" :key="role.value" :value="role.value">
                                {{ role.label }}
                            </option>
                        </select>
                    </div>

                    <!-- Actions -->
                    <div class="flex justify-end space-x-3 pt-4">
                        <button 
                            type="button"
                            @click="close"
                            class="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-slate-800 rounded-lg transition-colors"
                        >
                            Cancel
                        </button>
                        <button 
                            type="submit"
                            :disabled="submitting"
                            class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors"
                        >
                            {{ submitting ? 'Saving...' : 'Save Changes' }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </Teleport>
</template>
