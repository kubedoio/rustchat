// Auth Composable - For components that need auth functionality

import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/authStore'
import { authService } from '../services/authService'
import type { LoginCredentials, UpdateStatusRequest } from '../repositories/authRepository'

export function useAuth() {
  const store = useAuthStore()
  const router = useRouter()

  // Reactive state
  const user = computed(() => store.user)
  const isAuthenticated = computed(() => store.isAuthenticated)
  const isAdmin = computed(() => store.isAdmin)
  const error = computed(() => store.error)
  const isInitializing = computed(() => store.isInitializing)

  // Actions
  async function login(credentials: LoginCredentials): Promise<void> {
    await authService.login(credentials)
  }

  async function logout(): Promise<void> {
    await authService.logout()
    await router.push('/login')
  }

  async function updateStatus(request: UpdateStatusRequest): Promise<void> {
    await authService.updateStatus(request)
  }

  async function refreshProfile(): Promise<void> {
    await authService.fetchProfile()
  }

  return {
    // State
    user,
    isAuthenticated,
    isAdmin,
    error,
    isInitializing,

    // Actions
    login,
    logout,
    updateStatus,
    refreshProfile
  }
}

// Composable for route guards
export function useAuthGuard() {
  const store = useAuthStore()

  return {
    requireAuth: async () => {
      if (!store.isAuthenticated) {
        // Try to initialize from storage
        const success = await authService.initialize()
        return success
      }
      return true
    },

    requireAdmin: () => {
      return store.isAdmin
    }
  }
}
