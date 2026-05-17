// Admin Store - Backwards-compatible state management for admin

import { defineStore } from 'pinia'
import { ref } from 'vue'
import adminApi, {
    type ServerConfig,
    type AdminUser,
    type AuditLog,
    type SystemStats,
    type HealthStatus
} from '../../../api/admin'
import { getApiErrorMessage, getErrorMessage } from '../../../core/errors/errorUtils'

export const useAdminStore = defineStore('adminStore', () => {
    // State
    const config = ref<ServerConfig | null>(null)
    const users = ref<AdminUser[]>([])
    const usersTotal = ref(0)
    const auditLogs = ref<AuditLog[]>([])
    const stats = ref<SystemStats | null>(null)
    const health = ref<HealthStatus | null>(null)
    const loading = ref(false)
    const error = ref<string | null>(null)

    // Actions
    async function fetchConfig() {
        loading.value = true
        error.value = null
        try {
            const response = await adminApi.getConfig()
            config.value = response.data
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to load config'
        } finally {
            loading.value = false
        }
    }

    async function updateConfig(category: string, data: Record<string, unknown>) {
        loading.value = true
        error.value = null
        try {
            await adminApi.updateConfig(category, data)
            await fetchConfig() // Refresh
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to update config'
            throw e
        } finally {
            loading.value = false
        }
    }

    async function fetchUsers(params?: Parameters<typeof adminApi.listUsers>[0]) {
        loading.value = true
        error.value = null
        try {
            const response = await adminApi.listUsers(params)
            users.value = response.data.users
            usersTotal.value = response.data.total
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to load users'
        } finally {
            loading.value = false
        }
    }

    async function createUser(data: Parameters<typeof adminApi.createUser>[0]) {
        loading.value = true
        error.value = null
        try {
            const response = await adminApi.createUser(data)
            users.value.unshift(response.data)
            usersTotal.value++
            return response.data
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to create user'
            throw e
        } finally {
            loading.value = false
        }
    }

    async function updateUser(id: string, data: Parameters<typeof adminApi.updateUser>[1]) {
        try {
            const response = await adminApi.updateUser(id, data)
            const idx = users.value.findIndex(u => u.id === id)
            if (idx !== -1) users.value[idx] = response.data
            return response.data
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to update user'
            throw e
        }
    }

    async function deactivateUser(id: string) {
        try {
            await adminApi.deactivateUser(id)
            const user = users.value.find(u => u.id === id)
            if (user) user.is_active = false
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to deactivate user'
            throw e
        }
    }

    async function reactivateUser(id: string) {
        try {
            await adminApi.reactivateUser(id)
            const user = users.value.find(u => u.id === id)
            if (user) user.is_active = true
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to reactivate user'
            throw e
        }
    }

    async function deleteUser(id: string, data: { confirm: string; reason?: string }) {
        try {
            const response = await adminApi.deleteUser(id, data)
            const user = users.value.find(u => u.id === id)
            if (user) {
                user.is_active = false
                user.deleted_at = new Date().toISOString()
                user.delete_reason = data.reason ?? null
            }
            return response.data
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to delete user'
            throw e
        }
    }

    async function wipeUser(id: string) {
        try {
            const response = await adminApi.wipeUser(id)
            // Remove user from the list after successful wipe
            users.value = users.value.filter(u => u.id !== id)
            usersTotal.value--
            return response.data
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to wipe user'
            throw e
        }
    }

    async function fetchAuditLogs(params?: Parameters<typeof adminApi.listAuditLogs>[0]) {
        loading.value = true
        try {
            const response = await adminApi.listAuditLogs(params)
            auditLogs.value = response.data
        } catch (e: unknown) {
            error.value = getApiErrorMessage(e) || 'Failed to load audit logs'
        } finally {
            loading.value = false
        }
    }

    async function fetchStats() {
        try {
            const response = await adminApi.getStats()
            stats.value = response.data
        } catch (e: unknown) {
            // Stats endpoint might not exist yet
            console.warn('Stats not available:', getErrorMessage(e))
        }
    }

    async function fetchHealth() {
        try {
            const response = await adminApi.getHealth()
            health.value = response.data
        } catch (e: unknown) {
            console.warn('Health endpoint not available:', getErrorMessage(e))
        }
    }

    // Feature-compatible setters
    function setConfig(value: ServerConfig | null) {
        config.value = value
    }

    function setUsers(value: AdminUser[]) {
        users.value = value
    }

    function addUser(user: AdminUser) {
        users.value.unshift(user)
        usersTotal.value++
    }

    function updateUserInStore(user: AdminUser) {
        const index = users.value.findIndex(u => u.id === user.id)
        if (index !== -1) {
            users.value[index] = user
        }
    }

    function updateUserStatus(id: string, isActive: boolean) {
        const user = users.value.find(u => u.id === id)
        if (user) {
            user.is_active = isActive
        }
    }

    function setUsersTotal(value: number) {
        usersTotal.value = value
    }

    function setAuditLogs(value: AuditLog[]) {
        auditLogs.value = value
    }

    function setStats(value: SystemStats | null) {
        stats.value = value
    }

    function setHealth(value: HealthStatus | null) {
        health.value = value
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

    return {
        // State
        config,
        users,
        usersTotal,
        auditLogs,
        stats,
        health,
        loading,
        error,
        // Actions
        fetchConfig,
        updateConfig,
        fetchUsers,
        createUser,
        updateUser,
        deactivateUser,
        reactivateUser,
        deleteUser,
        wipeUser,
        fetchAuditLogs,
        fetchStats,
        fetchHealth,
        setConfig,
        setUsers,
        addUser,
        updateUserInStore,
        updateUserStatus,
        setUsersTotal,
        setAuditLogs,
        setStats,
        setHealth,
        setLoading,
        setError,
        clearError
    }
})
