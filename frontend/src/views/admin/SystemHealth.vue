<script setup lang="ts">
import { onMounted } from 'vue';
import { useAdminStore } from '../../features/admin/stores/adminStore';
import { Activity, Database, HardDrive, Wifi, Server, CheckCircle, AlertCircle, Clock, MemoryStick, FolderOpen } from 'lucide-vue-next';

const adminStore = useAdminStore();

onMounted(() => {
    adminStore.fetchHealth();
});

const formatUptime = (seconds: number | undefined) => {
    if (!seconds) return '—';
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return `${days}d ${hours}h ${mins}m`;
};
</script>

<template>
    <div class="space-y-6">
        <div>
            <h1 class="text-2xl font-bold text-gray-900">System Health</h1>
            <p class="text-gray-500 mt-1">Monitor system status and diagnostics</p>
        </div>

        <div v-if="adminStore.health" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <!-- Overall Status -->
            <div class="bg-white rounded-xl p-6 border border-gray-200">
                <div class="flex items-center justify-between mb-4">
                    <h3 class="font-semibold text-gray-900">Overall Status</h3>
                    <span :class="[
                        'px-2 py-1 text-xs font-bold rounded-full uppercase',
                        adminStore.health.status === 'healthy' ? 'bg-green-100 text-green-800' :
                        adminStore.health.status === 'degraded' ? 'bg-yellow-100 text-yellow-800' :
                        'bg-red-100 text-red-800'
                    ]">
                        {{ adminStore.health.status }}
                    </span>
                </div>
                <div class="flex items-center text-gray-500">
                    <Server class="w-5 h-5 mr-2" />
                    <span>Version {{ adminStore.health.version }}</span>
                </div>
            </div>

            <!-- Uptime -->
            <div class="bg-white rounded-xl p-6 border border-gray-200">
                <div class="flex items-center mb-4">
                    <Clock class="w-5 h-5 text-blue-500 mr-2" />
                    <h3 class="font-semibold text-gray-900">Uptime</h3>
                </div>
                <p class="text-2xl font-bold text-gray-900">
                    {{ formatUptime(adminStore.health.uptime_seconds) }}
                </p>
            </div>

            <!-- WebSocket -->
            <div class="bg-white rounded-xl p-6 border border-gray-200">
                <div class="flex items-center mb-4">
                    <Wifi class="w-5 h-5 text-purple-500 mr-2" />
                    <h3 class="font-semibold text-gray-900">WebSocket</h3>
                </div>
                <p class="text-2xl font-bold text-gray-900">
                    {{ adminStore.health.websocket.active_connections }}
                </p>
                <p class="text-sm text-gray-500">Active connections</p>
            </div>

            <!-- Database -->
            <div class="bg-white rounded-xl p-6 border border-gray-200">
                <div class="flex items-center justify-between mb-4">
                    <div class="flex items-center">
                        <Database class="w-5 h-5 text-indigo-500 mr-2" />
                        <h3 class="font-semibold text-gray-900">Database</h3>
                    </div>
                    <CheckCircle v-if="adminStore.health.database.connected" class="w-5 h-5 text-green-500" />
                    <AlertCircle v-else class="w-5 h-5 text-red-500" />
                </div>
                <p class="text-sm text-gray-500">
                    Latency: <span class="font-medium text-gray-900">{{ adminStore.health.database.latency_ms }}ms</span>
                </p>
            </div>

            <!-- Redis -->
            <div class="bg-white rounded-xl p-6 border border-gray-200">
                <div class="flex items-center justify-between mb-4">
                    <div class="flex items-center">
                        <MemoryStick class="w-5 h-5 text-pink-500 mr-2" />
                        <h3 class="font-semibold text-gray-900">Redis</h3>
                    </div>
                    <CheckCircle v-if="adminStore.health.redis.connected" class="w-5 h-5 text-green-500" />
                    <AlertCircle v-else class="w-5 h-5 text-red-500" />
                </div>
                <p class="text-sm text-gray-500">
                    Latency: <span class="font-medium text-gray-900">{{ adminStore.health.redis.latency_ms }}ms</span>
                </p>
            </div>

            <!-- Storage -->
            <div class="bg-white rounded-xl p-6 border border-gray-200">
                <div class="flex items-center justify-between mb-4">
                    <div class="flex items-center">
                        <HardDrive class="w-5 h-5 text-orange-500 mr-2" />
                        <h3 class="font-semibold text-gray-900">Storage</h3>
                    </div>
                    <CheckCircle v-if="adminStore.health.storage.connected" class="w-5 h-5 text-green-500" />
                    <AlertCircle v-else class="w-5 h-5 text-red-500" />
                </div>
                <p class="text-sm text-gray-500">
                    Type: <span class="font-medium text-gray-900">{{ adminStore.health.storage.type }}</span>
                </p>
            </div>

            <!-- Disk -->
            <div class="bg-white rounded-xl p-6 border border-gray-200">
                <div class="flex items-center justify-between mb-4">
                    <div class="flex items-center">
                        <FolderOpen class="w-5 h-5 text-teal-500 mr-2" />
                        <h3 class="font-semibold text-gray-900">Disk</h3>
                    </div>
                    <CheckCircle v-if="adminStore.health.disk.connected" class="w-5 h-5 text-green-500" />
                    <AlertCircle v-else class="w-5 h-5 text-red-500" />
                </div>
                <p class="text-sm text-gray-500">
                    Used: <span class="font-medium text-gray-900">{{ adminStore.health.disk.used_percent }}%</span>
                </p>
                <p class="text-sm text-gray-500">
                    Available: <span class="font-medium text-gray-900">{{ adminStore.health.disk.available_mb }} MB</span>
                </p>
            </div>
        </div>

        <div v-else class="bg-white rounded-xl p-12 text-center border border-gray-200">
            <Activity class="w-12 h-12 mx-auto text-gray-300 animate-pulse mb-4" />
            <p class="text-gray-500">Loading health status...</p>
        </div>
    </div>
</template>
