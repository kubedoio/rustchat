// Admin Service - Business logic for admin functions

import { adminRepository, type AuditLogQuery } from '../repositories/adminRepository'
import type { AdminUser } from '../../../api/admin'
import { useAdminStore } from '../stores/adminStore'
import { AppError } from '../../../core/errors/AppError'

class AdminService {
  private get store() {
    return useAdminStore()
  }

  // Config
  async loadConfig(): Promise<void> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      const config = await adminRepository.getConfig()
      this.store.setConfig(config)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to load config'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  async updateConfig(category: string, data: unknown): Promise<void> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      await adminRepository.updateConfig(category, data)
      await this.loadConfig()
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to update config'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Users
  async loadUsers(params?: { page?: number; perPage?: number; search?: string }): Promise<void> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      const result = await adminRepository.listUsers(params)
      this.store.setUsers(result.users)
      this.store.setUsersTotal(result.total)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to load users'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  async createUser(data: {
    email: string
    username: string
    password: string
    role?: string
    displayName?: string
  }): Promise<AdminUser> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      const user = await adminRepository.createUser(data)
      this.store.addUser(user)
      return user
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to create user'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  async updateUser(
    id: string,
    data: { email?: string; username?: string; role?: string; isActive?: boolean }
  ): Promise<AdminUser> {
    try {
      const user = await adminRepository.updateUser(id, data)
      this.store.updateUserInStore(user)
      return user
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to update user'
      )
      throw error
    }
  }

  async deactivateUser(id: string): Promise<void> {
    try {
      await adminRepository.deactivateUser(id)
      this.store.updateUserStatus(id, false)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to deactivate user'
      )
      throw error
    }
  }

  async reactivateUser(id: string): Promise<void> {
    try {
      await adminRepository.reactivateUser(id)
      this.store.updateUserStatus(id, true)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to reactivate user'
      )
      throw error
    }
  }

  // Audit logs
  async loadAuditLogs(params?: AuditLogQuery): Promise<void> {
    this.store.setLoading(true)
    try {
      const logs = await adminRepository.listAuditLogs(params)
      this.store.setAuditLogs(logs)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to load audit logs'
      )
    } finally {
      this.store.setLoading(false)
    }
  }

  // Stats
  async loadStats(): Promise<void> {
    try {
      const stats = await adminRepository.getStats()
      this.store.setStats(stats)
    } catch (error) {
      console.warn('Stats not available:', error)
    }
  }

  // Health
  async loadHealth(): Promise<void> {
    try {
      const health = await adminRepository.getHealth()
      this.store.setHealth(health)
    } catch (error) {
      console.warn('Health endpoint not available:', error)
    }
  }
}

export const adminService = new AdminService()
