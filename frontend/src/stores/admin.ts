import { defineStore } from 'pinia';
import { ref } from 'vue';
import adminApi, {
    type ServerConfig,
    type AdminUser,
    type AuditLog,
    type SystemStats,
    type HealthStatus
} from '../api/admin';

export const useAdminStore = defineStore('admin', () => {
    // State
    const config = ref<ServerConfig | null>(null);
    const users = ref<AdminUser[]>([]);
    const usersTotal = ref(0);
    const auditLogs = ref<AuditLog[]>([]);
    const stats = ref<SystemStats | null>(null);
    const health = ref<HealthStatus | null>(null);
    const loading = ref(false);
    const error = ref<string | null>(null);

    // Actions
    async function fetchConfig() {
        loading.value = true;
        error.value = null;
        try {
            const response = await adminApi.getConfig();
            config.value = response.data;
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to load config';
        } finally {
            loading.value = false;
        }
    }

    async function updateConfig(category: string, data: any) {
        loading.value = true;
        error.value = null;
        try {
            await adminApi.updateConfig(category, data);
            await fetchConfig(); // Refresh
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to update config';
            throw e;
        } finally {
            loading.value = false;
        }
    }

    async function fetchUsers(params?: Parameters<typeof adminApi.listUsers>[0]) {
        loading.value = true;
        error.value = null;
        try {
            const response = await adminApi.listUsers(params);
            users.value = response.data.users;
            usersTotal.value = response.data.total;
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to load users';
        } finally {
            loading.value = false;
        }
    }

    async function createUser(data: Parameters<typeof adminApi.createUser>[0]) {
        loading.value = true;
        error.value = null;
        try {
            const response = await adminApi.createUser(data);
            users.value.unshift(response.data);
            usersTotal.value++;
            return response.data;
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to create user';
            throw e;
        } finally {
            loading.value = false;
        }
    }

    async function updateUser(id: string, data: Parameters<typeof adminApi.updateUser>[1]) {
        try {
            const response = await adminApi.updateUser(id, data);
            const idx = users.value.findIndex(u => u.id === id);
            if (idx !== -1) users.value[idx] = response.data;
            return response.data;
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to update user';
            throw e;
        }
    }

    async function deactivateUser(id: string) {
        try {
            await adminApi.deactivateUser(id);
            const user = users.value.find(u => u.id === id);
            if (user) user.is_active = false;
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to deactivate user';
            throw e;
        }
    }

    async function reactivateUser(id: string) {
        try {
            await adminApi.reactivateUser(id);
            const user = users.value.find(u => u.id === id);
            if (user) user.is_active = true;
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to reactivate user';
            throw e;
        }
    }

    async function fetchAuditLogs(params?: Parameters<typeof adminApi.listAuditLogs>[0]) {
        loading.value = true;
        try {
            const response = await adminApi.listAuditLogs(params);
            auditLogs.value = response.data;
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to load audit logs';
        } finally {
            loading.value = false;
        }
    }

    async function fetchStats() {
        try {
            const response = await adminApi.getStats();
            stats.value = response.data;
        } catch (e: any) {
            // Stats endpoint might not exist yet
            console.warn('Stats not available:', e.message);
        }
    }

    async function fetchHealth() {
        try {
            const response = await adminApi.getHealth();
            health.value = response.data;
        } catch (e: any) {
            console.warn('Health endpoint not available:', e.message);
        }
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
        fetchAuditLogs,
        fetchStats,
        fetchHealth,
    };
});
