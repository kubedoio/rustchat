<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import adminEmailApi, {
    type MailProviderResponse,
    type WorkflowResponse,
    type EmailTemplateFamily,
    type TemplateVersionResponse,
    type EmailOutboxResponse,
    type EmailOutboxDetails,
    type EmailEventResponse,
} from '../../api/adminEmail'
import { RefreshCw, Mailbox, ListChecks, FileText, Clock3, Activity, Trash2, Star, Eye } from 'lucide-vue-next'

type TabKey = 'providers' | 'workflows' | 'templates' | 'outbox' | 'events'

const activeTab = ref<TabKey>('providers')
const loading = ref(false)
const error = ref('')
const notice = ref('')

const providers = ref<MailProviderResponse[]>([])
const workflows = ref<WorkflowResponse[]>([])
const templateFamilies = ref<EmailTemplateFamily[]>([])
const templateVersions = ref<TemplateVersionResponse[]>([])
const outbox = ref<EmailOutboxResponse[]>([])
const events = ref<EmailEventResponse[]>([])
const selectedOutbox = ref<EmailOutboxDetails | null>(null)
const selectedFamilyId = ref<string>('')

const outboxFilters = ref({
    status: '',
    workflow_key: '',
    recipient_email: '',
})

const eventFilters = ref({
    workflow_key: '',
    event_type: '',
})

const providerTestStatus = ref<Record<string, string>>({})

const familyForm = ref({
    key: '',
    name: '',
    description: '',
    workflow_key: '',
})
const editingFamilyId = ref<string | null>(null)
const familyEditDraft = ref<Record<string, { name: string; description: string }>>({})

const versionForm = ref({
    locale: 'en',
    subject: '',
    body_text: '',
    body_html: '',
})



const tabs: Array<{ key: TabKey; label: string; icon: any }> = [
    { key: 'providers', label: 'Providers', icon: Mailbox },
    { key: 'workflows', label: 'Workflows', icon: ListChecks },
    { key: 'templates', label: 'Templates', icon: FileText },
    { key: 'outbox', label: 'Outbox', icon: Clock3 },
    { key: 'events', label: 'Events', icon: Activity },
]

function setMessage(msg = '', kind: 'error' | 'notice' = 'notice') {
    if (kind === 'error') {
        error.value = msg
        if (msg) notice.value = ''
    } else {
        notice.value = msg
        if (msg) error.value = ''
    }
}

function extractError(e: any, fallback: string) {
    return e?.response?.data?.error?.message || e?.response?.data?.message || e?.response?.data?.error || e?.message || fallback
}

function formatDate(value?: string | null) {
    if (!value) return '—'
    try {
        return new Date(value).toLocaleString()
    } catch {
        return value
    }
}

const selectedFamily = computed(() => templateFamilies.value.find(f => f.id === selectedFamilyId.value) || null)

watch(selectedFamilyId, async (id) => {
    templateVersions.value = []
    if (!id) return
    try {
        const { data } = await adminEmailApi.listTemplateVersions(id)
        templateVersions.value = data
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to load template versions'), 'error')
    }
})

async function refreshAll() {
    loading.value = true
    setMessage('')
    try {
        const [providersRes, workflowsRes, familiesRes] = await Promise.all([
            adminEmailApi.listProviders(),
            adminEmailApi.listWorkflows(),
            adminEmailApi.listTemplateFamilies(),
        ])
        providers.value = providersRes.data
        workflows.value = workflowsRes.data
        templateFamilies.value = familiesRes.data

        if (!selectedFamilyId.value && templateFamilies.value.length > 0) {
            const firstFamily = templateFamilies.value[0]
            if (firstFamily) {
                selectedFamilyId.value = firstFamily.id
            }
        } else if (selectedFamilyId.value) {
            const { data } = await adminEmailApi.listTemplateVersions(selectedFamilyId.value)
            templateVersions.value = data
        }

        await Promise.all([loadOutbox(), loadEvents()])
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to load email admin data'), 'error')
    } finally {
        loading.value = false
    }
}

async function loadOutbox() {
    const params: any = {}
    if (outboxFilters.value.status) params.status = outboxFilters.value.status
    if (outboxFilters.value.workflow_key) params.workflow_key = outboxFilters.value.workflow_key
    if (outboxFilters.value.recipient_email) params.recipient_email = outboxFilters.value.recipient_email
    const { data } = await adminEmailApi.listOutbox(params)
    outbox.value = data
}

async function loadEvents() {
    const params: any = {}
    if (eventFilters.value.workflow_key) params.workflow_key = eventFilters.value.workflow_key
    if (eventFilters.value.event_type) params.event_type = eventFilters.value.event_type
    const { data } = await adminEmailApi.listEvents(params)
    events.value = data
}

async function deleteProvider(id: string) {
    if (!confirm('Delete this email provider?')) return
    try {
        await adminEmailApi.deleteProvider(id)
        providers.value = providers.value.filter(p => p.id !== id)
        setMessage('Provider deleted')
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to delete provider'), 'error')
    }
}

async function setDefaultProvider(id: string) {
    try {
        const { data } = await adminEmailApi.setDefaultProvider(id)
        providers.value = providers.value.map(p => ({ ...p, is_default: p.id === data.id }))
        setMessage('Default provider updated')
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to set default provider'), 'error')
    }
}

async function testProvider(id: string) {
    // Prompt for test email recipient
    const to = prompt('Enter test recipient email address:')
    if (!to || !to.trim()) return
    
    providerTestStatus.value = { ...providerTestStatus.value, [id]: 'Testing...' }
    try {
        const { data } = await adminEmailApi.testProvider(id, to.trim())
        providerTestStatus.value = {
            ...providerTestStatus.value,
            [id]: data.success
                ? `Success (${data.stage || 'sent'})`
                : `Failed (${data.stage || 'error'}): ${data.error || 'Unknown error'}`,
        }
    } catch (e: any) {
        providerTestStatus.value = { ...providerTestStatus.value, [id]: extractError(e, 'Provider test failed') }
    }
}

function editableFamilyDraft(family: EmailTemplateFamily) {
    if (!familyEditDraft.value[family.id]) {
        familyEditDraft.value[family.id] = {
            name: family.name,
            description: family.description || '',
        }
    }
    return familyEditDraft.value[family.id]!
}

async function createOrUpdateFamily() {
    try {
        if (editingFamilyId.value) {
            const draft = familyEditDraft.value[editingFamilyId.value]
            if (!draft) {
                throw new Error('Template family edit state is missing')
            }
            const { data } = await adminEmailApi.updateTemplateFamily(editingFamilyId.value, {
                name: draft.name,
                description: draft.description || null,
            })
            const idx = templateFamilies.value.findIndex(f => f.id === data.id)
            if (idx >= 0) templateFamilies.value[idx] = data
            editingFamilyId.value = null
            setMessage('Template family updated')
        } else {
            const { data } = await adminEmailApi.createTemplateFamily({
                key: familyForm.value.key.trim(),
                name: familyForm.value.name.trim(),
                description: familyForm.value.description.trim() || null,
                workflow_key: familyForm.value.workflow_key.trim() || null,
            })
            templateFamilies.value.push(data)
            selectedFamilyId.value = data.id
            familyForm.value = { key: '', name: '', description: '', workflow_key: '' }
            setMessage('Template family created')
        }
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to save template family'), 'error')
    }
}

async function removeFamily(family: EmailTemplateFamily) {
    if (!confirm(`Delete template family "${family.name}"?`)) return
    try {
        await adminEmailApi.deleteTemplateFamily(family.id)
        templateFamilies.value = templateFamilies.value.filter(f => f.id !== family.id)
        if (selectedFamilyId.value === family.id) {
            selectedFamilyId.value = templateFamilies.value[0]?.id || ''
        }
        setMessage('Template family deleted')
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to delete template family'), 'error')
    }
}

async function createTemplateVersion() {
    if (!selectedFamilyId.value) return
    try {
        await adminEmailApi.createTemplateVersion(selectedFamilyId.value, {
            locale: versionForm.value.locale.trim() || 'en',
            subject: versionForm.value.subject,
            body_text: versionForm.value.body_text,
            body_html: versionForm.value.body_html,
            variables: [],
            is_compiled_from_mjml: false,
            mjml_source: null,
        })
        const { data } = await adminEmailApi.listTemplateVersions(selectedFamilyId.value)
        templateVersions.value = data
        versionForm.value = { locale: 'en', subject: '', body_text: '', body_html: '' }
        setMessage('Template version created')
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to create template version'), 'error')
    }
}

async function publishVersion(versionId: string) {
    try {
        await adminEmailApi.publishTemplateVersion(versionId)
        if (selectedFamilyId.value) {
            const { data } = await adminEmailApi.listTemplateVersions(selectedFamilyId.value)
            templateVersions.value = data
        }
        setMessage('Template version published')
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to publish template version'), 'error')
    }
}

async function saveWorkflow(wf: WorkflowResponse) {
    try {
        const { data } = await adminEmailApi.updateWorkflow(wf.id, {
            enabled: wf.enabled,
            default_locale: wf.default_locale,
            selected_template_family_id: wf.selected_template_family_id || null,
            policy: wf.policy,
        })
        const idx = workflows.value.findIndex(w => w.id === wf.id)
        if (idx >= 0) workflows.value[idx] = data
        setMessage(`Workflow "${wf.workflow_key}" updated`)
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to update workflow'), 'error')
        await refreshAll()
    }
}

async function refreshOutbox() {
    try {
        await loadOutbox()
        setMessage('Outbox refreshed')
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to load outbox'), 'error')
    }
}

async function viewOutboxDetails(id: string) {
    try {
        const { data } = await adminEmailApi.getOutboxEntry(id)
        selectedOutbox.value = data
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to load outbox entry'), 'error')
    }
}

async function cancelOutbox(id: string) {
    try {
        await adminEmailApi.cancelOutboxEntry(id)
        await loadOutbox()
        setMessage('Outbox entry cancelled')
    } catch (e: any) {
        setMessage(extractError(e, 'Cannot cancel outbox entry'), 'error')
    }
}

async function retryOutbox(id: string) {
    try {
        await adminEmailApi.retryOutboxEntry(id)
        await loadOutbox()
        setMessage('Outbox entry queued for retry')
    } catch (e: any) {
        setMessage(extractError(e, 'Cannot retry outbox entry'), 'error')
    }
}

async function refreshEvents() {
    try {
        await loadEvents()
        setMessage('Email events refreshed')
    } catch (e: any) {
        setMessage(extractError(e, 'Failed to load email events'), 'error')
    }
}



onMounted(() => {
    refreshAll()
})
</script>

<template>
    <div class="space-y-6">
        <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
            <div class="flex items-center justify-between gap-3">
                <div>
                    <h2 class="text-lg font-semibold text-text-1">Email Workflows & Templates</h2>
                    <p class="text-sm text-text-3 mt-1">Manage providers, workflow routing, templates, outbox, and event history.</p>
                </div>
                <button @click="refreshAll" :disabled="loading" class="inline-flex items-center gap-2 px-3 py-2 rounded-lg border border-border-2 text-sm text-text-2">
                    <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': loading }" />
                    {{ loading ? 'Refreshing...' : 'Refresh' }}
                </button>
            </div>

            <div v-if="error" class="mt-4 rounded-lg border border-danger/20 bg-danger/10 px-3 py-2 text-sm text-danger">
                {{ error }}
            </div>
            <div v-else-if="notice" class="mt-4 rounded-lg border border-success/20 bg-success/10 px-3 py-2 text-sm text-success">
                {{ notice }}
            </div>

            <div class="mt-5 flex flex-wrap gap-2">
                <button
                    v-for="tab in tabs"
                    :key="tab.key"
                    @click="activeTab = tab.key"
                    class="inline-flex items-center gap-2 px-3 py-2 rounded-lg text-sm border transition-colors"
                    :class="activeTab === tab.key
                        ? 'bg-brand text-white border-brand'
                        : 'border-border-2 text-text-2 hover:bg-bg-surface-2'"
                >
                    <component :is="tab.icon" class="w-4 h-4" />
                    {{ tab.label }}
                </button>
            </div>
        </div>

        <div v-if="activeTab === 'providers'" class="space-y-6">
            <!-- Note: Provider creation/editing is handled in EmailSettings.vue -->
            <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
                <div class="overflow-x-auto">
                    <table class="min-w-full text-sm">
                        <thead class="text-left text-text-3">
                            <tr>
                                <th class="py-2 pr-4">Provider</th>
                                <th class="py-2 pr-4">Sender</th>
                                <th class="py-2 pr-4">Status</th>
                                <th class="py-2 pr-4">Test</th>
                                <th class="py-2 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="provider in providers" :key="provider.id" class="border-t border-border-1">
                                <td class="py-3 pr-4">
                                    <div class="font-medium text-text-1">{{ provider.host }}:{{ provider.port }}</div>
                                    <div class="text-xs text-text-3">{{ provider.tls_mode }} · {{ provider.username || 'no auth' }}</div>
                                </td>
                                <td class="py-3 pr-4">
                                    <div>{{ provider.from_name }}</div>
                                    <div class="text-xs text-text-3">{{ provider.from_address }}</div>
                                </td>
                                <td class="py-3 pr-4">
                                    <span class="px-2 py-1 rounded text-xs" :class="provider.enabled ? 'bg-success/10 text-success' : 'bg-bg-surface-2 text-text-2'">
                                        {{ provider.enabled ? 'Enabled' : 'Disabled' }}
                                    </span>
                                    <span v-if="provider.is_default" class="ml-2 px-2 py-1 rounded text-xs bg-warning/10 text-warning">Default</span>
                                </td>
                                <td class="py-3 pr-4">
                                    <div class="text-xs text-text-3 min-h-4">{{ providerTestStatus[provider.id] || '—' }}</div>
                                </td>
                                <td class="py-3 text-right">
                                    <div class="inline-flex items-center gap-2">
                                        <button @click="testProvider(provider.id)" class="px-2 py-1 rounded border border-border-2 text-xs">Test</button>
                                        <button @click="setDefaultProvider(provider.id)" class="px-2 py-1 rounded border border-border-2 text-xs"><Star class="w-3 h-3 inline mr-1" />Default</button>
                                        <button @click="deleteProvider(provider.id)" class="px-2 py-1 rounded border border-danger/30 text-danger text-xs"><Trash2 class="w-3 h-3 inline mr-1" />Delete</button>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

        </div>

        <div v-else-if="activeTab === 'workflows'" class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
            <div class="overflow-x-auto">
                <table class="min-w-full text-sm">
                    <thead class="text-left text-text-3">
                        <tr>
                            <th class="py-2 pr-4">Workflow</th>
                            <th class="py-2 pr-4">Enabled</th>
                            <th class="py-2 pr-4">Locale</th>
                            <th class="py-2 pr-4">Template Family</th>
                            <th class="py-2 pr-4">Policy</th>
                            <th class="py-2 text-right">Save</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="wf in workflows" :key="wf.id" class="border-t border-border-1">
                            <td class="py-3 pr-4">
                                <div class="font-medium text-text-1">{{ wf.name }}</div>
                                <div class="text-xs text-text-3">{{ wf.workflow_key }} · {{ wf.category }}</div>
                            </td>
                            <td class="py-3 pr-4">
                                <label class="inline-flex items-center gap-2">
                                    <input v-model="wf.enabled" type="checkbox" :disabled="!wf.can_disable" class="w-4 h-4 rounded" />
                                    <span class="text-xs text-text-3">{{ wf.can_disable ? 'Editable' : 'Required' }}</span>
                                </label>
                            </td>
                            <td class="py-3 pr-4">
                                <input v-model="wf.default_locale" class="w-20 px-2 py-1 rounded border border-border-2 bg-bg-surface-1" />
                            </td>
                            <td class="py-3 pr-4">
                                <select v-model="wf.selected_template_family_id" class="w-56 px-2 py-1 rounded border border-border-2 bg-bg-surface-1">
                                    <option :value="null">Default / none</option>
                                    <option v-for="f in templateFamilies" :key="f.id" :value="f.id">{{ f.key }}</option>
                                </select>
                            </td>
                            <td class="py-3 pr-4">
                                <details class="max-w-sm">
                                    <summary class="cursor-pointer text-xs text-brand">View policy</summary>
                                    <pre class="mt-2 p-2 rounded bg-bg-surface-2 text-text-1 text-xs overflow-auto">{{ JSON.stringify(wf.policy, null, 2) }}</pre>
                                </details>
                            </td>
                            <td class="py-3 text-right">
                                <button @click="saveWorkflow(wf)" class="px-3 py-1 rounded bg-brand hover:bg-brand/80 text-white text-xs">Save</button>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>

        <div v-else-if="activeTab === 'templates'" class="space-y-6">
            <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
                <div class="flex items-center justify-between">
                    <h3 class="text-base font-semibold text-text-1">Template Families</h3>
                    <button @click="editingFamilyId = null" class="text-xs text-text-3">Clear edit state</button>
                </div>
                <div class="mt-4 grid grid-cols-1 md:grid-cols-4 gap-3">
                    <input v-model="familyForm.key" placeholder="family_key" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <input v-model="familyForm.name" placeholder="Display name" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <input v-model="familyForm.workflow_key" placeholder="workflow_key (optional)" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <button @click="createOrUpdateFamily" class="inline-flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-brand hover:bg-brand/80 text-white">
                        <Plus class="w-4 h-4" /> Create Family
                    </button>
                </div>
                <textarea v-model="familyForm.description" rows="2" placeholder="Description" class="mt-3 w-full px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />

                <div class="mt-4 overflow-x-auto">
                    <table class="min-w-full text-sm">
                        <thead class="text-left text-text-3">
                            <tr>
                                <th class="py-2 pr-4">Select</th>
                                <th class="py-2 pr-4">Key</th>
                                <th class="py-2 pr-4">Name</th>
                                <th class="py-2 pr-4">Workflow</th>
                                <th class="py-2 pr-4">Type</th>
                                <th class="py-2 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="family in templateFamilies" :key="family.id" class="border-t border-border-1">
                                <td class="py-3 pr-4">
                                    <input type="radio" name="selectedFamily" :value="family.id" v-model="selectedFamilyId" />
                                </td>
                                <td class="py-3 pr-4 font-mono text-xs">{{ family.key }}</td>
                                <td class="py-3 pr-4">
                                    <template v-if="editingFamilyId === family.id && !family.is_system">
                                        <input v-model="editableFamilyDraft(family).name" class="w-full px-2 py-1 rounded border border-border-2 bg-bg-surface-1" />
                                        <textarea v-model="editableFamilyDraft(family).description" rows="2" class="mt-2 w-full px-2 py-1 rounded border border-border-2 bg-bg-surface-1" />
                                    </template>
                                    <template v-else>
                                        <div class="font-medium text-text-1">{{ family.name }}</div>
                                        <div class="text-xs text-text-3">{{ family.description || '—' }}</div>
                                    </template>
                                </td>
                                <td class="py-3 pr-4 text-xs">{{ family.workflow_key || '—' }}</td>
                                <td class="py-3 pr-4">
                                    <span class="px-2 py-1 rounded text-xs" :class="family.is_system ? 'bg-bg-surface-2 text-text-2' : 'bg-brand/10 text-brand'">
                                        {{ family.is_system ? 'System' : 'Custom' }}
                                    </span>
                                </td>
                                <td class="py-3 text-right">
                                    <div class="inline-flex gap-2">
                                        <button v-if="!family.is_system && editingFamilyId !== family.id" @click="editingFamilyId = family.id" class="px-2 py-1 rounded border border-border-2 text-xs">Edit</button>
                                        <button v-if="!family.is_system && editingFamilyId === family.id" @click="createOrUpdateFamily" class="px-2 py-1 rounded bg-brand text-white text-xs">Save</button>
                                        <button v-if="!family.is_system && editingFamilyId === family.id" @click="editingFamilyId = null" class="px-2 py-1 rounded border border-border-2 text-xs">Cancel</button>
                                        <button v-if="!family.is_system" @click="removeFamily(family)" class="px-2 py-1 rounded border border-danger/30 text-danger text-xs">Delete</button>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5" v-if="selectedFamily">
                <h3 class="text-base font-semibold text-text-1">Template Versions: {{ selectedFamily.key }}</h3>
                <div class="mt-4 grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm mb-1 text-text-2">Locale</label>
                        <input v-model="versionForm.locale" class="w-full px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    </div>
                    <div>
                        <label class="block text-sm mb-1 text-text-2">Subject</label>
                        <input v-model="versionForm.subject" class="w-full px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    </div>
                </div>
                <div class="mt-4 grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm mb-1 text-text-2">Body (text)</label>
                        <textarea v-model="versionForm.body_text" rows="6" class="w-full px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    </div>
                    <div>
                        <label class="block text-sm mb-1 text-text-2">Body (HTML)</label>
                        <textarea v-model="versionForm.body_html" rows="6" class="w-full px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    </div>
                </div>
                <div class="mt-4">
                    <button @click="createTemplateVersion" class="inline-flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-brand hover:bg-brand/80 text-white">
                        <Plus class="w-4 h-4" />
                        Create Version
                    </button>
                </div>

                <div class="mt-5 overflow-x-auto">
                    <table class="min-w-full text-sm">
                        <thead class="text-left text-text-3">
                            <tr>
                                <th class="py-2 pr-4">Version</th>
                                <th class="py-2 pr-4">Status</th>
                                <th class="py-2 pr-4">Locale</th>
                                <th class="py-2 pr-4">Subject</th>
                                <th class="py-2 pr-4">Created</th>
                                <th class="py-2 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="version in templateVersions" :key="version.id" class="border-t border-border-1">
                                <td class="py-3 pr-4 font-medium">v{{ version.version }}</td>
                                <td class="py-3 pr-4">{{ version.status }}</td>
                                <td class="py-3 pr-4">{{ version.locale }}</td>
                                <td class="py-3 pr-4">{{ version.subject }}</td>
                                <td class="py-3 pr-4 text-xs">{{ formatDate(version.created_at) }}</td>
                                <td class="py-3 text-right">
                                    <button v-if="version.status !== 'published'" @click="publishVersion(version.id)" class="px-3 py-1 rounded bg-brand hover:bg-brand/80 text-white text-xs">Publish</button>
                                </td>
                            </tr>
                            <tr v-if="templateVersions.length === 0">
                                <td colspan="6" class="py-4 text-sm text-text-3">No versions yet for this family.</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>

        <div v-else-if="activeTab === 'outbox'" class="space-y-6">
            <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                    <input v-model="outboxFilters.status" placeholder="status (queued/failed/...)" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <input v-model="outboxFilters.workflow_key" placeholder="workflow_key" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <input v-model="outboxFilters.recipient_email" placeholder="recipient email" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <button @click="refreshOutbox" class="inline-flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-brand hover:bg-brand/80 text-white">
                        <RefreshCw class="w-4 h-4" /> Refresh
                    </button>
                </div>
            </div>
            <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
                <div class="overflow-x-auto">
                    <table class="min-w-full text-sm">
                        <thead class="text-left text-text-3">
                            <tr>
                                <th class="py-2 pr-4">Recipient</th>
                                <th class="py-2 pr-4">Workflow</th>
                                <th class="py-2 pr-4">Status</th>
                                <th class="py-2 pr-4">Attempts</th>
                                <th class="py-2 pr-4">Created</th>
                                <th class="py-2 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="entry in outbox" :key="entry.id" class="border-t border-border-1">
                                <td class="py-3 pr-4">
                                    <div class="font-medium">{{ entry.recipient_email }}</div>
                                    <div class="text-xs text-text-3 truncate max-w-[20rem]">{{ entry.subject }}</div>
                                </td>
                                <td class="py-3 pr-4 text-xs">{{ entry.workflow_key || '—' }}</td>
                                <td class="py-3 pr-4">{{ entry.status }}</td>
                                <td class="py-3 pr-4">{{ entry.attempt_count }}/{{ entry.max_attempts }}</td>
                                <td class="py-3 pr-4 text-xs">{{ formatDate(entry.created_at) }}</td>
                                <td class="py-3 text-right">
                                    <div class="inline-flex gap-2">
                                        <button @click="viewOutboxDetails(entry.id)" class="px-2 py-1 rounded border border-border-2 text-xs"><Eye class="w-3 h-3 inline mr-1" />View</button>
                                        <button v-if="entry.status === 'queued'" @click="cancelOutbox(entry.id)" class="px-2 py-1 rounded border border-danger/30 text-danger text-xs">Cancel</button>
                                        <button v-if="entry.status === 'failed'" @click="retryOutbox(entry.id)" class="px-2 py-1 rounded border border-brand/30 text-brand text-xs">Retry</button>
                                    </div>
                                </td>
                            </tr>
                            <tr v-if="outbox.length === 0">
                                <td colspan="6" class="py-4 text-sm text-text-3">No outbox entries found.</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                <div v-if="selectedOutbox" class="mt-4 rounded-lg border border-border-1 p-4">
                    <div class="flex items-center justify-between">
                        <h4 class="font-semibold text-text-1">Outbox Entry Details</h4>
                        <button @click="selectedOutbox = null" class="text-xs text-text-3">Close</button>
                    </div>
                    <pre class="mt-3 p-3 rounded-lg bg-bg-surface-2 text-text-1 text-xs overflow-auto">{{ JSON.stringify(selectedOutbox, null, 2) }}</pre>
                </div>
            </div>
        </div>

        <div v-else-if="activeTab === 'events'" class="space-y-6">
            <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <input v-model="eventFilters.workflow_key" placeholder="workflow_key" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <input v-model="eventFilters.event_type" placeholder="event_type" class="px-3 py-2 rounded-lg border border-border-2 bg-bg-surface-1" />
                    <button @click="refreshEvents" class="inline-flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-brand hover:bg-brand/80 text-white">
                        <RefreshCw class="w-4 h-4" /> Refresh
                    </button>
                </div>
            </div>
            <div class="rounded-xl border border-border-1 bg-bg-surface-1 p-5">
                <div class="overflow-x-auto">
                    <table class="min-w-full text-sm">
                        <thead class="text-left text-text-3">
                            <tr>
                                <th class="py-2 pr-4">Time</th>
                                <th class="py-2 pr-4">Event</th>
                                <th class="py-2 pr-4">Workflow</th>
                                <th class="py-2 pr-4">Recipient</th>
                                <th class="py-2 pr-4">Status</th>
                                <th class="py-2 pr-4">Error</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="event in events" :key="event.id" class="border-t border-border-1">
                                <td class="py-3 pr-4 text-xs">{{ formatDate(event.created_at) }}</td>
                                <td class="py-3 pr-4">{{ event.event_type }}</td>
                                <td class="py-3 pr-4 text-xs">{{ event.workflow_key || '—' }}</td>
                                <td class="py-3 pr-4">{{ event.recipient_email }}</td>
                                <td class="py-3 pr-4 text-xs">{{ event.status_code ?? '—' }}</td>
                                <td class="py-3 pr-4 text-xs text-danger">{{ event.error_category || event.error_message || '—' }}</td>
                            </tr>
                            <tr v-if="events.length === 0">
                                <td colspan="6" class="py-4 text-sm text-text-3">No email events found.</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>
</template>
