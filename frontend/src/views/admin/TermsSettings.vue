<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { FileText, Plus, Edit2, Trash2, CheckCircle, AlertCircle, Users, Eye, X, Save, AlertTriangle } from 'lucide-vue-next';
import { HttpClient } from '../../api/http/HttpClient';

// Create v4 API client
const v4Api = new HttpClient({
    baseURL: '/api/v4',
    requestInterceptor: (config) => {
        const token = localStorage.getItem('auth_token');
        if (token) {
            config.headers = {
                ...config.headers,
                Authorization: `Bearer ${token}`,
            };
        }
        return config;
    },
});

interface TermsOfService {
    id: string;
    version: string;
    title: string;
    content: string;
    summary: string | null;
    is_active: boolean;
    effective_date: string;
    created_at: string;
}

interface TermsStats {
    total_users: number;
    accepted_count: number;
    pending_count: number;
    acceptance_rate: number;
}

interface PendingUser {
    id: string;
    username: string;
    email: string;
    display_name: string | null;
    created_at: string;
}

const termsList = ref<TermsOfService[]>([]);
const currentTerms = ref<TermsOfService | null>(null);
const stats = ref<TermsStats | null>(null);
const pendingUsers = ref<PendingUser[]>([]);
const loading = ref(false);
const error = ref('');

// Modals
const showCreateModal = ref(false);
const showEditModal = ref(false);
const showPreviewModal = ref(false);
const showPendingModal = ref(false);
const editingTerms = ref<TermsOfService | null>(null);

// Form
const form = ref({
    version: '',
    title: '',
    content: '',
    summary: '',
    effective_date: new Date().toISOString().split('T')[0],
});

onMounted(async () => {
    await fetchTermsList();
    await fetchTermsStats();
});

async function fetchTermsList() {
    loading.value = true;
    try {
        const { data } = await v4Api.get('/terms_of_service');
        termsList.value = data;
        currentTerms.value = data.find((t: TermsOfService) => t.is_active) || null;
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to load terms';
    } finally {
        loading.value = false;
    }
}

async function fetchTermsStats() {
    try {
        const { data } = await v4Api.get('/terms_of_service/stats/summary');
        if (data.has_active_terms) {
            stats.value = {
                total_users: data.total_users,
                accepted_count: data.accepted_count,
                pending_count: data.pending_count,
                acceptance_rate: data.acceptance_rate,
            };
            pendingUsers.value = data.pending_users || [];
            currentTerms.value = data.current_terms;
        }
    } catch (e: any) {
        console.error('Failed to load stats', e);
    }
}

function resetForm() {
    form.value = {
        version: '',
        title: '',
        content: '',
        summary: '',
        effective_date: new Date().toISOString().split('T')[0],
    };
    editingTerms.value = null;
}

function openCreateModal() {
    resetForm();
    showCreateModal.value = true;
}

function openEditModal(terms: TermsOfService) {
    editingTerms.value = terms;
    form.value = {
        version: terms.version,
        title: terms.title,
        content: terms.content,
        summary: terms.summary || '',
        effective_date: new Date(terms.effective_date).toISOString().split('T')[0],
    };
    showEditModal.value = true;
}

function openPreviewModal(terms: TermsOfService) {
    editingTerms.value = terms;
    showPreviewModal.value = true;
}

async function createTerms() {
    try {
        await v4Api.post('/terms_of_service', {
            ...form.value,
            effective_date: new Date(form.value.effective_date || new Date()).toISOString(),
        });
        showCreateModal.value = false;
        resetForm();
        await fetchTermsList();
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to create terms';
    }
}

async function updateTerms() {
    if (!editingTerms.value) return;
    try {
        await v4Api.put(`/terms_of_service/${editingTerms.value.id}`, {
            title: form.value.title,
            content: form.value.content,
            summary: form.value.summary || undefined,
            effective_date: new Date(form.value.effective_date || new Date()).toISOString(),
        });
        showEditModal.value = false;
        resetForm();
        await fetchTermsList();
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to update terms';
    }
}

async function activateTerms(terms: TermsOfService) {
    if (!confirm(`Activate "${terms.title}"? This will require all users to accept the new terms.`)) {
        return;
    }
    try {
        await v4Api.post(`/terms_of_service/${terms.id}/activate`);
        await fetchTermsList();
        await fetchTermsStats();
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to activate terms';
    }
}

async function deleteTerms(terms: TermsOfService) {
    if (terms.is_active) {
        alert('Cannot delete active terms. Deactivate first.');
        return;
    }
    if (!confirm(`Delete "${terms.title}"? This action cannot be undone.`)) {
        return;
    }
    try {
        await v4Api.delete(`/terms_of_service/${terms.id}`);
        await fetchTermsList();
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to delete terms';
    }
}

const formatDate = (date: string) => {
    return new Date(date).toLocaleDateString();
};
</script>

<template>
    <div class="space-y-5">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-lg font-semibold text-text-1">Terms of Service</h1>
                <p class="text-text-3 text-xs mt-0.5">Manage terms versions and track user acceptances</p>
            </div>
            <button 
                @click="openCreateModal"
                class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
            >
                <Plus class="w-3.5 h-3.5" />
                New Terms Version
            </button>
        </div>

        <!-- Error Alert -->
        <div v-if="error" class="flex items-center gap-2 p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs">
            <AlertTriangle class="w-4 h-4 shrink-0" />
            {{ error }}
        </div>

        <!-- Stats Cards -->
        <div v-if="stats" class="grid grid-cols-2 md:grid-cols-4 gap-3">
            <div class="bg-bg-surface-1 rounded-lg border border-border-1 p-4">
                <div class="flex items-center gap-2 text-text-3 mb-1">
                    <Users class="w-3.5 h-3.5" />
                    <span class="text-[10px] uppercase tracking-wider">Total Users</span>
                </div>
                <div class="text-2xl font-semibold text-text-1">{{ stats.total_users }}</div>
            </div>
            <div class="bg-bg-surface-1 rounded-lg border border-border-1 p-4">
                <div class="flex items-center gap-2 text-success mb-1">
                    <CheckCircle class="w-3.5 h-3.5" />
                    <span class="text-[10px] uppercase tracking-wider">Accepted</span>
                </div>
                <div class="text-2xl font-semibold text-success">{{ stats.accepted_count }}</div>
            </div>
            <div class="bg-bg-surface-1 rounded-lg border border-border-1 p-4">
                <div class="flex items-center gap-2 text-warning mb-1">
                    <AlertCircle class="w-3.5 h-3.5" />
                    <span class="text-[10px] uppercase tracking-wider">Pending</span>
                </div>
                <div class="text-2xl font-semibold text-warning">{{ stats.pending_count }}</div>
                <button 
                    v-if="stats.pending_count > 0"
                    @click="showPendingModal = true"
                    class="text-[10px] text-brand hover:underline mt-1"
                >
                    View users
                </button>
            </div>
            <div class="bg-bg-surface-1 rounded-lg border border-border-1 p-4">
                <div class="flex items-center gap-2 text-primary mb-1">
                    <FileText class="w-3.5 h-3.5" />
                    <span class="text-[10px] uppercase tracking-wider">Acceptance Rate</span>
                </div>
                <div class="text-2xl font-semibold text-primary">{{ stats.acceptance_rate.toFixed(1) }}%</div>
            </div>
        </div>

        <!-- Current Active Terms -->
        <div v-if="currentTerms" class="bg-brand/5 rounded-xl border border-brand/20 p-5">
            <div class="flex items-start justify-between">
                <div class="flex items-start gap-3">
                    <div class="rounded-lg bg-brand/10 p-2">
                        <CheckCircle class="w-4 h-4 text-brand" />
                    </div>
                    <div>
                        <div class="flex items-center gap-2">
                            <h3 class="font-semibold text-text-1">{{ currentTerms.title }}</h3>
                            <span class="px-1.5 py-0.5 bg-brand text-white text-[9px] rounded font-medium">Active</span>
                        </div>
                        <p class="text-[10px] text-text-3 mt-1">Version {{ currentTerms.version }} • Effective {{ formatDate(currentTerms.effective_date) }}</p>
                        <p v-if="currentTerms.summary" class="text-xs text-text-2 mt-2">{{ currentTerms.summary }}</p>
                    </div>
                </div>
                <div class="flex items-center gap-1">
                    <button 
                        @click="openPreviewModal(currentTerms)"
                        class="p-1.5 text-text-4 hover:text-brand hover:bg-brand/10 rounded-lg transition-colors"
                        title="Preview"
                    >
                        <Eye class="w-3.5 h-3.5" />
                    </button>
                    <button 
                        @click="openEditModal(currentTerms)"
                        class="p-1.5 text-text-4 hover:text-primary hover:bg-primary/10 rounded-lg transition-colors"
                        title="Edit"
                    >
                        <Edit2 class="w-3.5 h-3.5" />
                    </button>
                </div>
            </div>
        </div>

        <!-- All Terms Versions -->
        <div class="bg-bg-surface-1 rounded-xl border border-border-1 overflow-hidden">
            <div class="px-5 py-4 border-b border-border-1">
                <h3 class="font-semibold text-text-1 text-sm">All Versions</h3>
            </div>
            
            <div v-if="loading" class="p-8 text-center">
                <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-brand mx-auto"></div>
            </div>

            <div v-else-if="termsList.length === 0" class="p-8 text-center text-text-3 text-xs">
                No terms of service defined yet.
            </div>

            <div v-else class="divide-y divide-border-1">
                <div 
                    v-for="terms in termsList" 
                    :key="terms.id"
                    class="px-5 py-4 flex items-center justify-between hover:bg-bg-surface-2/50 transition-colors"
                >
                    <div class="flex items-start gap-3">
                        <div class="rounded-lg p-2" :class="terms.is_active ? 'bg-brand/10' : 'bg-bg-surface-2'">
                            <FileText class="w-4 h-4" :class="terms.is_active ? 'text-brand' : 'text-text-3'" />
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <span class="font-medium text-text-1 text-xs">{{ terms.title }}</span>
                                <span v-if="terms.is_active" class="px-1.5 py-0.5 bg-brand text-white text-[9px] rounded font-medium">Active</span>
                            </div>
                            <p class="text-[10px] text-text-3 mt-0.5">Version {{ terms.version }} • Created {{ formatDate(terms.created_at) }}</p>
                        </div>
                    </div>
                    <div class="flex items-center gap-1">
                        <button 
                            v-if="!terms.is_active"
                            @click="activateTerms(terms)"
                            class="px-2 py-1 text-[10px] text-brand hover:bg-brand/10 rounded transition-colors"
                        >
                            Activate
                        </button>
                        <button 
                            @click="openPreviewModal(terms)"
                            class="p-1.5 text-text-4 hover:text-brand hover:bg-brand/10 rounded-lg transition-colors"
                            title="Preview"
                        >
                            <Eye class="w-3.5 h-3.5" />
                        </button>
                        <button 
                            @click="openEditModal(terms)"
                            class="p-1.5 text-text-4 hover:text-primary hover:bg-primary/10 rounded-lg transition-colors"
                            title="Edit"
                        >
                            <Edit2 class="w-3.5 h-3.5" />
                        </button>
                        <button 
                            @click="deleteTerms(terms)"
                            class="p-1.5 text-text-4 hover:text-danger hover:bg-danger/10 rounded-lg transition-colors"
                            title="Delete"
                        >
                            <Trash2 class="w-3.5 h-3.5" />
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Create/Edit Modal -->
        <div v-if="showCreateModal || showEditModal" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showCreateModal = false; showEditModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col">
                <div class="flex items-center justify-between px-5 py-4 border-b border-border-1">
                    <h3 class="text-sm font-bold text-text-1">{{ editingTerms ? 'Edit Terms' : 'Create Terms Version' }}</h3>
                    <button @click="showCreateModal = false; showEditModal = false" class="p-1.5 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                
                <div class="p-5 space-y-4 overflow-y-auto">
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs font-medium text-text-2 mb-1.5">Version</label>
                            <input 
                                v-model="form.version"
                                type="text"
                                :disabled="!!editingTerms"
                                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none disabled:opacity-50"
                                placeholder="e.g. 1.0"
                            />
                        </div>
                        <div>
                            <label class="block text-xs font-medium text-text-2 mb-1.5">Effective Date</label>
                            <input 
                                v-model="form.effective_date"
                                type="date"
                                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
                            />
                        </div>
                    </div>
                    
                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1.5">Title</label>
                        <input 
                            v-model="form.title"
                            type="text"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
                            placeholder="Terms of Service"
                        />
                    </div>
                    
                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1.5">Summary (optional)</label>
                        <input 
                            v-model="form.summary"
                            type="text"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
                            placeholder="Brief summary of changes"
                        />
                    </div>
                    
                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1.5">Content (Markdown supported)</label>
                        <textarea 
                            v-model="form.content"
                            rows="10"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none resize-none font-mono"
                            placeholder="# Terms of Service\n\nYour terms content here..."
                        />
                    </div>
                </div>

                <div class="flex items-center justify-end gap-2 px-5 py-4 border-t border-border-1">
                    <button 
                        @click="showCreateModal = false; showEditModal = false"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button 
                        @click="editingTerms ? updateTerms() : createTerms()"
                        :disabled="!form.version || !form.title || !form.content"
                        class="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-brand hover:bg-brand/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
                    >
                        <Save class="w-3.5 h-3.5" />
                        {{ editingTerms ? 'Save Changes' : 'Create Version' }}
                    </button>
                </div>
            </div>
        </div>

        <!-- Preview Modal -->
        <div v-if="showPreviewModal && editingTerms" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showPreviewModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col">
                <div class="flex items-center justify-between px-5 py-4 border-b border-border-1">
                    <div>
                        <h3 class="text-sm font-bold text-text-1">{{ editingTerms.title }}</h3>
                        <p class="text-[10px] text-text-3">Version {{ editingTerms.version }}</p>
                    </div>
                    <button @click="showPreviewModal = false" class="p-1.5 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                
                <div class="p-5 overflow-y-auto">
                    <div v-if="editingTerms.summary" class="p-3 bg-bg-surface-2 rounded-lg mb-4">
                        <p class="text-xs text-text-2">{{ editingTerms.summary }}</p>
                    </div>
                    <div class="prose prose-sm max-w-none text-text-1">
                        <pre class="whitespace-pre-wrap font-sans text-xs">{{ editingTerms.content }}</pre>
                    </div>
                </div>
            </div>
        </div>

        <!-- Pending Users Modal -->
        <div v-if="showPendingModal" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showPendingModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-lg max-h-[90vh] overflow-hidden flex flex-col">
                <div class="flex items-center justify-between px-5 py-4 border-b border-border-1">
                    <div>
                        <h3 class="text-sm font-bold text-text-1">Users Pending Acceptance</h3>
                        <p class="text-[10px] text-text-3">{{ pendingUsers.length }} users haven't accepted the terms</p>
                    </div>
                    <button @click="showPendingModal = false" class="p-1.5 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                
                <div class="overflow-y-auto max-h-[60vh]">
                    <div v-if="pendingUsers.length === 0" class="p-8 text-center text-text-3 text-xs">
                        All users have accepted the terms.
                    </div>
                    <div v-else class="divide-y divide-border-1">
                        <div v-for="user in pendingUsers" :key="user.id" class="px-5 py-3 flex items-center gap-3">
                            <div class="w-8 h-8 rounded-full bg-brand/10 flex items-center justify-center text-brand text-xs font-bold">
                                {{ (user.username?.charAt(0) || 'U').toUpperCase() }}
                            </div>
                            <div class="min-w-0 flex-1">
                                <div class="text-xs font-medium text-text-1">{{ user.display_name || user.username }}</div>
                                <div class="text-[10px] text-text-3 truncate">{{ user.email }}</div>
                            </div>
                            <span class="text-[10px] text-text-4">{{ formatDate(user.created_at) }}</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
