<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { Save, Globe, Upload, Clock, Activity } from 'lucide-vue-next';

const adminStore = useAdminStore();

const form = ref({
    site_name: '',
    logo_url: '',
    site_description: '',
    site_url: '',
    max_file_size_mb: 50,
    max_simultaneous_connections: 5,
    default_locale: 'en',
    default_timezone: 'UTC',
});

const saving = ref(false);

onMounted(async () => {
    await adminStore.fetchConfig();
    if (adminStore.config?.site) {
        form.value = { ...form.value, ...adminStore.config.site };
    }
});

watch(() => adminStore.config?.site, (site) => {
    if (site) {
        form.value = { ...form.value, ...site };
    }
});

const saveSettings = async () => {
    saving.value = true;
    try {
        await adminStore.updateConfig('site', form.value);
    } finally {
        saving.value = false;
    }
};
</script>

<template>
    <div class="space-y-6">
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Server Settings</h1>
                <p class="text-gray-500 dark:text-gray-400 mt-1">Configure your RustChat instance</p>
            </div>
            <button 
                @click="saveSettings"
                :disabled="saving"
                class="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
            >
                <Save class="w-5 h-5 mr-2" />
                {{ saving ? 'Saving...' : 'Save Changes' }}
            </button>
        </div>

        <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 divide-y divide-gray-200 dark:divide-slate-700">
            <!-- Site Information -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Globe class="w-5 h-5 text-gray-400 mr-2" />
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Site Information</h2>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Site Name</label>
                        <input 
                            v-model="form.site_name"
                            type="text"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                            placeholder="RustChat"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Site URL</label>
                        <input 
                            v-model="form.site_url"
                            type="url"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                            placeholder="https://chat.example.com"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Logo URL (50x50 recommended)</label>
                        <input 
                            v-model="form.logo_url"
                            type="text"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                            placeholder="https://example.com/logo.png"
                        />
                    </div>
                    <div class="md:col-span-2">
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Site Description</label>
                        <textarea 
                            v-model="form.site_description"
                            rows="2"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                            placeholder="A self-hosted team collaboration platform"
                        ></textarea>
                    </div>
                </div>
            </div>

            <!-- File Uploads -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Upload class="w-5 h-5 text-gray-400 mr-2" />
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">File Uploads</h2>
                </div>
                <div class="max-w-xs">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Max File Size (MB)</label>
                    <input 
                        v-model.number="form.max_file_size_mb"
                        type="number"
                        min="1"
                        max="500"
                        class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                    />
                </div>
            </div>

            <!-- Connection Limits -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Activity class="w-5 h-5 text-gray-400 mr-2" />
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Connection Limits</h2>
                </div>
                <div class="max-w-xs">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Max Simultaneous Connections per User</label>
                    <input 
                        v-model.number="form.max_simultaneous_connections"
                        type="number"
                        min="1"
                        max="100"
                        class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                    />
                </div>
            </div>

            <!-- Localization -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Clock class="w-5 h-5 text-gray-400 mr-2" />
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Localization</h2>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Default Locale</label>
                        <select 
                            v-model="form.default_locale"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                        >
                            <option value="en">English</option>
                            <option value="es">Spanish</option>
                            <option value="fr">French</option>
                            <option value="de">German</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Default Timezone</label>
                        <select 
                            v-model="form.default_timezone"
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                        >
                            <option value="UTC">UTC</option>
                            <option value="America/New_York">Eastern Time</option>
                            <option value="America/Los_Angeles">Pacific Time</option>
                            <option value="Europe/London">London</option>
                            <option value="Europe/Paris">Paris</option>
                        </select>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
