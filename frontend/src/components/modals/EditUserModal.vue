<script setup lang="ts">
import { ref, watch } from 'vue';
import { X, UserCog, Shield, Type } from 'lucide-vue-next';
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
    { value: 'member', label: 'Member', description: 'Standard user with basic permissions' },
    { value: 'team_admin', label: 'Team Admin', description: 'Can manage teams and channels' },
    { value: 'org_admin', label: 'Org Admin', description: 'Can manage organization-wide settings' },
    { value: 'system_admin', label: 'System Admin', description: 'Full administrative access' },
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
            <div class="relative bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 w-full max-w-md overflow-hidden">
                <!-- Header -->
                <div class="flex items-center justify-between px-5 py-4 border-b border-border-1">
                    <div class="flex items-center gap-2.5">
                        <div class="rounded-lg bg-brand/10 p-1.5">
                            <UserCog class="w-4 h-4 text-brand" />
                        </div>
                        <div>
                            <h2 class="text-sm font-semibold text-text-1">Edit User</h2>
                            <p v-if="user" class="text-[10px] text-text-3">@{{ user.username }}</p>
                        </div>
                    </div>
                    <button @click="close" class="p-1.5 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-4" />
                    </button>
                </div>

                <!-- Form -->
                <form @submit.prevent="submit" class="p-5 space-y-4">
                    <!-- Error -->
                    <div v-if="error" class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs">
                        {{ error }}
                    </div>

                    <div v-if="user" class="mb-4 p-3 bg-bg-surface-2 rounded-lg flex items-center gap-3">
                        <div class="w-10 h-10 rounded-full bg-brand/10 flex items-center justify-center text-brand font-bold text-sm shrink-0">
                            {{ user.username.charAt(0).toUpperCase() }}
                        </div>
                        <div class="min-w-0">
                            <p class="text-xs font-medium text-text-1 truncate">{{ user.username }}</p>
                            <p class="text-[10px] text-text-3 truncate">{{ user.email }}</p>
                        </div>
                    </div>

                    <!-- Display Name -->
                    <div>
                        <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                            <Type class="w-3.5 h-3.5 text-text-4" />
                            Display Name
                        </label>
                        <input 
                            v-model="form.display_name"
                            type="text"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                            placeholder="John Doe"
                        />
                    </div>

                    <!-- Role -->
                    <div>
                        <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                            <Shield class="w-3.5 h-3.5 text-text-4" />
                            Role
                        </label>
                        <div class="space-y-1.5">
                            <label 
                                v-for="role in roles" 
                                :key="role.value"
                                class="flex items-start gap-2.5 p-2.5 rounded-lg border cursor-pointer transition-all"
                                :class="form.role === role.value ? 'border-brand bg-brand/5' : 'border-border-1 hover:bg-bg-surface-2'"
                            >
                                <input 
                                    v-model="form.role"
                                    type="radio"
                                    :value="role.value"
                                    class="w-3.5 h-3.5 text-brand mt-0.5"
                                />
                                <div>
                                    <div class="text-xs font-medium text-text-1">{{ role.label }}</div>
                                    <div class="text-[10px] text-text-3">{{ role.description }}</div>
                                </div>
                            </label>
                        </div>
                    </div>

                    <!-- Actions -->
                    <div class="flex justify-end gap-2 pt-4">
                        <button 
                            type="button"
                            @click="close"
                            class="px-3 py-2 text-text-2 hover:bg-bg-surface-2 rounded-lg text-xs font-medium transition-colors"
                        >
                            Cancel
                        </button>
                        <button 
                            type="submit"
                            :disabled="submitting"
                            class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
                        >
                            {{ submitting ? 'Saving...' : 'Save Changes' }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </Teleport>
</template>
