<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { FileText, Search, User, Settings, Shield, Database } from 'lucide-vue-next';

const adminStore = useAdminStore();
const actionFilter = ref('');

onMounted(() => {
    adminStore.fetchAuditLogs();
});

const actionIcons: Record<string, any> = {
    'user.create': User,
    'user.update': User,
    'user.deactivate': User,
    'config.update': Settings,
    'sso.update': Shield,
    'retention.create': Database,
};

const formatDate = (date: string) => {
    return new Date(date).toLocaleString();
};
</script>

<template>
    <div class="space-y-6">
        <div>
            <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Audit Logs</h1>
            <p class="text-gray-500 dark:text-gray-400 mt-1">View administrative actions and changes</p>
        </div>

        <!-- Filters -->
        <div class="flex items-center space-x-4">
            <div class="relative flex-1 max-w-md">
                <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
                <input 
                    v-model="actionFilter"
                    type="text"
                    placeholder="Filter by action..."
                    class="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800"
                />
            </div>
        </div>

        <!-- Logs Table -->
        <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 overflow-hidden">
            <table class="min-w-full divide-y divide-gray-200 dark:divide-slate-700">
                <thead class="bg-gray-50 dark:bg-slate-900">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Action</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Target</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Actor</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">IP</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Date</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-200 dark:divide-slate-700">
                    <tr v-for="log in adminStore.auditLogs" :key="log.id" class="hover:bg-gray-50 dark:hover:bg-slate-700/50">
                        <td class="px-6 py-4 whitespace-nowrap">
                            <div class="flex items-center">
                                <component :is="actionIcons[log.action] || FileText" class="w-4 h-4 text-gray-400 mr-2" />
                                <span class="text-sm font-medium text-gray-900 dark:text-white">{{ log.action }}</span>
                            </div>
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                            {{ log.target_type }}{{ log.target_id ? `:${log.target_id.substring(0, 8)}` : '' }}
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                            {{ log.actor_user_id?.substring(0, 8) || 'System' }}
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                            {{ log.actor_ip || '—' }}
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                            {{ formatDate(log.created_at) }}
                        </td>
                    </tr>
                    <tr v-if="adminStore.auditLogs.length === 0">
                        <td colspan="5" class="px-6 py-12 text-center text-gray-500">
                            <FileText class="w-12 h-12 mx-auto mb-4 text-gray-300" />
                            <p>No audit logs found</p>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
</template>
