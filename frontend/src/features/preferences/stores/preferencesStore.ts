// Preferences Store - Backwards-compatible state management for user preferences

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { preferencesApi, type StatusPreset } from '../../../api/preferences'

export const usePreferencesStore = defineStore('preferencesStore', () => {
    // State
    const status = ref<any | null>(null)
    const preferences = ref<any | null>(null)
    const statusPresets = ref<any[]>([])
    const loading = ref(false)
    const error = ref<string | null>(null)

    // Getters
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

    // Legacy async actions
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

    // Feature-compatible setters
    function setStatus(value: any | null) {
        status.value = value
    }

    function setPreferences(value: any | null) {
        preferences.value = value
    }

    function setStatusPresets(value: any[]) {
        statusPresets.value = value
    }

    function setLoading(value: boolean) {
        loading.value = value
    }

    function setError(err: string | null) {
        error.value = err
    }

    function clearError() {
        error.value = null
    }

    function clear() {
        status.value = null
        preferences.value = null
        statusPresets.value = []
        error.value = null
    }

    return {
        // State
        status,
        preferences,
        statusPresets,
        loading,
        error,
        // Getters
        hasStatus,
        statusDisplay,
        // Actions
        fetchStatus,
        updateStatus,
        clearStatus,
        fetchPreferences,
        updatePreferences,
        fetchStatusPresets,
        applyPreset,
        setStatus,
        setPreferences,
        setStatusPresets,
        setLoading,
        setError,
        clearError,
        clear
    }
})
