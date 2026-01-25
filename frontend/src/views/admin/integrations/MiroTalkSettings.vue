<template>
  <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 p-6">
    <div class="flex items-center justify-between mb-4">
        <div class="flex items-center">
            <Video class="w-6 h-6 text-blue-500 mr-3" />
            <div>
                <h3 class="font-semibold text-gray-900 dark:text-white">MiroTalk Video Conferencing</h3>
                <p class="text-sm text-gray-500 dark:text-gray-400">Enable SFU or P2P video calls integration</p>
            </div>
        </div>

        <input
            type="checkbox"
            v-model="config.is_active"
            class="w-5 h-5 text-indigo-600 rounded"
        />
    </div>

    <div v-if="loading" class="text-gray-500 dark:text-gray-400">Loading...</div>
    <div v-else-if="config.is_active" class="mt-4 pt-4 border-t border-gray-200 dark:border-slate-700 space-y-4">

        <!-- Mode -->
        <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Mode</label>
            <select v-model="config.mode" class="w-full px-3 py-2 border rounded-lg dark:bg-slate-900 dark:border-gray-600 dark:text-white">
                <option value="sfu">MiroTalk SFU</option>
                <option value="p2p">MiroTalk P2P</option>
            </select>
        </div>

        <!-- Base URL -->
        <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Base URL</label>
            <input type="text" v-model="config.base_url" placeholder="https://mirotalk.example.com" class="w-full px-3 py-2 border rounded-lg dark:bg-slate-900 dark:border-gray-600 dark:text-white">
        </div>

        <!-- API Key -->
        <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">API Key Secret</label>
            <input type="password" v-model="config.api_key_secret" class="w-full px-3 py-2 border rounded-lg dark:bg-slate-900 dark:border-gray-600 dark:text-white">
        </div>

        <!-- Room Prefix -->
        <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Default Room Prefix</label>
            <input type="text" v-model="config.default_room_prefix" placeholder="rustchat" class="w-full px-3 py-2 border rounded-lg dark:bg-slate-900 dark:border-gray-600 dark:text-white">
        </div>

        <!-- Join Behavior -->
        <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Join Behavior</label>
            <select v-model="config.join_behavior" class="w-full px-3 py-2 border rounded-lg dark:bg-slate-900 dark:border-gray-600 dark:text-white">
                <option value="new_tab">Open in New Tab</option>
                <option value="embed_iframe">Embed in Iframe (Modal)</option>
            </select>
        </div>

        <!-- Requirements Warning -->
        <div class="bg-yellow-50 dark:bg-yellow-900/20 p-4 rounded-lg border border-yellow-200 dark:border-yellow-800">
            <div class="flex">
                <div class="flex-shrink-0">
                    <AlertTriangle class="h-5 w-5 text-yellow-600 dark:text-yellow-500" />
                </div>
                <div class="ml-3">
                    <h3 class="text-sm font-medium text-yellow-800 dark:text-yellow-400">Requirements</h3>
                    <div class="mt-2 text-sm text-yellow-700 dark:text-yellow-300">
                        <p>Ensure your MiroTalk instance is reachable and configured with valid SSL. For embedded mode, browser permissions (camera/mic) must be allowed for the iframe origin.</p>
                    </div>
                </div>
            </div>
        </div>

        <!-- Actions -->
        <div class="flex items-center justify-between pt-4 mt-4 border-t border-gray-200 dark:border-slate-700">
             <button
                @click="testConnection"
                :disabled="!config.is_active || testing"
                class="inline-flex items-center px-4 py-2 border border-gray-300 dark:border-gray-600 shadow-sm text-sm font-medium rounded-md text-gray-700 dark:text-gray-300 bg-white dark:bg-slate-700 hover:bg-gray-50 dark:hover:bg-slate-600 focus:outline-none"
            >
                <span v-if="testing">Testing...</span>
                <span v-else>Save & Test Connection</span>
            </button>

            <button
                @click="save(false)"
                :disabled="saving"
                class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none"
            >
                <span v-if="saving">Saving...</span>
                <span v-else>Save Configuration</span>
            </button>
        </div>

        <!-- Test Result -->
        <div v-if="testResult" class="mt-4 p-4 rounded-md overflow-x-auto" :class="testSuccess ? 'bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400' : 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400'">
            <pre class="text-xs whitespace-pre-wrap">{{ testResult }}</pre>
        </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { adminApi, type MiroTalkConfig } from '../../../api/admin';
import { Video, AlertTriangle } from 'lucide-vue-next';

const loading = ref(true);
const saving = ref(false);
const testing = ref(false);
const testResult = ref<string | null>(null);
const testSuccess = ref(false);

const config = ref<MiroTalkConfig>({
    is_active: false,
    mode: 'disabled',
    base_url: '',
    api_key_secret: '',
    default_room_prefix: '',
    join_behavior: 'new_tab'
});

onMounted(async () => {
    try {
        const { data } = await adminApi.getMiroTalkConfig();
        // Backend returns default if not found
        config.value = data;
        if (config.value.mode === 'disabled' && config.value.is_active) {
             config.value.mode = 'sfu';
        }
    } catch (e) {
        console.error("Failed to load MiroTalk config", e);
    } finally {
        loading.value = false;
    }
});

// Watch for is_active toggle to auto-expand/collapse or auto-save if needed
// Actually we only save on button click.

async function save(silent = false) {
    saving.value = true;
    try {
        const payload = { ...config.value };
        if (!payload.is_active) {
            payload.mode = 'disabled';
        }
        const { data } = await adminApi.updateMiroTalkConfig(payload);
        config.value = data;
        if (!silent) alert('MiroTalk configuration saved successfully.');
    } catch (e) {
        console.error(e);
        if (!silent) alert('Failed to save configuration.');
        throw e;
    } finally {
        saving.value = false;
    }
}

async function testConnection() {
    testing.value = true;
    testResult.value = null;
    try {
        await save(true); // Auto-save silently
        const { data } = await adminApi.testMiroTalkConnection();
        testSuccess.value = true;
        testResult.value = JSON.stringify(data, null, 2);
    } catch (e: any) {
        testSuccess.value = false;
        testResult.value = e.response?.data?.error?.message || e.message || 'Connection failed';
    } finally {
        testing.value = false;
    }
}
</script>
