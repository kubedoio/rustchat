<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { FileText, CheckCircle, AlertTriangle } from 'lucide-vue-next';
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
    effective_date: string;
}

interface TermsStatus {
    has_accepted: boolean;
    current_terms: TermsOfService | null;
    accepted_version: string | null;
    acceptance_required: boolean;
}

const show = ref(false);
const terms = ref<TermsOfService | null>(null);
const accepted = ref(false);
const submitting = ref(false);
const error = ref('');
const hasChecked = ref(false);

const emit = defineEmits<{
    (e: 'accepted'): void;
}>();

onMounted(async () => {
    await checkTermsStatus();
});

async function checkTermsStatus() {
    try {
        const { data } = await v4Api.get<TermsStatus>('/terms_of_service/status');
        hasChecked.value = true;
        
        if (data.acceptance_required && data.current_terms) {
            terms.value = data.current_terms;
            show.value = true;
        } else {
            // Terms already accepted or no active terms
            emit('accepted');
        }
    } catch (e: any) {
        console.error('Failed to check terms status', e);
        // On error, allow the user through
        emit('accepted');
    }
}

async function acceptTerms() {
    if (!accepted.value || !terms.value) return;
    
    submitting.value = true;
    error.value = '';
    
    try {
        await v4Api.post('/terms_of_service/accept', {
            terms_id: terms.value.id,
        });
        show.value = false;
        emit('accepted');
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to accept terms. Please try again.';
    } finally {
        submitting.value = false;
    }
}

function logout() {
    // Trigger logout - this will be handled by the auth store
    window.location.href = '/login';
}
</script>

<template>
    <Teleport to="body">
        <div 
            v-if="show && terms" 
            class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm"
        >
            <div class="relative bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 w-full max-w-2xl max-h-[90vh] flex flex-col">
                <!-- Header -->
                <div class="flex items-start gap-3 px-6 py-5 border-b border-border-1">
                    <div class="rounded-lg bg-brand/10 p-2 shrink-0">
                        <FileText class="w-5 h-5 text-brand" />
                    </div>
                    <div class="flex-1 min-w-0">
                        <h2 class="text-lg font-semibold text-text-1">{{ terms.title }}</h2>
                        <p class="text-xs text-text-3 mt-1">
                            Version {{ terms.version }} • Effective {{ new Date(terms.effective_date).toLocaleDateString() }}
                        </p>
                    </div>
                    <div class="px-2 py-1 bg-warning/10 text-warning text-[10px] rounded font-medium shrink-0">
                        Required
                    </div>
                </div>

                <!-- Error Alert -->
                <div v-if="error" class="mx-6 mt-4 flex items-center gap-2 p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs">
                    <AlertTriangle class="w-4 h-4 shrink-0" />
                    {{ error }}
                </div>

                <!-- Content -->
                <div class="flex-1 overflow-y-auto p-6">
                    <div v-if="terms.summary" class="p-3 bg-bg-surface-2 rounded-lg mb-4">
                        <p class="text-xs text-text-2">{{ terms.summary }}</p>
                    </div>
                    
                    <div class="prose prose-sm max-w-none text-text-1">
                        <pre class="whitespace-pre-wrap font-sans text-sm leading-relaxed">{{ terms.content }}</pre>
                    </div>
                </div>

                <!-- Footer -->
                <div class="px-6 py-5 border-t border-border-1 space-y-4">
                    <label class="flex items-start gap-3 cursor-pointer">
                        <input 
                            v-model="accepted"
                            type="checkbox"
                            class="w-4 h-4 text-brand rounded border-border-1 mt-0.5 shrink-0"
                        />
                        <span class="text-xs text-text-2">
                            I have read and agree to the {{ terms.title }}. I understand that by accepting these terms, 
                            I am bound by the policies and guidelines outlined above.
                        </span>
                    </label>

                    <div class="flex items-center justify-end gap-3">
                        <button 
                            @click="logout"
                            class="px-4 py-2 text-text-3 hover:text-text-1 text-xs font-medium transition-colors"
                        >
                            Decline & Logout
                        </button>
                        <button 
                            @click="acceptTerms"
                            :disabled="!accepted || submitting"
                            class="flex items-center gap-2 px-4 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
                        >
                            <CheckCircle class="w-3.5 h-3.5" />
                            {{ submitting ? 'Accepting...' : 'Accept Terms' }}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </Teleport>
</template>
