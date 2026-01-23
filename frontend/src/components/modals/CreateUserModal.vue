<script setup lang="ts">
import { ref, computed } from 'vue';
import { X, UserPlus, Eye, EyeOff } from 'lucide-vue-next';
import { useAdminStore } from '../../stores/admin';

const props = defineProps<{
    open: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'created'): void
}>();

const adminStore = useAdminStore();

const form = ref({
    username: '',
    email: '',
    password: '',
    confirmPassword: '',
    display_name: '',
    role: 'member',
});

const showPassword = ref(false);
const submitting = ref(false);
const error = ref('');

const roles = [
    { value: 'member', label: 'Member' },
    { value: 'team_admin', label: 'Team Admin' },
    { value: 'org_admin', label: 'Org Admin' },
    { value: 'system_admin', label: 'System Admin' },
];

const isValid = computed(() => {
    return form.value.username.length >= 3 &&
           form.value.email.includes('@') &&
           form.value.password.length >= 8 &&
           form.value.password === form.value.confirmPassword;
});

const passwordErrors = computed(() => {
    const errors: string[] = [];
    if (form.value.password && form.value.password.length < 8) {
        errors.push('Password must be at least 8 characters');
    }
    if (form.value.confirmPassword && form.value.password !== form.value.confirmPassword) {
        errors.push('Passwords do not match');
    }
    return errors;
});

async function submit() {
    if (!isValid.value) return;
    
    submitting.value = true;
    error.value = '';
    
    try {
        await adminStore.createUser({
            username: form.value.username,
            email: form.value.email,
            password: form.value.password,
            role: form.value.role,
            display_name: form.value.display_name || undefined,
        });
        
        // Reset form
        form.value = {
            username: '',
            email: '',
            password: '',
            confirmPassword: '',
            display_name: '',
            role: 'member',
        };
        
        emit('created');
        emit('close');
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to create user';
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
                        <UserPlus class="w-5 h-5 text-indigo-500" />
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Create User</h2>
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

                    <!-- Username -->
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Username *</label>
                        <input 
                            v-model="form.username"
                            type="text"
                            required
                            minlength="3"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500"
                            placeholder="johndoe"
                        />
                    </div>

                    <!-- Email -->
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Email *</label>
                        <input 
                            v-model="form.email"
                            type="email"
                            required
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500"
                            placeholder="john@example.com"
                        />
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

                    <!-- Password -->
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Password *</label>
                        <div class="relative">
                            <input 
                                v-model="form.password"
                                :type="showPassword ? 'text' : 'password'"
                                required
                                minlength="8"
                                class="w-full px-4 py-2 pr-10 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500"
                                placeholder="Min 8 characters"
                            />
                            <button 
                                type="button"
                                @click="showPassword = !showPassword"
                                class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
                            >
                                <Eye v-if="!showPassword" class="w-4 h-4" />
                                <EyeOff v-else class="w-4 h-4" />
                            </button>
                        </div>
                    </div>

                    <!-- Confirm Password -->
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Confirm Password *</label>
                        <input 
                            v-model="form.confirmPassword"
                            :type="showPassword ? 'text' : 'password'"
                            required
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500"
                            placeholder="Repeat password"
                        />
                    </div>

                    <!-- Validation Errors -->
                    <div v-if="passwordErrors.length" class="text-sm text-red-500 space-y-1">
                        <p v-for="err in passwordErrors" :key="err">{{ err }}</p>
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
                            :disabled="!isValid || submitting"
                            class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors"
                        >
                            {{ submitting ? 'Creating...' : 'Create User' }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </Teleport>
</template>
