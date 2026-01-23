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
            // @ts-ignore
            const { data } = await client.put('/users/me/status', status)
            if (user.value) {
                if (data.presence) user.value.presence = data.presence
                // Merge other status fields if needed, or re-fetch me
                user.value.status_text = data.text
                user.value.status_emoji = data.emoji
                user.value.status_expires_at = data.expires_at
            }
        } catch (e) {
            console.error('Failed to update status', e)
        }
    }

    return { token, user, isAuthenticated, login, logout, fetchMe, updateStatus }
})
