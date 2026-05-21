// Auth Service - Business logic for authentication
// Handles login flow, session management, and status updates

import { log } from '@/utils/log';
import { authRepository, type LoginCredentials, type UpdateStatusRequest } from '../repositories/authRepository'
import type { User } from '../../../core/entities/User'
import { useAuthStore } from '../stores/authStore'
import { AppError } from '../../../core/errors/AppError'

// Global auth state for non-component access
let globalAuthToken: string | null = null

export function getGlobalAuthToken(): string | null {
  // First check memory
  if (globalAuthToken) return globalAuthToken
  
  // Then check localStorage
  return authRepository.getStoredToken()
}

class AuthService {
  private get store() {
    return useAuthStore()
  }

  // Initialize auth state from storage (call on app startup)
  async initialize(): Promise<boolean> {
    const token = authRepository.getStoredToken()
    
    if (!token) {
      return false
    }

    // Set global token for API client
    globalAuthToken = token
    
    // Set cookie for media access
    authRepository.setAuthCookie(token)

    try {
      // Fetch user profile
      const user = await authRepository.fetchMe()
      this.store.setUser(user)
      this.store.setToken(token)
      
      return true
    } catch (error) {
      // Token is invalid, clear everything
      this.logout()
      return false
    }
  }

  // Login with credentials
  async login(credentials: LoginCredentials): Promise<User> {
    try {
      const { token, user } = await authRepository.login(credentials)
      
      // Store token
      authRepository.setStoredToken(token)
      globalAuthToken = token
      
      // Set cookie for media access
      authRepository.setAuthCookie(token)
      
      // Update store
      this.store.setToken(token)
      this.store.setUser(user)
      
      // Fetch full profile
      await this.fetchProfile()
      
      return this.store.user!
    } catch (error) {
      this.store.setError(
        error instanceof AppError 
          ? error.message 
          : 'Login failed. Please check your credentials.'
      )
      throw error
    }
  }

  // Fetch current user profile
  async fetchProfile(): Promise<User | null> {
    if (!this.store.token) return null

    // Sync cookie (token may be in localStorage but cookie cleared)
    authRepository.setAuthCookie(this.store.token)

    try {
      const user = await authRepository.fetchMe()
      this.store.setUser(user)
      return user
    } catch (error) {
      // Session expired or invalid
      await this.logout()
      return null
    }
  }

  // Logout
  async logout(): Promise<void> {
    // Clear server session (optional)
    await authRepository.logout()
    
    // Clear storage
    authRepository.clearStoredToken()
    authRepository.clearAuthCookie()
    globalAuthToken = null
    
    // Clear store
    this.store.clear()
  }

  // Update user status/presence
  async updateStatus(request: UpdateStatusRequest): Promise<void> {
    if (!this.store.isAuthenticated) {
      throw new AppError('Not authenticated', 'AUTH_REQUIRED')
    }

    try {
      const result = await authRepository.updateStatus(request)
      
      // Update local user state
      const user = this.store.user
      if (user) {
        const updatedUser: User = {
          ...user,
          ...(result.presence && { presence: result.presence as any }),
          ...(result.text !== undefined || result.emoji !== undefined ? {
            customStatus: {
              emoji: result.emoji || '',
              text: result.text || '',
              expiresAt: result.expiresAt ? new Date(result.expiresAt) : undefined
            }
          } : {})
        }
        this.store.setUser(updatedUser)
      }
    } catch (error) {
      log.error('Failed to update status', error)
      throw error
    }
  }

  // Load auth policy (signup settings)
  async loadAuthPolicy(): Promise<void> {
    try {
      const policy = await authRepository.getAuthPolicy()
      this.store.setAuthPolicy(policy)
    } catch (error) {
      log.error('Failed to load auth policy', error)
    }
  }

  // Handle 401 errors (called by API client interceptor)
  handleUnauthorized(): void {
    if (this.store.isAuthenticated) {
      void this.logout()
    }
  }

  // Check if token exists (for initial route guards)
  hasToken(): boolean {
    return !!authRepository.getStoredToken()
  }
}

export const authService = new AuthService()
