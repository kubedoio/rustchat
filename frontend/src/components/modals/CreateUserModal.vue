<script setup lang="ts">
import { ref, computed } from 'vue';
import { X, UserPlus, Eye, EyeOff, Shield, Type, Mail, Lock } from 'lucide-vue-next';
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
    { value: 'member', label: 'Member', description: 'Standard user with basic permissions' },
    { value: 'team_admin', label: 'Team Admin', description: 'Can manage teams and channels' },
    { value: 'org_admin', label: 'Org Admin', description: 'Can manage organization-wide settings' },
    { value: 'system_admin', label: 'System Admin', description: 'Full administrative access' },
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
            <div class="relative bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 w-full max-w-md overflow-hidden">
                <!-- Header -->
                <div class="flex items-center justify-between px-5 py-4 border-b border-border-1">
                    <div class="flex items-center gap-2.5">
                        <div class="rounded-lg bg-brand/10 p-1.5">
                            <UserPlus class="w-4 h-4 text-brand" />
                        </div>
                        <h2 class="text-sm font-semibold text-text-1">Create User</h2>
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

                    <!-- Username -->
                    <div>
                        <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                            <Type class="w-3.5 h-3.5 text-text-4" />
                            Username *
                        </label>
                        <input 
                            v-model="form.username"
                            type="text"
                            required
                            minlength="3"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                            placeholder="johndoe"
                        />
                    </div>

                    <!-- Email -->
                    <div>
                        <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                            <Mail class="w-3.5 h-3.5 text-text-4" />
                            Email *
                        </label>
                        <input 
                            v-model="form.email"
                            type="email"
                            required
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                            placeholder="john@example.com"
                        />
                    </div>

                    <!-- Display Name -->
                    <div>
                        <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                            <UserPlus class="w-3.5 h-3.5 text-text-4" />
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
                        <select 
                            v-model="form.role"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                        >
                            <option v-for="role in roles" :key="role.value" :value="role.value">
                                {{ role.label }}
                            </option>
                        </select>
                    </div>

                    <!-- Password -->
                    <div>
                        <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                            <Lock class="w-3.5 h-3.5 text-text-4" />
                            Password *
                        </label>
                        <div class="relative">
                            <input 
                                v-model="form.password"
                                :type="showPassword ? 'text' : 'password'"
                                required
                                minlength="8"
                                class="w-full px-3 py-2 pr-10 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                                placeholder="Min 8 characters"
                            />
                            <button 
                                type="button"
                                @click="showPassword = !showPassword"
                                class="absolute right-3 top-1/2 -translate-y-1/2 text-text-4 hover:text-text-2 p-1"
                            >
                                <Eye v-if="!showPassword" class="w-3.5 h-3.5" />
                                <EyeOff v-else class="w-3.5 h-3.5" />
                            </button>
                        </div>
                    </div>

                    <!-- Confirm Password -->
                    <div>
                        <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                            <Lock class="w-3.5 h-3.5 text-text-4" />
                            Confirm Password *
                        </label>
                        <input 
                            v-model="form.confirmPassword"
                            :type="showPassword ? 'text' : 'password'"
                            required
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                            placeholder="Repeat password"
                        />
                    </div>

                    <!-- Validation Errors -->
                    <div v-if="passwordErrors.length" class="text-xs text-danger space-y-1">
                        <p v-for="err in passwordErrors" :key="err">{{ err }}</p>
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
                            :disabled="!isValid || submitting"
                            class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
                        >
                            {{ submitting ? 'Creating...' : 'Create User' }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </Teleport>
</template>
