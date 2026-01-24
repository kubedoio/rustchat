import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useStorage } from '@vueuse/core'
import client from '../api/client'
import { useRouter } from 'vue-router'

export const useAuthStore = defineStore('auth', () => {
    const token = useStorage('auth_token', '')
    const user = ref<any>(null)
    const router = useRouter()

    const isAuthenticated = computed(() => !!token.value)

    async function login(credentials: any) {
        const { data } = await client.post('/auth/login', credentials)
        token.value = data.token
        user.value = data.user
        // Fetch full profile
        await fetchMe()
    }

    async function fetchMe() {
        if (!token.value) return
        try {
            const { data } = await client.get('/auth/me')
            // Map custom_status fields for easier access
            if (data.custom_status) {
                data.status_text = data.custom_status.text
                data.status_emoji = data.custom_status.emoji
                data.status_expires_at = data.custom_status.expires_at
            }
            user.value = data
        } catch (e) {
            logout()
        }
    }

    async function logout() {
        token.value = ''
        user.value = null
        router.push('/login')
    }

    async function updateStatus(status: { presence?: string, text?: string, emoji?: string, duration_minutes?: number }) {
        if (!token.value) return
        try {
            const { data } = await client.put('/users/me/status', status)
            if (user.value) {
                if (data.presence) user.value.presence = data.presence
                user.value.status_text = data.text
                user.value.status_emoji = data.emoji
                user.value.status_expires_at = data.expires_at
                
                // Also update the nested object to stay in sync
                user.value.custom_status = {
                    text: data.text,
                    emoji: data.emoji,
                    expires_at: data.expires_at
                }
            }
        } catch (e) {
            console.error('Failed to update status', e)
        }
    }

    const authPolicy = ref<any>(null)

    async function getAuthPolicy() {
        try {
            const { data } = await client.get('/auth/policy')
            authPolicy.value = data
            return data
        } catch (e) {
            console.error('Failed to fetch auth policy', e)
        }
    }

    return { token, user, isAuthenticated, login, logout, fetchMe, updateStatus, authPolicy, getAuthPolicy }
})
