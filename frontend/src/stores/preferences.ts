import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { preferencesApi, type UserStatus, type UserPreferences, type StatusPreset } from '../api/preferences'

export const usePreferencesStore = defineStore('preferences', () => {
    const status = ref<UserStatus | null>(null)
    const preferences = ref<UserPreferences | null>(null)
    const statusPresets = ref<StatusPreset[]>([])
    const loading = ref(false)

    const hasStatus = computed(() =>
        status.value && (status.value.text || status.value.emoji)
    )

    const statusDisplay = computed(() => {
        if (!status.value) return null
        if (status.value.emoji && status.value.text) {
            return `${status.value.emoji} ${status.value.text}`
        }
        return status.value.emoji || status.value.text
    })

    async function fetchStatus() {
        try {
            const response = await preferencesApi.getMyStatus()
            status.value = response.data
        } catch (e) {
            console.error('Failed to fetch status:', e)
        }
    }

    async function updateStatus(data: { text?: string; emoji?: string; duration_minutes?: number }) {
        loading.value = true
        try {
            const response = await preferencesApi.updateMyStatus(data)
            status.value = response.data
            return response.data
        } finally {
            loading.value = false
        }
    }

    async function clearStatus() {
        loading.value = true
        try {
            const response = await preferencesApi.clearMyStatus()
            status.value = response.data
        } finally {
            loading.value = false
        }
    }

    async function fetchPreferences() {
        try {
            const response = await preferencesApi.getMyPreferences()
            preferences.value = response.data
        } catch (e) {
            console.error('Failed to fetch preferences:', e)
        }
    }

    async function updatePreferences(data: Record<string, unknown>) {
        loading.value = true
        try {
            const response = await preferencesApi.updateMyPreferences(data as any)
            preferences.value = response.data
            return response.data
        } finally {
            loading.value = false
        }
    }

    async function fetchStatusPresets() {
        try {
            const response = await preferencesApi.listStatusPresets()
            statusPresets.value = response.data
        } catch (e) {
            console.error('Failed to fetch status presets:', e)
        }
    }

    async function applyPreset(preset: StatusPreset) {
        return updateStatus({
            text: preset.text,
            emoji: preset.emoji,
            duration_minutes: preset.duration_minutes || undefined,
        })
    }

    return {
        status,
        preferences,
        statusPresets,
        loading,
        hasStatus,
        statusDisplay,
        fetchStatus,
        updateStatus,
        clearStatus,
        fetchPreferences,
        updatePreferences,
        fetchStatusPresets,
        applyPreset,
    }
})
