<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { Webhook, Terminal, Bot, Save, AlertCircle, CheckCircle } from 'lucide-vue-next';

const adminStore = useAdminStore();

const form = ref({
    enable_webhooks: true,
    enable_slash_commands: true,
    enable_bots: true,
    max_webhooks_per_team: 10,
    webhook_payload_size_kb: 100,
});

const saving = ref(false);
const saveSuccess = ref(false);
const saveError = ref('');

onMounted(async () => {
    await adminStore.fetchConfig();
    if (adminStore.config?.integrations) {
        form.value = { ...form.value, ...adminStore.config.integrations };
    }
});

// Sync form when config changes
watch(() => adminStore.config?.integrations, (integrations) => {
    if (integrations) {
        form.value = { ...form.value, ...integrations };
    }
});

const saveSettings = async () => {
    saving.value = true;
    saveError.value = '';
    saveSuccess.value = false;
    
    try {
        await adminStore.updateConfig('integrations', form.value);
        saveSuccess.value = true;
        setTimeout(() => saveSuccess.value = false, 3000);
    } catch (e: any) {
        saveError.value = e.response?.data?.message || 'Failed to save settings';
    } finally {
        saving.value = false;
    }
};
</script>

<template>
    <div class="space-y-6">
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Integrations</h1>
                <p class="text-gray-500 dark:text-gray-400 mt-1">Configure webhooks, slash commands, and bots</p>
            </div>
            <div class="flex items-center gap-3">
                <span v-if="saveSuccess" class="flex items-center text-green-600 text-sm">
                    <CheckCircle class="w-4 h-4 mr-1" /> Saved
                </span>
                <button 
                    @click="saveSettings"
                    :disabled="saving"
                    class="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
                >
                    <Save class="w-5 h-5 mr-2" />
                    {{ saving ? 'Saving...' : 'Save Changes' }}
                </button>
            </div>
        </div>

        <!-- Error Alert -->
        <div v-if="saveError" class="flex items-center gap-2 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-700 dark:text-red-400">
            <AlertCircle class="w-5 h-5 shrink-0" />
            {{ saveError }}
        </div>

        <div class="space-y-4">
            <!-- Webhooks -->
            <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 p-6">
                <div class="flex items-center justify-between mb-4">
                    <div class="flex items-center">
                        <Webhook class="w-6 h-6 text-indigo-500 mr-3" />
                        <div>
                            <h3 class="font-semibold text-gray-900 dark:text-white">Webhooks</h3>
                            <p class="text-sm text-gray-500">Incoming and outgoing webhooks for integrations</p>
                        </div>
                    </div>
                    <input type="checkbox" v-model="form.enable_webhooks" class="w-5 h-5 text-indigo-600 rounded" />
                </div>
                <div v-if="form.enable_webhooks" class="grid grid-cols-2 gap-4 mt-4 pt-4 border-t border-gray-200 dark:border-slate-700">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Max per Team</label>
                        <input v-model.number="form.max_webhooks_per_team" type="number" class="w-full px-3 py-2 border rounded-lg dark:bg-slate-900 dark:border-gray-600 dark:text-white" />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Max Payload (KB)</label>
                        <input v-model.number="form.webhook_payload_size_kb" type="number" class="w-full px-3 py-2 border rounded-lg dark:bg-slate-900 dark:border-gray-600 dark:text-white" />
                    </div>
                </div>
            </div>

            <!-- Slash Commands -->
            <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 p-6">
                <div class="flex items-center justify-between">
                    <div class="flex items-center">
                        <Terminal class="w-6 h-6 text-green-500 mr-3" />
                        <div>
                            <h3 class="font-semibold text-gray-900 dark:text-white">Slash Commands</h3>
                            <p class="text-sm text-gray-500">Custom commands for teams and channels</p>
                        </div>
                    </div>
                    <input type="checkbox" v-model="form.enable_slash_commands" class="w-5 h-5 text-indigo-600 rounded" />
                </div>
            </div>

            <!-- Bots -->
            <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 p-6">
                <div class="flex items-center justify-between">
                    <div class="flex items-center">
                        <Bot class="w-6 h-6 text-purple-500 mr-3" />
                        <div>
                            <h3 class="font-semibold text-gray-900 dark:text-white">Bot Accounts</h3>
                            <p class="text-sm text-gray-500">Allow creation of bot users for automation</p>
                        </div>
                    </div>
                    <input type="checkbox" v-model="form.enable_bots" class="w-5 h-5 text-indigo-600 rounded" />
                </div>
            </div>
        </div>
    </div>
</template>

