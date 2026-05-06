<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { Mail, Send, AlertCircle, CheckCircle, Plus, Trash2, Star } from 'lucide-vue-next';
import api from '../../api/client';
import type { MailProvider, CreateMailProviderRequest } from '../../api/admin';
import EmailAdminWorkbench from '../../components/admin/EmailAdminWorkbench.vue';

const providers = ref<MailProvider[]>([]);
const defaultProvider = ref<MailProvider | null>(null);
const loading = ref(false);
const saving = ref(false);
const saveSuccess = ref(false);
const saveError = ref('');

const testEmail = ref('');
const testing = ref(false);
const testSuccess = ref(false);
const testError = ref('');

const showAddModal = ref(false);
const editingProvider = ref<MailProvider | null>(null);

const form = ref<CreateMailProviderRequest>({
    provider_type: 'smtp',
    host: '',
    port: 587,
    username: '',
    password: '',
    tls_mode: 'starttls',
    skip_cert_verify: false,
    from_address: '',
    from_name: 'RustChat',
    reply_to: '',
    enabled: true,
    is_default: false,
});

function extractUiErrorMessage(e: any, fallback: string) {
    // Handle plain Error objects (e.g., thrown from test response)
    if (e instanceof Error && e.message && !(e as any).response) {
        return e.message;
    }
    if (!e?.response) {
        return `${fallback}: cannot connect to the RustChat server. Check the server URL/network and try again.`;
    }
    return (
        e.response?.data?.error?.message ||
        e.response?.data?.message ||
        e.response?.data?.error ||
        e.message ||
        fallback
    );
}

async function fetchProviders() {
    loading.value = true;
    try {
        const { data } = await api.get('/admin/email/providers');
        providers.value = data;
        defaultProvider.value = data.find((p: MailProvider) => p.is_default) || data[0] || null;
    } catch (e: any) {
        saveError.value = extractUiErrorMessage(e, 'Failed to load email providers');
    } finally {
        loading.value = false;
    }
}

onMounted(fetchProviders);

function resetForm() {
    form.value = {
        provider_type: 'smtp',
        host: '',
        port: 587,
        username: '',
        password: '',
        tls_mode: 'starttls',
        skip_cert_verify: false,
        from_address: '',
        from_name: 'RustChat',
        reply_to: '',
        enabled: true,
        is_default: providers.value.length === 0, // First provider is default
    };
    editingProvider.value = null;
}

function editProvider(provider: MailProvider) {
    editingProvider.value = provider;
    form.value = {
        provider_type: provider.provider_type,
        host: provider.host,
        port: provider.port,
        username: provider.username,
        password: '', // Don't show existing password
        tls_mode: provider.tls_mode,
        skip_cert_verify: provider.skip_cert_verify,
        from_address: provider.from_address,
        from_name: provider.from_name,
        reply_to: provider.reply_to || '',
        enabled: provider.enabled,
        is_default: provider.is_default,
    };
    showAddModal.value = true;
}

const saveProvider = async () => {
    saving.value = true;
    saveError.value = '';
    saveSuccess.value = false;
    
    try {
        if (editingProvider.value) {
            // Update existing - only send password if provided
            const data: Partial<CreateMailProviderRequest> = { ...form.value };
            if (!data.password) delete data.password;
            await api.put(`/admin/email/providers/${editingProvider.value.id}`, data);
        } else {
            // Create new
            await api.post('/admin/email/providers', form.value);
        }
        await fetchProviders();
        saveSuccess.value = true;
        showAddModal.value = false;
        resetForm();
        setTimeout(() => saveSuccess.value = false, 3000);
    } catch (e: any) {
        saveError.value = extractUiErrorMessage(e, 'Failed to save email provider');
    } finally {
        saving.value = false;
    }
};

const deleteProvider = async (id: string) => {
    if (!confirm('Are you sure you want to delete this provider?')) return;
    try {
        await api.delete(`/admin/email/providers/${id}`);
        await fetchProviders();
    } catch (e: any) {
        saveError.value = extractUiErrorMessage(e, 'Failed to delete provider');
    }
};

const setDefault = async (id: string) => {
    try {
        await api.post(`/admin/email/providers/${id}/default`);
        await fetchProviders();
    } catch (e: any) {
        saveError.value = extractUiErrorMessage(e, 'Failed to set default provider');
    }
};

const sendTestEmail = async () => {
    if (!testEmail.value || !defaultProvider.value) return;
    testing.value = true;
    testError.value = '';
    testSuccess.value = false;

    try {
        const { data } = await api.post(`/admin/email/providers/${defaultProvider.value.id}/test`, { 
            to_email: testEmail.value 
        });
        if (!data?.success) {
            // Build detailed error message from response
            let errorMsg = data?.error || 'SMTP test failed';
            if (data?.stage) {
                errorMsg = `[${data.stage}] ${errorMsg}`;
            }
            testError.value = errorMsg;
            return;
        }
        testSuccess.value = true;
        setTimeout(() => testSuccess.value = false, 5000);
    } catch (e: any) {
        testError.value = extractUiErrorMessage(e, 'Failed to send test email');
    } finally {
        testing.value = false;
    }
};
</script>

<template>
    <div class="space-y-6">
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">Email & SMTP</h1>
                <p class="text-gray-500 mt-1">Configure email delivery providers</p>
            </div>
            <div class="flex items-center gap-3">
                <span v-if="saveSuccess" class="flex items-center text-green-600 text-sm">
                    <CheckCircle class="w-4 h-4 mr-1" /> Saved
                </span>
                <button 
                    @click="showAddModal = true; resetForm()"
                    class="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg font-medium transition-colors"
                >
                    <Plus class="w-5 h-5 mr-2" />
                    Add Provider
                </button>
            </div>
        </div>

        <!-- Error Alert -->
        <div v-if="saveError" class="flex items-center gap-2 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700">
            <AlertCircle class="w-5 h-5 shrink-0" />
            {{ saveError }}
        </div>

        <!-- Providers List -->
        <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <div class="flex items-center mb-6">
                <Mail class="w-5 h-5 text-gray-400 mr-2" />
                <h2 class="text-lg font-semibold text-gray-900">Email Providers</h2>
            </div>

            <div v-if="loading" class="text-center py-8 text-gray-500">
                Loading providers...
            </div>

            <div v-else-if="providers.length === 0" class="text-center py-8 text-gray-500">
                No email providers configured. Add one to enable email notifications.
            </div>

            <div v-else class="space-y-4">
                <div v-for="provider in providers" :key="provider.id" 
                     class="flex items-center justify-between p-4 border border-gray-200 rounded-lg"
                     :class="{ 'bg-indigo-50 border-indigo-200': provider.is_default }">
                    <div class="flex items-center gap-4">
                        <div class="w-10 h-10 rounded-full bg-gray-100 flex items-center justify-center">
                            <Mail class="w-5 h-5 text-gray-500" />
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <span class="font-medium text-gray-900">{{ provider.host }}:{{ provider.port }}</span>
                                <span v-if="provider.is_default" class="px-2 py-0.5 bg-indigo-100 text-indigo-700 text-xs rounded-full">Default</span>
                                <span v-if="!provider.enabled" class="px-2 py-0.5 bg-gray-100 text-gray-600 text-xs rounded-full">Disabled</span>
                            </div>
                            <div class="text-sm text-gray-500">
                                {{ provider.from_name }} &lt;{{ provider.from_address }}&gt; • {{ provider.tls_mode.toUpperCase() }}
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center gap-2">
                        <button v-if="!provider.is_default" @click="setDefault(provider.id)"
                                class="p-2 text-gray-500 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg"
                                title="Set as default">
                            <Star class="w-5 h-5" />
                        </button>
                        <button @click="editProvider(provider)"
                                class="p-2 text-gray-500 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg">
                            Edit
                        </button>
                        <button @click="deleteProvider(provider.id)"
                                class="p-2 text-red-500 hover:text-red-700 hover:bg-red-50 rounded-lg">
                            <Trash2 class="w-5 h-5" />
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Test Email -->
        <div v-if="defaultProvider" class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <h3 class="text-lg font-semibold text-gray-900 mb-4">Send Test Email</h3>
            
            <div v-if="testSuccess" class="mb-4 p-3 bg-green-50 text-green-700 rounded-lg text-sm">
                Test email sent successfully!
            </div>
            
            <div v-if="testError" class="mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm">
                {{ testError }}
            </div>

            <div class="flex items-center space-x-4">
                <input 
                    v-model="testEmail"
                    type="email"
                    class="flex-1 px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900"
                    placeholder="test@example.com"
                />
                <button 
                    @click="sendTestEmail"
                    :disabled="testing || !testEmail"
                    class="flex items-center px-4 py-2 bg-green-600 hover:bg-green-700 disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
                >
                    <Send class="w-4 h-4 mr-2" />
                    {{ testing ? 'Sending...' : 'Send Test' }}
                </button>
            </div>
        </div>

        <EmailAdminWorkbench />

        <!-- Add/Edit Modal -->
        <div v-if="showAddModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
            <div class="bg-white rounded-xl shadow-lg max-w-2xl w-full max-h-[90vh] overflow-y-auto">
                <div class="p-6 border-b border-gray-200">
                    <h3 class="text-lg font-semibold text-gray-900">
                        {{ editingProvider ? 'Edit Provider' : 'Add Email Provider' }}
                    </h3>
                </div>
                
                <div class="p-6 space-y-4">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">SMTP Host</label>
                            <input v-model="form.host" type="text" placeholder="smtp.example.com"
                                   class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">SMTP Port</label>
                            <input v-model.number="form.port" type="number"
                                   class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Username</label>
                            <input v-model="form.username" type="text"
                                   class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">
                                Password {{ editingProvider ? '(leave blank to keep existing)' : '' }}
                            </label>
                            <input v-model="form.password" type="password"
                                   class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">From Address</label>
                            <input v-model="form.from_address" type="email" placeholder="noreply@example.com"
                                   class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">From Name</label>
                            <input v-model="form.from_name" type="text"
                                   class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900" />
                        </div>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">SMTP Security</label>
                            <select v-model="form.tls_mode"
                                    class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900">
                                <option value="starttls">STARTTLS (587)</option>
                                <option value="implicit_tls">Implicit TLS (465)</option>
                                <option value="none">Plain (no TLS)</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Reply-To (optional)</label>
                            <input v-model="form.reply_to" type="email" placeholder="support@example.com"
                                   class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-white text-gray-900" />
                        </div>
                    </div>

                    <div class="flex items-center gap-4">
                        <label class="flex items-center">
                            <input type="checkbox" v-model="form.enabled" class="w-4 h-4 text-indigo-600 rounded mr-2" />
                            <span class="text-gray-700 text-sm">Enabled</span>
                        </label>
                        <label class="flex items-center">
                            <input type="checkbox" v-model="form.is_default" class="w-4 h-4 text-indigo-600 rounded mr-2" />
                            <span class="text-gray-700 text-sm">Set as default</span>
                        </label>
                        <label class="flex items-center">
                            <input type="checkbox" v-model="form.skip_cert_verify" class="w-4 h-4 text-indigo-600 rounded mr-2" />
                            <span class="text-gray-700 text-sm">Skip cert verify (testing only)</span>
                        </label>
                    </div>
                </div>

                <div class="p-6 border-t border-gray-200 flex justify-end gap-3">
                    <button @click="showAddModal = false" class="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg">
                        Cancel
                    </button>
                    <button @click="saveProvider" :disabled="saving || !form.host"
                            class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg font-medium">
                        {{ saving ? 'Saving...' : (editingProvider ? 'Update' : 'Create') }}
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>
