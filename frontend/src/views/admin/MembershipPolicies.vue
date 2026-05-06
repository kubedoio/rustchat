<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { 
    Plus, Search, Edit3, Trash2, RefreshCw, Users, Building2, 
    Hash, CheckCircle, XCircle, Globe, Shield, AlertCircle
} from 'lucide-vue-next';
import { format } from 'date-fns';
import { useToast } from '../../composables/useToast';
import membershipPoliciesApi, { 
    type PolicyWithTargets, 
    type PolicyScopeType,
    type PolicySourceType,
    type AutoMembershipPolicyAudit 
} from '../../api/membershipPolicies';
import PolicyEditorModal from '../../components/admin/PolicyEditorModal.vue';
import PolicyPreviewModal from '../../components/admin/PolicyPreviewModal.vue';

const toast = useToast();

// State
const policies = ref<PolicyWithTargets[]>([]);
const loading = ref(false);
const searchQuery = ref('');
const filterScope = ref<PolicyScopeType | 'all'>('all');
const filterEnabled = ref<boolean | 'all'>('all');

// Modals
const showEditor = ref(false);
const showPreview = ref(false);
const showAudit = ref(false);
const editingPolicy = ref<PolicyWithTargets | null>(null);
const previewingPolicy = ref<PolicyWithTargets | null>(null);
const auditPolicy = ref<PolicyWithTargets | null>(null);
const auditLogs = ref<AutoMembershipPolicyAudit[]>([]);
const auditLoading = ref(false);

// Fetch policies
async function fetchPolicies() {
    loading.value = true;
    try {
        const query: any = {};
        if (filterScope.value !== 'all') query.scope_type = filterScope.value;
        if (filterEnabled.value !== 'all') query.enabled = filterEnabled.value;
        
        const response = await membershipPoliciesApi.listPolicies(query);
        policies.value = response.data;
    } catch (error: any) {
        toast.error('Failed to load policies', error.response?.data?.message || error.message);
    } finally {
        loading.value = false;
    }
}

// Filtered policies
const filteredPolicies = computed(() => {
    let result = policies.value;
    
    if (searchQuery.value.trim()) {
        const query = searchQuery.value.toLowerCase();
        result = result.filter(p => 
            p.name.toLowerCase().includes(query) ||
            p.description?.toLowerCase().includes(query)
        );
    }
    
    return result;
});

// Create new policy
function createPolicy() {
    editingPolicy.value = null;
    showEditor.value = true;
}

// Edit existing policy
function editPolicy(policy: PolicyWithTargets) {
    editingPolicy.value = policy;
    showEditor.value = true;
}

// Preview policy impact
function openPreview(policy: PolicyWithTargets) {
    previewingPolicy.value = policy;
    showPreview.value = true;
}

// View audit logs
async function viewAudit(policy: PolicyWithTargets) {
    auditPolicy.value = policy;
    showAudit.value = true;
    auditLoading.value = true;
    
    try {
        const response = await membershipPoliciesApi.getPolicyAudit(policy.id, { limit: 50 });
        auditLogs.value = response.data;
    } catch (error: any) {
        toast.error('Failed to load audit logs', error.response?.data?.message || error.message);
    } finally {
        auditLoading.value = false;
    }
}

// Toggle policy enabled state
async function togglePolicy(policy: PolicyWithTargets) {
    try {
        await membershipPoliciesApi.updatePolicy(policy.id, { enabled: !policy.enabled });
        policy.enabled = !policy.enabled;
        toast.success(`Policy ${policy.enabled ? 'enabled' : 'disabled'}`);
    } catch (error: any) {
        toast.error('Failed to update policy', error.response?.data?.message || error.message);
    }
}

// Delete policy
async function deletePolicy(policy: PolicyWithTargets) {
    if (!confirm(`Are you sure you want to delete the policy "${policy.name}"?`)) {
        return;
    }
    
    try {
        await membershipPoliciesApi.deletePolicy(policy.id);
        policies.value = policies.value.filter(p => p.id !== policy.id);
        toast.success('Policy deleted');
    } catch (error: any) {
        toast.error('Failed to delete policy', error.response?.data?.message || error.message);
    }
}

// Handle policy saved
function handlePolicySaved(savedPolicy: PolicyWithTargets) {
    const index = policies.value.findIndex(p => p.id === savedPolicy.id);
    if (index >= 0) {
        policies.value[index] = savedPolicy;
    } else {
        policies.value.push(savedPolicy);
    }
    showEditor.value = false;
    toast.success('Policy saved successfully');
}

// Helper functions
function getScopeIcon(scope: PolicyScopeType) {
    return scope === 'global' ? Globe : Building2;
}

function getSourceLabel(source: PolicySourceType) {
    const labels: Record<PolicySourceType, string> = {
        all_users: 'All Users',
        auth_service: 'Auth Service',
        group: 'Group',
        role: 'Role',
        org: 'Organization'
    };
    return labels[source] || source;
}

function getTargetSummary(policy: PolicyWithTargets) {
    const teams = policy.targets.filter(t => t.target_type === 'team').length;
    const channels = policy.targets.filter(t => t.target_type === 'channel').length;
    
    const parts: string[] = [];
    if (teams > 0) parts.push(`${teams} team${teams > 1 ? 's' : ''}`);
    if (channels > 0) parts.push(`${channels} channel${channels > 1 ? 's' : ''}`);
    
    return parts.join(', ') || 'No targets';
}

onMounted(fetchPolicies);
</script>

<template>
    <div class="space-y-6">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">Membership Policies</h1>
                <p class="text-gray-500 mt-1">
                    Configure automatic team and channel membership based on user attributes
                </p>
            </div>
            <button
                @click="createPolicy"
                class="flex items-center px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
            >
                <Plus class="w-5 h-5 mr-2" />
                New Policy
            </button>
        </div>

        <!-- Filters -->
        <div class="flex flex-wrap gap-4 p-4 bg-white rounded-lg shadow-sm">
            <div class="flex-1 min-w-[200px]">
                <div class="relative">
                    <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
                    <input
                        v-model="searchQuery"
                        type="text"
                        placeholder="Search policies..."
                        class="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
                    />
                </div>
            </div>
            
            <select
                v-model="filterScope"
                @change="fetchPolicies"
                class="px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
            >
                <option value="all">All Scopes</option>
                <option value="global">Global</option>
                <option value="team">Team</option>
            </select>
            
            <select
                v-model="filterEnabled"
                @change="fetchPolicies"
                class="px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
            >
                <option value="all">All Status</option>
                <option value="true">Enabled</option>
                <option value="false">Disabled</option>
            </select>
            
            <button
                @click="fetchPolicies"
                class="flex items-center px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
            >
                <RefreshCw class="w-4 h-4 mr-2" :class="{ 'animate-spin': loading }" />
                Refresh
            </button>
        </div>

        <!-- Policies List -->
        <div class="bg-white rounded-lg shadow-sm overflow-hidden">
            <div v-if="loading" class="p-8 text-center text-gray-500">
                <RefreshCw class="w-8 h-8 animate-spin mx-auto mb-4" />
                <p>Loading policies...</p>
            </div>
            
            <div v-else-if="filteredPolicies.length === 0" class="p-8 text-center text-gray-500">
                <Shield class="w-16 h-16 mx-auto mb-4 text-gray-300" />
                <p class="text-lg font-medium text-gray-700">No policies found</p>
                <p class="mt-1">Create a policy to automatically manage team and channel memberships</p>
            </div>
            
            <div v-else class="divide-y divide-gray-200">
                <div
                    v-for="policy in filteredPolicies"
                    :key="policy.id"
                    class="p-6 hover:bg-gray-50 transition-colors"
                >
                    <div class="flex items-start justify-between">
                        <div class="flex-1 min-w-0">
                            <div class="flex items-center space-x-3">
                                <component
                                    :is="getScopeIcon(policy.scope_type)"
                                    class="w-5 h-5 text-gray-400"
                                />
                                <h3 class="text-lg font-semibold text-gray-900">
                                    {{ policy.name }}
                                </h3>
                                <span
                                    :class="[
                                        'px-2 py-0.5 text-xs rounded-full font-medium',
                                        policy.enabled
                                            ? 'bg-green-100 text-green-800'
                                            : 'bg-gray-100 text-gray-800'
                                    ]"
                                >
                                    {{ policy.enabled ? 'Enabled' : 'Disabled' }}
                                </span>
                                <span
                                    v-if="policy.scope_type === 'team'"
                                    class="px-2 py-0.5 text-xs rounded-full bg-blue-100 text-blue-800"
                                >
                                    Team
                                </span>
                            </div>
                            
                            <p v-if="policy.description" class="mt-1 text-gray-600">
                                {{ policy.description }}
                            </p>
                            
                            <div class="mt-3 flex flex-wrap items-center gap-4 text-sm text-gray-500">
                                <span class="flex items-center">
                                    <Users class="w-4 h-4 mr-1" />
                                    Applies to: {{ getSourceLabel(policy.source_type) }}
                                </span>
                                <span class="flex items-center">
                                    <Hash class="w-4 h-4 mr-1" />
                                    {{ getTargetSummary(policy) }}
                                </span>
                                <span class="flex items-center">
                                    Priority: {{ policy.priority }}
                                </span>
                            </div>
                        </div>
                        
                        <div class="flex items-center space-x-2 ml-4">
                            <button
                                @click="openPreview(policy)"
                                class="p-2 text-gray-500 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition-colors"
                                title="Preview impact"
                            >
                                <AlertCircle class="w-5 h-5" />
                            </button>
                            <button
                                @click="viewAudit(policy)"
                                class="p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                                title="View audit log"
                            >
                                <CheckCircle class="w-5 h-5" />
                            </button>
                            <button
                                @click="togglePolicy(policy)"
                                :class="[
                                    'p-2 rounded-lg transition-colors',
                                    policy.enabled
                                        ? 'text-green-600 hover:bg-green-50'
                                        : 'text-gray-400 hover:text-green-600 hover:bg-green-50'
                                ]"
                                :title="policy.enabled ? 'Disable' : 'Enable'"
                            >
                                <CheckCircle class="w-5 h-5" :class="{ 'fill-current': policy.enabled }" />
                            </button>
                            <button
                                @click="editPolicy(policy)"
                                class="p-2 text-gray-500 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition-colors"
                                title="Edit"
                            >
                                <Edit3 class="w-5 h-5" />
                            </button>
                            <button
                                @click="deletePolicy(policy)"
                                class="p-2 text-gray-500 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                                title="Delete"
                            >
                                <Trash2 class="w-5 h-5" />
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Policy Editor Modal -->
        <PolicyEditorModal
            v-if="showEditor"
            :policy="editingPolicy"
            @close="showEditor = false"
            @saved="handlePolicySaved"
        />

        <!-- Policy Preview Modal -->
        <PolicyPreviewModal
            v-if="showPreview"
            :policy="previewingPolicy"
            @close="showPreview = false"
        />

        <!-- Audit Log Modal -->
        <div
            v-if="showAudit"
            class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
            @click.self="showAudit = false"
        >
            <div class="bg-white rounded-lg shadow-xl max-w-4xl w-full mx-4 max-h-[80vh] flex flex-col">
                <div class="flex items-center justify-between p-6 border-b border-gray-200">
                    <div>
                        <h2 class="text-xl font-bold text-gray-900">
                            Audit Log: {{ auditPolicy?.name }}
                        </h2>
                        <p class="text-sm text-gray-500 mt-1">Recent policy application events</p>
                    </div>
                    <button
                        @click="showAudit = false"
                        class="p-2 text-gray-500 hover:text-gray-700 rounded-lg"
                    >
                        <XCircle class="w-6 h-6" />
                    </button>
                </div>
                
                <div class="flex-1 overflow-auto p-6">
                    <div v-if="auditLoading" class="text-center py-8">
                        <RefreshCw class="w-8 h-8 animate-spin mx-auto text-gray-400" />
                    </div>
                    
                    <div v-else-if="auditLogs.length === 0" class="text-center py-8 text-gray-500">
                        <AlertCircle class="w-12 h-12 mx-auto mb-2 text-gray-300" />
                        <p>No audit entries found</p>
                    </div>
                    
                    <table v-else class="w-full">
                        <thead class="text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                            <tr>
                                <th class="pb-3">Time</th>
                                <th class="pb-3">User</th>
                                <th class="pb-3">Target</th>
                                <th class="pb-3">Action</th>
                                <th class="pb-3">Status</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200">
                            <tr v-for="log in auditLogs" :key="log.id" class="text-sm">
                                <td class="py-3 text-gray-500">
                                    {{ format(new Date(log.created_at), 'MMM d, HH:mm') }}
                                </td>
                                <td class="py-3 font-mono text-xs">{{ log.user_id.slice(0, 8) }}...</td>
                                <td class="py-3">
                                    <span class="capitalize">{{ log.target_type }}</span>
                                    <span class="text-gray-400">{{ log.target_id.slice(0, 8) }}...</span>
                                </td>
                                <td class="py-3 capitalize">{{ log.action }}</td>
                                <td class="py-3">
                                    <span
                                        :class="[
                                            'px-2 py-1 rounded-full text-xs font-medium',
                                            log.status === 'success'
                                                ? 'bg-green-100 text-green-800'
                                                : log.status === 'failed'
                                                    ? 'bg-red-100 text-red-800'
                                                    : 'bg-yellow-100 text-yellow-800'
                                        ]"
                                    >
                                        {{ log.status }}
                                    </span>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>
</template>
