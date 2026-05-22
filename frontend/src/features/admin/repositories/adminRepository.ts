// Admin Repository - Data access for admin functions

import { adminApi } from '../../../api/admin'
import type {
  ServerConfig,
  AdminUser,
  AuditLog,
  SystemStats,
  HealthStatus,
} from '../../../api/admin'
import { withRetry } from '../../../core/services/retry'

export interface UserListResponse {
  users: AdminUser[]
  total: number
}

export interface AuditLogQuery {
  page?: number
  perPage?: number
  userId?: string
  action?: string
}

export const adminRepository = {
  // Config
  async getConfig(): Promise<ServerConfig> {
    return withRetry(async () => {
      const response = await adminApi.getConfig()
      return response.data
    })
  },

  async updateConfig(category: string, data: unknown): Promise<void> {
    await withRetry(() => adminApi.updateConfig(category, data as Record<string, unknown>))
  },

  // Users
  async listUsers(params?: {
    page?: number
    perPage?: number
    search?: string
  }): Promise<UserListResponse> {
    return withRetry(async () => {
      const response = await adminApi.listUsers(params)
      return response.data
    })
  },

  async createUser(data: {
    email: string
    username: string
    password: string
    role?: string
    displayName?: string
  }): Promise<AdminUser> {
    return withRetry(async () => {
      const response = await adminApi.createUser({
        email: data.email,
        username: data.username,
        password: data.password,
        role: data.role,
        display_name: data.displayName,
      })
      return response.data
    })
  },

  async updateUser(
    id: string,
    data: {
      email?: string
      username?: string
      role?: string
      isActive?: boolean
    }
  ): Promise<AdminUser> {
    return withRetry(async () => {
      const response = await adminApi.updateUser(id, data)
      return response.data
    })
  },

  async deactivateUser(id: string): Promise<void> {
    await withRetry(() => adminApi.deactivateUser(id))
  },

  async reactivateUser(id: string): Promise<void> {
    await withRetry(() => adminApi.reactivateUser(id))
  },

  // Audit logs
  async listAuditLogs(params?: AuditLogQuery): Promise<AuditLog[]> {
    return withRetry(async () => {
      const response = await adminApi.listAuditLogs(params)
      return response.data
    })
  },

  // Stats
  async getStats(): Promise<SystemStats> {
    return withRetry(async () => {
      const response = await adminApi.getStats()
      return response.data
    })
  },

  // Health
  async getHealth(): Promise<HealthStatus> {
    return withRetry(async () => {
      const response = await adminApi.getHealth()
      return response.data
    })
  },
}
