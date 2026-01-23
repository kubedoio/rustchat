<script setup lang="ts">
import { onMounted } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { Activity, Users, MessageSquare, HardDrive, CheckCircle, AlertCircle, Server } from 'lucide-vue-next';

const adminStore = useAdminStore();

onMounted(() => {
    adminStore.fetchStats();
    adminStore.fetchHealth();
});

const statCards = [
    { key: 'total_users', label: 'Total Users', icon: Users, color: 'bg-blue-500' },
    { key: 'active_users', label: 'Active Users', icon: Activity, color: 'bg-green-500' },
    { key: 'total_teams', label: 'Teams', icon: Server, color: 'bg-purple-500' },
    { key: 'messages_24h', label: 'Messages (24h)', icon: MessageSquare, color: 'bg-indigo-500' },
];
</script>

<template>
    <div class="space-y-8">
        <!-- Header -->
        <div>
            <h1 class="text-2xl font-bold text-gray-900 dark:text-white">System Overview</h1>
            <p class="text-gray-500 dark:text-gray-400 mt-1">Monitor your RustChat instance health and usage</p>
        </div>

        <!-- Stats Grid -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <div 
                v-for="stat in statCards" 
                :key="stat.key"
                class="bg-white dark:bg-slate-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-slate-700"
            >
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">
                            {{ stat.label }}
                        </p>
                        <p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
                            {{ adminStore.stats?.[stat.key as keyof typeof adminStore.stats] ?? '—' }}
                        </p>
                    </div>
                    <div :class="[stat.color, 'w-12 h-12 rounded-lg flex items-center justify-center']">
                        <component :is="stat.icon" class="w-6 h-6 text-white" />
                    </div>
                </div>
            </div>
        </div>

        <!-- Health Status -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-slate-700">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">System Health</h2>
            
            <div v-if="adminStore.health" class="grid grid-cols-1 md:grid-cols-3 gap-6">
                <!-- Database -->
                <div class="flex items-center space-x-3">
                    <div :class="[
                        'w-10 h-10 rounded-full flex items-center justify-center',
                        adminStore.health.database.connected ? 'bg-green-100 dark:bg-green-900/30' : 'bg-red-100 dark:bg-red-900/30'
                    ]">
                        <CheckCircle v-if="adminStore.health.database.connected" class="w-5 h-5 text-green-600 dark:text-green-400" />
                        <AlertCircle v-else class="w-5 h-5 text-red-600 dark:text-red-400" />
                    </div>
                    <div>
                        <p class="font-medium text-gray-900 dark:text-white">Database</p>
                        <p class="text-sm text-gray-500">
                            {{ adminStore.health.database.connected ? `${adminStore.health.database.latency_ms}ms latency` : 'Disconnected' }}
                        </p>
                    </div>
                </div>

                <!-- Storage -->
                <div class="flex items-center space-x-3">
                    <div :class="[
                        'w-10 h-10 rounded-full flex items-center justify-center',
                        adminStore.health.storage.connected ? 'bg-green-100 dark:bg-green-900/30' : 'bg-red-100 dark:bg-red-900/30'
                    ]">
                        <HardDrive :class="[
                            'w-5 h-5',
                            adminStore.health.storage.connected ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'
                        ]" />
                    </div>
                    <div>
                        <p class="font-medium text-gray-900 dark:text-white">Storage</p>
                        <p class="text-sm text-gray-500">{{ adminStore.health.storage.type }}</p>
                    </div>
                </div>

                <!-- WebSocket -->
                <div class="flex items-center space-x-3">
                    <div class="w-10 h-10 rounded-full bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
                        <Activity class="w-5 h-5 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div>
                        <p class="font-medium text-gray-900 dark:text-white">WebSocket</p>
                        <p class="text-sm text-gray-500">
                            {{ adminStore.health.websocket.active_connections }} connections
                        </p>
                    </div>
                </div>
            </div>

            <div v-else class="text-center py-8 text-gray-500">
                <p>Loading health status...</p>
            </div>
        </div>

        <!-- Instance Info -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-slate-700">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Instance Information</h2>
            <dl class="grid grid-cols-2 gap-4 text-sm">
                <div>
                    <dt class="text-gray-500 dark:text-gray-400">Version</dt>
                    <dd class="font-medium text-gray-900 dark:text-white">{{ adminStore.health?.version || 'v2.0.0' }}</dd>
                </div>
                <div>
                    <dt class="text-gray-500 dark:text-gray-400">Uptime</dt>
                    <dd class="font-medium text-gray-900 dark:text-white">
                        {{ adminStore.health?.uptime_seconds ? Math.floor(adminStore.health.uptime_seconds / 3600) + 'h' : '—' }}
                    </dd>
                </div>
            </dl>
        </div>
    </div>
</template>
