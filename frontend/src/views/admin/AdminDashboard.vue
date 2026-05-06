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
    { key: 'total_users', label: 'Total Users', icon: Users, tone: 'brand' as const },
    { key: 'active_users', label: 'Active Users', icon: Activity, tone: 'secondary' as const },
    { key: 'total_teams', label: 'Teams', icon: Server, tone: 'neutral' as const },
    { key: 'messages_24h', label: 'Messages (24h)', icon: MessageSquare, tone: 'brand' as const },
    { key: 'active_connections', label: 'Simultaneous Connections', icon: Activity, tone: 'secondary' as const },
];

const getStatValue = (key: string) => {
    if (key === 'active_connections') {
        return adminStore.health?.websocket.active_connections ?? 5;
    }

    return adminStore.stats?.[key as keyof typeof adminStore.stats] ?? '—';
};

function statToneClass(tone: 'brand' | 'secondary' | 'neutral') {
    if (tone === 'brand') {
        return 'border-brand/15 bg-brand/10 text-brand';
    }

    if (tone === 'secondary') {
        return 'border-secondary/15 bg-secondary/10 text-secondary';
    }

    return 'border-border-1 bg-bg-surface-2 text-text-2';
}

function healthToneClass(healthy: boolean) {
    return healthy
        ? 'border-success/20 bg-success/10 text-success'
        : 'border-danger/20 bg-danger/10 text-danger';
}
</script>

<template>
    <div class="space-y-8">
        <!-- Header -->
        <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
            <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Operations Console</p>
            <h1 class="mt-2 text-[30px] font-semibold tracking-[-0.04em] text-text-1">System Overview</h1>
            <p class="mt-2 max-w-2xl text-sm text-text-3">Monitor your RustChat instance health, usage, and realtime capacity from a calmer operational dashboard.</p>
        </div>

        <!-- Stats Grid -->
        <div class="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-5">
            <div 
                v-for="stat in statCards" 
                :key="stat.key"
                class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1"
            >
                <div class="mb-4 flex items-center justify-between">
                    <div>
                        <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-3">
                            {{ stat.label }}
                        </p>
                        <p class="mt-3 text-3xl font-semibold tracking-[-0.03em] text-text-1">
                            {{ getStatValue(stat.key) }}
                        </p>
                    </div>
                    <div :class="[statToneClass(stat.tone), 'flex h-11 w-11 items-center justify-center rounded-r-2 border']">
                        <component :is="stat.icon" class="h-5 w-5" />
                    </div>
                </div>
                <p class="text-sm text-text-3">Current snapshot of {{ stat.label.toLowerCase() }} across your workspace.</p>
            </div>
        </div>

        <!-- Health Status -->
        <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
            <div class="mb-4">
                <h2 class="text-lg font-semibold text-text-1">System Health</h2>
                <p class="mt-1 text-sm text-text-3">Critical services should stay readable at a glance without the dashboard shouting at you.</p>
            </div>
            
            <div v-if="adminStore.health" class="grid grid-cols-1 gap-4 md:grid-cols-3">
                <!-- Database -->
                <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
                    <div class="flex items-center gap-3">
                        <div :class="[healthToneClass(adminStore.health.database.connected), 'flex h-10 w-10 items-center justify-center rounded-full border']">
                            <CheckCircle v-if="adminStore.health.database.connected" class="h-5 w-5" />
                            <AlertCircle v-else class="h-5 w-5" />
                        </div>
                        <div>
                            <p class="font-semibold text-text-1">Database</p>
                            <p class="text-sm text-text-3">
                                {{ adminStore.health.database.connected ? `${adminStore.health.database.latency_ms}ms latency` : 'Disconnected' }}
                            </p>
                        </div>
                    </div>
                </div>

                <!-- Storage -->
                <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
                    <div class="flex items-center gap-3">
                        <div :class="[healthToneClass(adminStore.health.storage.connected), 'flex h-10 w-10 items-center justify-center rounded-full border']">
                            <HardDrive class="h-5 w-5" />
                        </div>
                        <div>
                            <p class="font-semibold text-text-1">Storage</p>
                            <p class="text-sm text-text-3">{{ adminStore.health.storage.type }}</p>
                        </div>
                    </div>
                </div>

                <!-- WebSocket -->
                <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
                    <div class="flex items-center gap-3">
                        <div class="flex h-10 w-10 items-center justify-center rounded-full border border-secondary/15 bg-secondary/10 text-secondary">
                            <Activity class="h-5 w-5" />
                        </div>
                        <div>
                            <p class="font-semibold text-text-1">WebSocket</p>
                            <p class="text-sm text-text-3">
                                {{ adminStore.health.websocket.active_connections }} connections
                            </p>
                        </div>
                    </div>
                </div>
            </div>

            <div v-else class="rounded-r-2 border border-border-1 bg-bg-surface-2 py-8 text-center text-text-3">
                <p>Loading health status...</p>
            </div>
        </div>

        <!-- Instance Info -->
        <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
            <h2 class="mb-4 text-lg font-semibold text-text-1">Instance Information</h2>
            <dl class="grid grid-cols-1 gap-4 text-sm md:grid-cols-2">
                <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
                    <dt class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-3">Version</dt>
                    <dd class="mt-2 font-semibold text-text-1">{{ adminStore.health?.version || 'v2.0.0' }}</dd>
                </div>
                <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
                    <dt class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-3">Uptime</dt>
                    <dd class="mt-2 font-semibold text-text-1">
                        {{ adminStore.health?.uptime_seconds ? Math.floor(adminStore.health.uptime_seconds / 3600) + 'h' : '—' }}
                    </dd>
                </div>
            </dl>
        </div>
    </div>
</template>
