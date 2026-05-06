<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { 
    AlertTriangle, CheckCircle, XCircle, RefreshCw, 
    Download, Filter, Activity, TrendingUp,
    FileText, ChevronDown, ChevronUp
} from 'lucide-vue-next';
import { format, subDays } from 'date-fns';
import { useToast } from '../../composables/useToast';
import api from '../../api/client';

const toast = useToast();

// State
const loading = ref(false);
const summary = ref({
    total_operations_24h: 0,
    successful_operations_24h: 0,
    failed_operations_24h: 0,
    failure_rate_24h: 0,
    pending_operations: 0,
    policies_with_failures: 0
});
const recentFailures = ref<any[]>([]);
const policyStats = ref<any[]>([]);
const auditLogs = ref<any[]>([]);
const showFilters = ref(false);

// Filters
const filters = ref({
    status: '',
    action: '',
    policy_id: '',
    from_date: format(subDays(new Date(), 7), 'yyyy-MM-dd'),
    to_date: format(new Date(), 'yyyy-MM-dd')
});

// Computed
const hasFailures = computed(() => summary.value.failed_operations_24h > 0);
const failureRateClass = computed(() => {
    const rate = summary.value.failure_rate_24h;
    if (rate < 5) return 'text-success';
    if (rate < 15) return 'text-warning';
    return 'text-danger';
});

// Fetch all data
async function fetchDashboard() {
    loading.value = true;
    try {
        const [summaryRes, failuresRes, statsRes] = await Promise.all([
            api.get('/admin/audit/membership/summary'),
            api.get('/admin/audit/membership/recent-failures'),
            api.get('/admin/audit/membership/failures')
        ]);
        
        summary.value = summaryRes.data;
        recentFailures.value = failuresRes.data;
        policyStats.value = statsRes.data;
    } catch (error: any) {
        toast.error('Failed to load audit data', error.message);
    } finally {
        loading.value = false;
    }
}

// Fetch audit logs with filters
async function fetchAuditLogs() {
    loading.value = true;
    try {
        const params: any = {};
        if (filters.value.status) params.status = filters.value.status;
        if (filters.value.action) params.action = filters.value.action;
        if (filters.value.from_date) params.from_date = new Date(filters.value.from_date).toISOString();
        if (filters.value.to_date) params.to_date = new Date(filters.value.to_date).toISOString();
        
        const response = await api.get('/admin/audit/membership', { params });
        auditLogs.value = response.data;
    } catch (error: any) {
        toast.error('Failed to load audit logs', error.message);
    } finally {
        loading.value = false;
    }
}

// Export logs
async function exportLogs() {
    try {
        const response = await api.get('/admin/audit/membership/export', {
            params: filters.value,
            responseType: 'blob'
        });
        
        const blob = new Blob([JSON.stringify(response.data, null, 2)], { type: 'application/json' });
        const url = window.URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = `audit-logs-${format(new Date(), 'yyyy-MM-dd')}.json`;
        link.click();
        window.URL.revokeObjectURL(url);
        
        toast.success('Audit logs exported');
    } catch (error: any) {
        toast.error('Export failed', error.message);
    }
}

// Refresh data
function refresh() {
    fetchDashboard();
    fetchAuditLogs();
}

onMounted(() => {
    fetchDashboard();
    fetchAuditLogs();
});
</script>

<template>
    <div class="space-y-6">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-text-1">Audit Dashboard</h1>
                <p class="text-text-3 mt-1">
                    Monitor membership policy execution and failures
                </p>
            </div>
            <div class="flex space-x-2">
                <button
                    @click="exportLogs"
                    class="flex items-center px-4 py-2 border border-border-2 rounded-lg hover:bg-bg-surface-2 transition-colors"
                >
                    <Download class="w-4 h-4 mr-2" />
                    Export
                </button>
                <button
                    @click="refresh"
                    class="flex items-center px-4 py-2 bg-brand text-white rounded-lg hover:bg-brand/90 transition-colors"
                    :disabled="loading"
                >
                    <RefreshCw class="w-4 h-4 mr-2" :class="{ 'animate-spin': loading }" />
                    Refresh
                </button>
            </div>
        </div>

        <!-- Summary Cards -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div class="p-6 bg-bg-surface-1 rounded-lg shadow-sm border border-border-1">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm font-medium text-text-3">Total Operations (24h)</p>
                        <p class="text-2xl font-bold text-text-1 mt-1">{{ summary.total_operations_24h }}</p>
                    </div>
                    <div class="p-3 bg-brand/10 rounded-lg">
                        <Activity class="w-6 h-6 text-brand" />
                    </div>
                </div>
            </div>

            <div class="p-6 bg-bg-surface-1 rounded-lg shadow-sm border border-border-1">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm font-medium text-text-3">Successful (24h)</p>
                        <p class="text-2xl font-bold text-success mt-1">{{ summary.successful_operations_24h }}</p>
                    </div>
                    <div class="p-3 bg-success/10 rounded-lg">
                        <CheckCircle class="w-6 h-6 text-success" />
                    </div>
                </div>
            </div>

            <div class="p-6 bg-bg-surface-1 rounded-lg shadow-sm border border-border-1">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm font-medium text-text-3">Failed (24h)</p>
                        <p class="text-2xl font-bold text-danger mt-1">{{ summary.failed_operations_24h }}</p>
                    </div>
                    <div class="p-3 bg-danger/10 rounded-lg">
                        <XCircle class="w-6 h-6 text-danger" />
                    </div>
                </div>
                <div v-if="hasFailures" class="mt-2 flex items-center text-sm text-danger">
                    <AlertTriangle class="w-4 h-4 mr-1" />
                    <span>{{ summary.policies_with_failures }} policies affected</span>
                </div>
            </div>

            <div class="p-6 bg-bg-surface-1 rounded-lg shadow-sm border border-border-1">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm font-medium text-text-3">Failure Rate (24h)</p>
                        <p class="text-2xl font-bold mt-1" :class="failureRateClass">
                            {{ summary.failure_rate_24h.toFixed(1) }}%
                        </p>
                    </div>
                    <div class="p-3 bg-warning/10 rounded-lg">
                        <TrendingUp class="w-6 h-6 text-warning" />
                    </div>
                </div>
            </div>
        </div>

        <!-- Alert Banner -->
        <div 
            v-if="hasFailures && summary.failure_rate_24h > 15" 
            class="p-4 bg-danger/10 border border-danger/20 rounded-lg flex items-start"
        >
            <AlertTriangle class="w-5 h-5 text-danger mr-3 mt-0.5" />
            <div>
                <h3 class="text-sm font-semibold text-danger">High Failure Rate Detected</h3>
                <p class="text-sm text-danger/80 mt-1">
                    The membership policy failure rate is {{ summary.failure_rate_24h.toFixed(1) }}% in the last 24 hours. 
                    Please review the recent failures below.
                </p>
            </div>
        </div>

        <!-- Recent Failures -->
        <div v-if="recentFailures.length > 0" class="bg-bg-surface-1 rounded-lg shadow-sm border border-border-1">
            <div class="px-6 py-4 border-b border-border-1">
                <h2 class="text-lg font-semibold text-text-1 flex items-center">
                    <AlertTriangle class="w-5 h-5 mr-2 text-danger" />
                    Recent Failures (Last Hour)
                </h2>
            </div>
            <div class="divide-y divide-border-1">
                <div
                    v-for="failure in recentFailures.slice(0, 5)"
                    :key="failure.id"
                    class="px-6 py-4 hover:bg-bg-surface-2"
                >
                    <div class="flex items-start justify-between">
                        <div>
                            <p class="text-sm font-medium text-text-1">
                                {{ failure.policy_name || 'Unknown Policy' }}
                            </p>
                            <p class="text-xs text-text-3 mt-1">
                                User: @{{ failure.username || failure.user_id.slice(0, 8) }} | 
                                Target: {{ failure.target_type }} {{ failure.target_id.slice(0, 8) }}...
                            </p>
                            <p class="text-xs text-danger mt-1 font-mono">{{ failure.error_message }}</p>
                        </div>
                        <span class="text-xs text-text-4">
                            {{ format(new Date(failure.created_at), 'HH:mm:ss') }}
                        </span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Policy Failure Stats -->
        <div v-if="policyStats.length > 0" class="bg-bg-surface-1 rounded-lg shadow-sm border border-border-1">
            <div class="px-6 py-4 border-b border-border-1">
                <h2 class="text-lg font-semibold text-text-1">Policies with Failures</h2>
            </div>
            <table class="w-full">
                <thead class="bg-bg-surface-2">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">Policy</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-text-3 uppercase">Total</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-text-3 uppercase">Failed</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-text-3 uppercase">Rate</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">Last Error</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-border-1">
                    <tr v-for="stat in policyStats" :key="stat.policy_id" class="hover:bg-bg-surface-2">
                        <td class="px-6 py-4 text-sm font-medium text-text-1">{{ stat.policy_name }}</td>
                        <td class="px-6 py-4 text-sm text-right text-text-2">{{ stat.total_operations }}</td>
                        <td class="px-6 py-4 text-sm text-right text-danger font-semibold">{{ stat.failed_operations }}</td>
                        <td class="px-6 py-4 text-sm text-right">
                            <span :class="{
                                'text-success': stat.failure_rate < 5,
                                'text-warning': stat.failure_rate >= 5 && stat.failure_rate < 15,
                                'text-danger': stat.failure_rate >= 15
                            }">{{ stat.failure_rate.toFixed(1) }}%</span>
                        </td>
                        <td class="px-6 py-4 text-xs text-text-3 truncate max-w-xs" :title="stat.last_error_message">
                            {{ stat.last_error_message || '-' }}
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>

        <!-- Audit Logs Section -->
        <div class="bg-bg-surface-1 rounded-lg shadow-sm border border-border-1">
            <div class="px-6 py-4 border-b border-border-1 flex items-center justify-between">
                <h2 class="text-lg font-semibold text-text-1 flex items-center">
                    <FileText class="w-5 h-5 mr-2" />
                    Audit Logs
                </h2>
                <button
                    @click="showFilters = !showFilters"
                    class="flex items-center text-sm text-text-2 hover:text-text-1"
                >
                    <Filter class="w-4 h-4 mr-1" />
                    Filters
                    <ChevronDown v-if="!showFilters" class="w-4 h-4 ml-1" />
                    <ChevronUp v-else class="w-4 h-4 ml-1" />
                </button>
            </div>

            <!-- Filters -->
            <div v-if="showFilters" class="px-6 py-4 bg-bg-surface-2 border-b border-border-1">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1">Status</label>
                        <select
                            v-model="filters.status"
                            @change="fetchAuditLogs"
                            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-sm"
                        >
                            <option value="">All</option>
                            <option value="success">Success</option>
                            <option value="failed">Failed</option>
                            <option value="pending">Pending</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1">Action</label>
                        <select
                            v-model="filters.action"
                            @change="fetchAuditLogs"
                            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-sm"
                        >
                            <option value="">All</option>
                            <option value="add">Add</option>
                            <option value="remove">Remove</option>
                            <option value="skip">Skip</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1">From</label>
                        <input
                            v-model="filters.from_date"
                            type="date"
                            @change="fetchAuditLogs"
                            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-sm"
                        />
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1">To</label>
                        <input
                            v-model="filters.to_date"
                            type="date"
                            @change="fetchAuditLogs"
                            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-sm"
                        />
                    </div>
                </div>
            </div>

            <!-- Logs Table -->
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-bg-surface-2">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">Time</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">Policy</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">User</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">Target</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">Action</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-3 uppercase">Status</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-border-1">
                        <tr v-for="log in auditLogs" :key="log.id" class="hover:bg-bg-surface-2">
                            <td class="px-6 py-4 text-sm text-text-3">
                                {{ format(new Date(log.created_at), 'MMM d, HH:mm:ss') }}
                            </td>
                            <td class="px-6 py-4 text-sm font-medium text-text-1">
                                {{ log.policy_name || 'Unknown' }}
                            </td>
                            <td class="px-6 py-4 text-sm text-text-2">
                                @{{ log.username || log.user_id.slice(0, 8) }}...
                            </td>
                            <td class="px-6 py-4 text-sm text-text-2">
                                {{ log.target_type }} {{ log.target_id.slice(0, 8) }}...
                            </td>
                            <td class="px-6 py-4 text-sm capitalize">{{ log.action }}</td>
                            <td class="px-6 py-4">
                                <span
                                    :class="{
                                        'px-2 py-1 text-xs rounded-full font-medium': true,
                                        'bg-success/10 text-success': log.status === 'success',
                                        'bg-danger/10 text-danger': log.status === 'failed',
                                        'bg-warning/10 text-warning': log.status === 'pending'
                                    }"
                                >
                                    {{ log.status }}
                                </span>
                            </td>
                        </tr>
                        <tr v-if="auditLogs.length === 0">
                            <td colspan="6" class="px-6 py-8 text-center text-text-3">
                                <FileText class="w-12 h-12 mx-auto mb-2 text-text-4" />
                                <p>No audit logs found</p>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
    </div>
</template>
