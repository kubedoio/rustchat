<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { Scale, Trash2, Save, AlertCircle, CheckCircle, Download } from 'lucide-vue-next';
import api from '../../api/client';

const adminStore = useAdminStore();

const form = ref({
    message_retention_days: 0,
    file_retention_days: 0,
});

const saving = ref(false);
const saveSuccess = ref(false);
const saveError = ref('');

const exporting = ref(false);
const exportSuccess = ref(false);

onMounted(async () => {
    await adminStore.fetchConfig();
    if (adminStore.config?.compliance) {
        form.value = { ...form.value, ...adminStore.config.compliance };
    }
});

watch(() => adminStore.config?.compliance, (compliance) => {
    if (compliance) {
        form.value = { ...form.value, ...compliance };
    }
});

const saveSettings = async () => {
    saving.value = true;
    saveError.value = '';
    saveSuccess.value = false;
    
    try {
        await adminStore.updateConfig('compliance', form.value);
        saveSuccess.value = true;
        setTimeout(() => saveSuccess.value = false, 3000);
    } catch (e: any) {
        saveError.value = e.response?.data?.message || 'Failed to save settings';
    } finally {
        saving.value = false;
    }
};

const triggerExport = async () => {
    exporting.value = true;
    try {
        await api.post('/admin/compliance/export');
        exportSuccess.value = true;
        setTimeout(() => exportSuccess.value = false, 5000);
    } catch (e) {
        console.error(e);
    } finally {
        exporting.value = false;
    }
};
</script>

<template>
    <div class="space-y-6">
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Compliance & Retention</h1>
                <p class="text-gray-500 dark:text-gray-400 mt-1">Configure data retention policies</p>
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

        <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 p-6">
            <div class="flex items-center mb-6">
                <Scale class="w-5 h-5 text-gray-400 mr-2" />
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Global Retention Policy</h2>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Message Retention (days)</label>
                    <input 
                        v-model.number="form.message_retention_days"
                        type="number"
                        min="0"
                        class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                    />
                    <p class="text-xs text-gray-500 mt-1">0 = Keep forever</p>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">File Retention (days)</label>
                    <input 
                        v-model.number="form.file_retention_days"
                        type="number"
                        min="0"
                        class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                    />
                    <p class="text-xs text-gray-500 mt-1">0 = Keep forever</p>
                </div>
            </div>

            <div v-if="form.message_retention_days > 0 || form.file_retention_days > 0" class="mt-6 p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
                <div class="flex items-start">
                    <Trash2 class="w-5 h-5 text-yellow-600 dark:text-yellow-400 mr-3 mt-0.5" />
                    <div>
                        <p class="font-medium text-yellow-800 dark:text-yellow-200">Data Deletion Warning</p>
                        <p class="text-sm text-yellow-700 dark:text-yellow-300 mt-1">
                            With these settings, data older than the retention period will be permanently deleted. 
                            This action cannot be undone.
                        </p>
                    </div>
                </div>
            </div>
        </div>

        <!-- Compliance Export -->
        <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 p-6">
            <div class="flex items-center justify-between mb-4">
                <div class="flex items-center">
                    <Download class="w-5 h-5 text-gray-400 mr-2" />
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Compliance Export</h2>
                </div>
            </div>
            <p class="text-sm text-gray-500 mb-4">
                Export all system data (messages, files, logs) for compliance auditing purposes.
                The export will be generated in the background and a download link will be emailed to you.
            </p>
            
            <div v-if="exportSuccess" class="mb-4 p-3 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400 rounded-lg text-sm">
                Compliance export started successfully.
            </div>

            <button 
                @click="triggerExport"
                :disabled="exporting"
                class="px-4 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-slate-700 rounded-lg text-gray-700 dark:text-gray-300 font-medium transition-colors"
            >
                {{ exporting ? 'Starting Export...' : 'Start Compliance Export' }}
            </button>
        </div>
    </div>
</template>

