import { log } from '@/utils/log'
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { useSessionStorage } from '@vueuse/core'
import client from '../../../api/client'
import type {
  AuthUser,
  LoginCredentials,
  AuthResponse,
  AuthPolicy,
  StatusUpdateInput,
  StatusSnapshot,
} from '../../../core/entities/Auth'
import type { PresenceStatus } from '../../../core/entities/User'
import { clearUserSummaryCache } from '../../../composables/useUserSummary'
import { clearChannelPermissionCache } from '../../permissions/capabilities'
import { usePresenceStore } from '../../presence'
import {
  clearStatusExpiryTimer as clearSharedStatusExpiryTimer,
  scheduleStatusExpiry,
} from '../../presence/statusExpiry'
import { useThemeStore } from '../../theme/stores/themeStore'
import { useMessageStore } from '@/features/messages/stores/messageStore'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { useUnreadStore } from '@/features/unreads/stores/unreadStore'
import { useTeamStore } from '@/features/teams/stores/teamStore'
import { useChannelPreferencesStore } from '@/features/channels/stores/channelPreferencesStore'
import { useUIStore } from '../../ui/stores/uiStore'
import { useCallsStore } from '@/stores/calls'

type LogoutReason = 'manual' | 'expired' | 'unauthorized'

function parseJwtExpiryMs(tokenValue: string): number | null {
  if (!tokenValue) {
    return null
  }

  const parts = tokenValue.split('.')
  if (parts.length < 2) {
    return null
  }

  const payloadPart = parts[1]
  if (!payloadPart) {
    return null
  }

  const normalized = payloadPart.replace(/-/g, '+').replace(/_/g, '/')
  const paddingLength = (4 - (normalized.length % 4)) % 4
  const padded = normalized + '='.repeat(paddingLength)

  try {
    const payload = JSON.parse(atob(padded)) as { exp?: unknown }
    const expSeconds = Number(payload.exp)
    if (!Number.isFinite(expSeconds) || expSeconds <= 0) {
      return null
    }
    return expSeconds * 1000
  } catch {
    return null
  }
}

export const useAuthStore = defineStore('authStore', () => {
  const token = useSessionStorage('auth_token', '')
  const user = ref<AuthUser | null>(null)
  const authPolicy = ref<AuthPolicy | null>(null)
  const error = ref<string | null>(null)
  const isInitializing = ref(false)
  let tokenExpiryTimer: ReturnType<typeof setTimeout> | null = null
  const statusExpiryTimers = new Map<string, ReturnType<typeof setTimeout>>()
  let isLoggingOut = false

  function clearTokenExpiryTimer() {
    if (tokenExpiryTimer) {
      clearTimeout(tokenExpiryTimer)
      tokenExpiryTimer = null
    }
  }

  function clearSelfStatusExpiryTimer() {
    clearSharedStatusExpiryTimer(statusExpiryTimers, 'self')
  }

  function syncUserStatusSnapshot(snapshot: StatusSnapshot) {
    if (!user.value) {
      return
    }

    const nextPresence = (snapshot.status ?? snapshot.presence) as PresenceStatus | undefined
    if (nextPresence) {
      user.value.presence = nextPresence
    }

    const nextExpiresAt = (snapshot.expiresAt ?? snapshot.expires_at ?? null) as
      | string
      | number
      | null
    user.value.status_text = snapshot.text ?? null
    user.value.status_emoji = snapshot.emoji ?? null
    user.value.status_expires_at = nextExpiresAt

    if (snapshot.text || snapshot.emoji) {
      user.value.custom_status = {
        text: (snapshot.text ?? null) as string | null,
        emoji: (snapshot.emoji ?? null) as string | null,
        expires_at: nextExpiresAt,
      }
    } else {
      user.value.custom_status = null
    }

    clearSelfStatusExpiryTimer()
    const expiryMs = scheduleStatusExpiry(statusExpiryTimers, 'self', nextExpiresAt, () => {
      if (!user.value) {
        return
      }
      user.value.status_text = null
      user.value.status_emoji = null
      user.value.status_expires_at = null
      user.value.custom_status = null
    })
    if (!expiryMs) {
      return
    }
  }

  async function clearSessionState() {
    useMessageStore().resetSessionState()
    useChannelStore().clearChannels()
    useUnreadStore().clearAllState()
    usePresenceStore().clear()
    clearChannelPermissionCache()
    clearUserSummaryCache()
    useTeamStore().clear()
    useChannelPreferencesStore().clearState()

    const uiStore = useUIStore()
    uiStore.closeVideoCall()
    uiStore.closeRhs()
    uiStore.closeSettings()
    uiStore.closeLhs()

    useCallsStore().resetSessionState()
  }

  function scheduleTokenExpiryLogout() {
    clearTokenExpiryTimer()

    const expiryMs = parseJwtExpiryMs(token.value)
    if (!expiryMs) {
      return
    }

    const remainingMs = expiryMs - Date.now()
    if (remainingMs <= 0) {
      void logout('expired')
      return
    }

    tokenExpiryTimer = setTimeout(() => {
      void logout('expired')
    }, remainingMs)
  }

  const isAuthenticated = computed(() => !!token.value && !!user.value)

  const currentUserId = computed((): string | null => {
    return user.value?.id || null
  })

  const isAdmin = computed(() => {
    return user.value?.role === 'system_admin' || user.value?.role === 'org_admin'
  })

  const isSystemAdmin = computed(() => {
    return user.value?.role === 'system_admin'
  })

  async function login(credentials: LoginCredentials) {
    const { data } = await client.post<AuthResponse>('/auth/login', credentials)
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
      syncUserStatusSnapshot({
        status: data.presence,
        text: data.status_text,
        emoji: data.status_emoji,
        expires_at: data.status_expires_at,
      })
      const themeStore = useThemeStore()
      await themeStore.syncFromServer()
    } catch (e) {
      await logout('unauthorized')
    }
  }

  async function logout(_reason: LogoutReason = 'manual') {
    if (isLoggingOut) {
      return
    }
    isLoggingOut = true
    clearTokenExpiryTimer()
    clearSelfStatusExpiryTimer()
    try {
      // Ask the backend to clear the HttpOnly MMAUTHTOKEN cookie so the
      // browser stops sending it on future requests.
      try {
        await client.post('/auth/logout')
      } catch (e) {
        log.error('Failed to notify server of logout', e)
      }

      token.value = ''
      user.value = null
      await clearSessionState()

      if (window.location.pathname !== '/login') {
        window.location.replace('/login')
      }
    } finally {
      isLoggingOut = false
    }
  }

  async function updateStatus(status: StatusUpdateInput) {
    if (!token.value) return
    try {
      const payload = { ...status }
      if (payload.presence && !payload.status) {
        payload.status = payload.presence
      }
      delete payload.presence

      const { data } = await client.put('/users/me/status', payload)
      syncUserStatusSnapshot({
        status: data.status,
        presence: data.presence,
        text: data.text,
        emoji: data.emoji,
        expiresAt: data.expires_at,
      })
    } catch (e) {
      log.error('Failed to update status', e)
    }
  }

  async function getAuthPolicy() {
    try {
      const { data } = await client.get('/auth/policy')
      authPolicy.value = data
      return data
    } catch (e) {
      log.error('Failed to fetch auth policy', e)
    }
  }

  function setToken(value: string) {
    token.value = value
  }

  function setUser(value: AuthUser | null) {
    user.value = value
  }

  function setAuthPolicy(value: AuthPolicy | null) {
    authPolicy.value = value
  }

  function setError(err: string | null) {
    error.value = err
  }

  function setInitializing(value: boolean) {
    isInitializing.value = value
  }

  function clearError() {
    error.value = null
  }

  function clear() {
    token.value = ''
    user.value = null
    error.value = null
  }

  watch(
    () => token.value,
    () => {
      if (!token.value) {
        clearTokenExpiryTimer()
        clearSelfStatusExpiryTimer()
        return
      }
      scheduleTokenExpiryLogout()
    },
    { immediate: true }
  )

  return {
    // State
    token,
    user,
    authPolicy,
    error,
    isInitializing,

    // Getters
    isAuthenticated,
    currentUserId,
    isAdmin,
    isSystemAdmin,

    // Legacy actions
    login,
    logout,
    fetchMe,
    updateStatus,
    getAuthPolicy,
    syncUserStatusSnapshot,

    // Actions
    setToken,
    setUser,
    setAuthPolicy,
    setError,
    setInitializing,
    clearError,
    clear,
  }
})
