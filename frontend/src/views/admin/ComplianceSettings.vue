<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useAdminStore } from '../../features/admin/stores/adminStore'
import { Scale, Trash2, Save, AlertCircle, CheckCircle, Download } from 'lucide-vue-next'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

const adminStore = useAdminStore()

const form = ref({
  message_retention_days: 0,
  file_retention_days: 0,
})

const saving = ref(false)
const saveSuccess = ref(false)
const saveError = ref('')

onMounted(async () => {
  await adminStore.fetchConfig()
  if (adminStore.config?.compliance) {
    form.value = { ...form.value, ...adminStore.config.compliance }
  }
})

watch(
  () => adminStore.config?.compliance,
  compliance => {
    if (compliance) {
      form.value = { ...form.value, ...compliance }
    }
  }
)

const saveSettings = async () => {
  saving.value = true
  saveError.value = ''
  saveSuccess.value = false

  try {
    await adminStore.updateConfig('compliance', form.value)
    saveSuccess.value = true
    setTimeout(() => (saveSuccess.value = false), 3000)
  } catch (e: unknown) {
    saveError.value = getApiErrorMessage(e) || 'Failed to save settings'
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold text-gray-900">Compliance & Retention</h1>
        <p class="text-gray-500 mt-1">Configure data retention policies</p>
      </div>
      <div class="flex items-center gap-3">
        <span v-if="saveSuccess" class="flex items-center text-green-600 text-sm">
          <CheckCircle class="w-4 h-4 mr-1" /> Saved
        </span>
        <button
          :disabled="saving"
          class="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
          @click="saveSettings"
        >
          <Save class="w-5 h-5 mr-2" />
          {{ saving ? 'Saving...' : 'Save Changes' }}
        </button>
      </div>
    </div>

    <!-- Error Alert -->
    <div
      v-if="saveError"
      class="flex items-center gap-2 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700"
    >
      <AlertCircle class="w-5 h-5 shrink-0" />
      {{ saveError }}
    </div>

    <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
      <div class="flex items-center mb-6">
        <Scale class="w-5 h-5 text-gray-400 mr-2" />
        <h2 class="text-lg font-semibold text-gray-900">Global Retention Policy</h2>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1"
            >Message Retention (days)</label
          >
          <input
            v-model.number="form.message_retention_days"
            type="number"
            min="0"
            class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
          />
          <p class="text-xs text-gray-500 mt-1">0 = Keep forever</p>
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">File Retention (days)</label>
          <input
            v-model.number="form.file_retention_days"
            type="number"
            min="0"
            class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
          />
          <p class="text-xs text-gray-500 mt-1">0 = Keep forever</p>
        </div>
      </div>

      <div
        v-if="form.message_retention_days > 0 || form.file_retention_days > 0"
        class="mt-6 p-4 bg-yellow-50 rounded-lg border border-yellow-200"
      >
        <div class="flex items-start">
          <Trash2 class="w-5 h-5 text-yellow-600 mr-3 mt-0.5" />
          <div>
            <p class="font-medium text-yellow-800">Data Deletion Warning</p>
            <p class="text-sm text-yellow-700 mt-1">
              With these settings, data older than the retention period will be permanently deleted.
              This action cannot be undone.
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- Compliance Export -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center">
          <Download class="w-5 h-5 text-gray-400 mr-2" />
          <h2 class="text-lg font-semibold text-gray-900">Compliance Export</h2>
        </div>
        <span class="px-2 py-1 text-xs font-medium bg-gray-100 text-gray-600 rounded-full"
          >Not implemented</span
        >
      </div>
      <p class="text-sm text-gray-500 mb-4">
        Exporting all system data (messages, files, logs) for compliance auditing is not implemented
        in this build. Retention settings above are enforced; a full compliance export is tracked on
        the roadmap.
      </p>
    </div>
  </div>
</template>
