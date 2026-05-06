<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { Key, Lock, Users } from 'lucide-vue-next';

const adminStore = useAdminStore();

const authForm = ref({
    enable_email_password: true,
    enable_sso: false,
    require_sso: false,
    allow_registration: true,
    password_min_length: 8,
    password_require_lowercase: true,
    password_require_uppercase: true,
    password_require_number: true,
    password_require_symbol: false,
    password_enable_forgot_link: true,
    session_length_hours: 24,
});

const saving = ref(false);

onMounted(async () => {
    await adminStore.fetchConfig();
    if (adminStore.config?.authentication) {
        authForm.value = { ...authForm.value, ...adminStore.config.authentication };
    }
});

const saveSettings = async () => {
    saving.value = true;
    try {
        await adminStore.updateConfig('authentication', authForm.value);
    } finally {
        saving.value = false;
    }
};
</script>

<template>
    <div class="space-y-6">
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">Security Settings</h1>
                <p class="text-gray-500 mt-1">Configure authentication and access policies</p>
            </div>
            <button 
                @click="saveSettings"
                :disabled="saving"
                class="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
            >
                {{ saving ? 'Saving...' : 'Save Changes' }}
            </button>
        </div>

        <!-- Authentication Methods -->
        <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <div class="flex items-center mb-6">
                <Key class="w-5 h-5 text-gray-400 mr-2" />
                <h2 class="text-lg font-semibold text-gray-900">Authentication Methods</h2>
            </div>
            
            <div class="space-y-4">
                <label class="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                    <div>
                        <p class="font-medium text-gray-900">Email & Password</p>
                        <p class="text-sm text-gray-500">Allow users to sign in with email and password</p>
                    </div>
                    <input type="checkbox" v-model="authForm.enable_email_password" class="w-5 h-5 text-indigo-600 rounded" />
                </label>

                <label class="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                    <div>
                        <p class="font-medium text-gray-900">Single Sign-On (OIDC)</p>
                        <p class="text-sm text-gray-500">Enable login via external identity provider</p>
                    </div>
                    <input type="checkbox" v-model="authForm.enable_sso" class="w-5 h-5 text-indigo-600 rounded" />
                </label>

                <label v-if="authForm.enable_sso" class="flex items-center justify-between p-4 bg-yellow-50 rounded-lg border border-yellow-200">
                    <div>
                        <p class="font-medium text-yellow-800">Require SSO</p>
                        <p class="text-sm text-yellow-600">Disable password login, require SSO only</p>
                    </div>
                    <input type="checkbox" v-model="authForm.require_sso" class="w-5 h-5 text-yellow-600 rounded" />
                </label>
            </div>
        </div>

        <!-- Password Policy -->
        <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <div class="flex items-center mb-6">
                <Lock class="w-5 h-5 text-gray-400 mr-2" />
                <h2 class="text-lg font-semibold text-gray-900">Password Policy</h2>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Minimum Length</label>
                    <input 
                        v-model.number="authForm.password_min_length"
                        type="number"
                        min="6"
                        max="32"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Session Length (hours)</label>
                    <input 
                        v-model.number="authForm.session_length_hours"
                        type="number"
                        min="1"
                        max="720"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
                    />
                </div>
            </div>
            
            <div class="mt-4 space-y-2">
                <label class="flex items-center">
                    <input type="checkbox" v-model="authForm.password_require_lowercase" class="w-4 h-4 text-indigo-600 rounded mr-3" />
                    <span class="text-gray-700">Require lowercase letter</span>
                </label>
                <label class="flex items-center">
                    <input type="checkbox" v-model="authForm.password_require_uppercase" class="w-4 h-4 text-indigo-600 rounded mr-3" />
                    <span class="text-gray-700">Require uppercase letter</span>
                </label>
                <label class="flex items-center">
                    <input type="checkbox" v-model="authForm.password_require_number" class="w-4 h-4 text-indigo-600 rounded mr-3" />
                    <span class="text-gray-700">Require number</span>
                </label>
                <label class="flex items-center">
                    <input type="checkbox" v-model="authForm.password_require_symbol" class="w-4 h-4 text-indigo-600 rounded mr-3" />
                    <span class="text-gray-700">Require symbol</span>
                </label>
                <label class="flex items-center">
                    <input type="checkbox" v-model="authForm.password_enable_forgot_link" class="w-4 h-4 text-indigo-600 rounded mr-3" />
                    <span class="text-gray-700">Enable forgot password link</span>
                </label>
            </div>
        </div>

        <!-- Registration -->
        <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <div class="flex items-center mb-6">
                <Users class="w-5 h-5 text-gray-400 mr-2" />
                <h2 class="text-lg font-semibold text-gray-900">Registration</h2>
            </div>
            
            <label class="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                <div>
                    <p class="font-medium text-gray-900">Allow Public Registration</p>
                    <p class="text-sm text-gray-500">Anyone can create an account</p>
                </div>
                <input type="checkbox" v-model="authForm.allow_registration" class="w-5 h-5 text-indigo-600 rounded" />
            </label>
        </div>
    </div>
</template>
