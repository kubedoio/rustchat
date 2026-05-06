<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useToast } from '../../composables/useToast'
import { adminApi, type SsoConfig, type CreateSsoConfigRequest, type AuthConfig } from '../../api/admin'
import { Shield, Plus, Edit2, Trash2, TestTube, AlertCircle, CheckCircle, HelpCircle } from 'lucide-vue-next'

const toast = useToast()

// State
const ssoConfigs = ref<SsoConfig[]>([])
const authConfig = ref<AuthConfig | null>(null)
const loading = ref(false)
const showAddModal = ref(false)
const showEditModal = ref(false)
const showTestModal = ref(false)
const testingConfig = ref<SsoConfig | null>(null)
const testResult = ref<{ success: boolean; message: string; details: any } | null>(null)
const editingConfig = ref<SsoConfig | null>(null)
const siteUrl = ref('')

// Form state for creating/editing
interface FormState {
  id?: string
  provider_type: 'github' | 'google' | 'oidc'
  provider_key: string
  display_name: string
  issuer_url: string
  client_id: string
  client_secret: string
  scopes: string[]
  is_active: boolean
  auto_provision: boolean
  default_role: string
  allow_domains: string[]
  github_org: string
  github_team: string
  groups_claim: string
  role_mappings: Record<string, string>
}

const form = ref<FormState>({
  provider_type: 'oidc',
  provider_key: '',
  display_name: '',
  issuer_url: '',
  client_id: '',
  client_secret: '',
  scopes: [],
  is_active: true,
  auto_provision: true,
  default_role: 'member',
  allow_domains: [],
  github_org: '',
  github_team: '',
  groups_claim: 'groups',
  role_mappings: {},
})

const defaultScopes = {
  github: ['read:user', 'user:email'],
  google: ['openid', 'profile', 'email'],
  oidc: ['openid', 'profile', 'email'],
}

// Computed
const callbackUrl = computed(() => {
  return `${siteUrl.value}/api/v1/oauth2/${form.value.provider_key}/callback`
})

const isOidc = computed(() => form.value.provider_type === 'oidc' || form.value.provider_type === 'google')
const isGithub = computed(() => form.value.provider_type === 'github')

// Load data
onMounted(async () => {
  await Promise.all([loadSsoConfigs(), loadAuthConfig(), loadSiteUrl()])
})

async function loadSsoConfigs() {
  try {
    const response = await adminApi.getSsoConfigs()
    ssoConfigs.value = response.data
  } catch (error) {
    toast.error('Failed to load SSO configurations')
  }
}

async function loadAuthConfig() {
  try {
    const response = await adminApi.getConfig()
    authConfig.value = response.data.authentication
  } catch (error) {
    console.error('Failed to load auth config', error)
  }
}

async function loadSiteUrl() {
  try {
    const response = await adminApi.getConfig()
    siteUrl.value = response.data.site.site_url || window.location.origin
  } catch {
    siteUrl.value = window.location.origin
  }
}

async function updateAuthSettings() {
  if (!authConfig.value) return
  
  try {
    await adminApi.updateConfig('authentication', authConfig.value)
    toast.success('Authentication settings updated')
  } catch (error) {
    toast.error('Failed to update authentication settings')
  }
}

// Modal handlers
function openAddModal() {
  resetForm()
  showAddModal.value = true
}

function openEditModal(config: SsoConfig) {
  editingConfig.value = config
  // Handle SAML configs by defaulting to oidc for editing (SAML not editable via this UI)
  const editableType: 'github' | 'google' | 'oidc' = config.provider_type === 'saml' ? 'oidc' : config.provider_type as 'github' | 'google' | 'oidc'
  form.value = {
    id: config.id,
    provider_type: editableType,
    provider_key: config.provider_key,
    display_name: config.display_name || '',
    issuer_url: config.issuer_url || '',
    client_id: config.client_id || '',
    client_secret: '', // Don't populate secret
    scopes: config.scopes?.length ? config.scopes : defaultScopes[editableType],
    is_active: config.is_active,
    auto_provision: config.auto_provision,
    default_role: config.default_role || 'member',
    allow_domains: config.allow_domains || [],
    github_org: config.github_org || '',
    github_team: config.github_team || '',
    groups_claim: config.groups_claim || 'groups',
    role_mappings: (config.role_mappings as Record<string, string>) || {},
  }
  showEditModal.value = true
}

function resetForm() {
  const type: 'github' | 'google' | 'oidc' = 'oidc'
  form.value = {
    provider_type: type,
    provider_key: '',
    display_name: '',
    issuer_url: '',
    client_id: '',
    client_secret: '',
    scopes: [...defaultScopes[type]],
    is_active: true,
    auto_provision: true,
    default_role: 'member',
    allow_domains: [],
    github_org: '',
    github_team: '',
    groups_claim: 'groups',
    role_mappings: {},
  }
  editingConfig.value = null
}

function onProviderTypeChange() {
  const type: 'github' | 'google' | 'oidc' = form.value.provider_type
  form.value.scopes = [...defaultScopes[type]]
  
  // Set default display name
  if (!form.value.display_name) {
    const defaults: Record<string, string> = {
      github: 'GitHub',
      google: 'Google',
      oidc: 'SSO',
    }
    form.value.display_name = defaults[type] || 'SSO'
  }
}

async function saveConfig() {
  loading.value = true
  try {
    const payload: CreateSsoConfigRequest = {
      provider_key: form.value.provider_key,
      provider_type: form.value.provider_type,
      display_name: form.value.display_name || undefined,
      issuer_url: form.value.issuer_url || undefined,
      client_id: form.value.client_id || undefined,
      client_secret: form.value.client_secret || undefined,
      scopes: form.value.scopes,
      is_active: form.value.is_active,
      auto_provision: form.value.auto_provision,
      default_role: form.value.default_role || undefined,
      allow_domains: form.value.allow_domains?.filter(Boolean) ?? undefined,
      github_org: form.value.github_org || undefined,
      github_team: form.value.github_team || undefined,
      groups_claim: form.value.groups_claim || undefined,
      role_mappings: Object.keys(form.value.role_mappings).length > 0 ? form.value.role_mappings : undefined,
    }

    // Remove empty strings for optional fields
    if (!payload.github_org) delete payload.github_org
    if (!payload.github_team) delete payload.github_team
    if (!payload.groups_claim) delete payload.groups_claim
    if (!payload.issuer_url) delete payload.issuer_url

    if (editingConfig.value) {
      // For updates, we use update API
      const updatePayload: any = { ...payload }
      delete updatePayload.provider_type // Can't change type on edit
      delete updatePayload.provider_key // Can't change key on edit
      
      // Only include client_secret if provided
      if (!updatePayload.client_secret) {
        delete updatePayload.client_secret
      }
      
      await adminApi.updateSsoConfig(editingConfig.value.id, updatePayload)
      toast.success('SSO configuration updated')
      showEditModal.value = false
    } else {
      await adminApi.createSsoConfig(payload)
      toast.success('SSO configuration created')
      showAddModal.value = false
    }
    
    await loadSsoConfigs()
  } catch (error: any) {
    const message = error.response?.data?.error || 'Failed to save SSO configuration'
    toast.error(message)
  } finally {
    loading.value = false
  }
}

async function deleteConfig(config: SsoConfig) {
  if (!confirm(`Are you sure you want to delete the "${config.display_name || config.provider_key}" configuration?`)) {
    return
  }
  
  try {
    await adminApi.deleteSsoConfig(config.id)
    toast.success('SSO configuration deleted')
    await loadSsoConfigs()
  } catch (error) {
    toast.error('Failed to delete SSO configuration')
  }
}

async function testConfig(config: SsoConfig) {
  testingConfig.value = config
  testResult.value = null
  showTestModal.value = true
  
  try {
    const response = await adminApi.testSsoConfig(config.id)
    testResult.value = response.data
  } catch (error: any) {
    testResult.value = {
      success: false,
      message: error.response?.data?.error || 'Test failed',
      details: null,
    }
  }
}

function getProviderIcon(type: string) {
  const icons: Record<string, string> = {
    github: 'GitHub',
    google: 'Google',
    oidc: 'SSO',
    saml: 'SAML',
  }
  return icons[type] || type
}

function getProviderBadgeClass(type: string) {
  const classes: Record<string, string> = {
    github: 'bg-bg-surface-2 text-text-1',
    google: 'bg-brand text-white',
    oidc: 'bg-primary text-white',
    saml: 'bg-warning text-white',
  }
  return classes[type] || 'bg-text-3 text-white'
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold text-text-1 flex items-center gap-2">
          <Shield class="w-6 h-6 text-brand" />
          Single Sign-On (SSO)
        </h1>
        <p class="text-text-3 mt-1">
          Configure OAuth2 and OIDC authentication providers
        </p>
      </div>
      <button
        @click="openAddModal"
        class="inline-flex items-center px-4 py-2 bg-brand text-white rounded-lg hover:bg-brand/90 transition-colors"
      >
        <Plus class="w-4 h-4 mr-2" />
        Add Provider
      </button>
    </div>

    <!-- Global SSO Settings -->
    <div v-if="authConfig" class="bg-bg-surface-1 rounded-lg shadow p-6">
      <h2 class="text-lg font-semibold text-text-1 mb-4">Global SSO Settings</h2>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <label class="flex items-center justify-between p-4 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <div>
            <div class="font-medium text-text-1">Enable SSO</div>
            <div class="text-sm text-text-3">Allow users to sign in with configured providers</div>
          </div>
          <input
            v-model="authConfig.enable_sso"
            @change="updateAuthSettings"
            type="checkbox"
            class="w-5 h-5 text-brand rounded focus:ring-brand"
          />
        </label>

        <label class="flex items-center justify-between p-4 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <div>
            <div class="font-medium text-text-1">Require SSO</div>
            <div class="text-sm text-text-3">Disable password login, SSO only</div>
          </div>
          <input
            v-model="authConfig.require_sso"
            @change="updateAuthSettings"
            type="checkbox"
            class="w-5 h-5 text-brand rounded focus:ring-brand"
          />
        </label>
      </div>
    </div>

    <!-- Provider List -->
    <div class="bg-bg-surface-1 rounded-lg shadow">
      <div class="p-6 border-b border-border-1">
        <h2 class="text-lg font-semibold text-text-1">Configured Providers</h2>
      </div>

      <div v-if="ssoConfigs.length === 0" class="p-8 text-center text-text-3">
        <Shield class="w-12 h-12 mx-auto mb-4 opacity-50" />
        <p>No SSO providers configured yet.</p>
        <button
          @click="openAddModal"
          class="mt-4 text-brand hover:text-brand/80 font-medium"
        >
          Add your first provider
        </button>
      </div>

      <div v-else class="divide-y divide-border-1">
        <div
          v-for="config in ssoConfigs"
          :key="config.id"
          class="p-6 hover:bg-bg-surface-2 transition-colors"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-4">
              <span
                class="px-2 py-1 text-xs font-semibold rounded"
                :class="getProviderBadgeClass(config.provider_type)"
              >
                {{ getProviderIcon(config.provider_type) }}
              </span>
              <div>
                <h3 class="font-semibold text-text-1">
                  {{ config.display_name || config.provider_key }}
                </h3>
                <p class="text-sm text-text-3">
                  Key: {{ config.provider_key }}
                  <span v-if="config.issuer_url" class="ml-2">• {{ config.issuer_url }}</span>
                </p>
                <div class="flex items-center gap-3 mt-1 text-sm">
                  <span
                    :class="config.is_active ? 'text-success' : 'text-text-3'"
                    class="flex items-center gap-1"
                  >
                    <span
                      class="w-2 h-2 rounded-full"
                      :class="config.is_active ? 'bg-success' : 'bg-text-4'"
                    />
                    {{ config.is_active ? 'Active' : 'Inactive' }}
                  </span>
                  <span class="text-text-4">|</span>
                  <span class="text-text-2">
                    Auto-provision: {{ config.auto_provision ? 'On' : 'Off' }}
                  </span>
                  <span v-if="config.allow_domains?.length" class="text-text-4">|</span>
                  <span v-if="config.allow_domains?.length" class="text-text-2">
                    Domains: {{ config.allow_domains.join(', ') }}
                  </span>
                  <span v-if="config.github_org" class="text-text-4">|</span>
                  <span v-if="config.github_org" class="text-text-2">
                    Org: {{ config.github_org }}
                    <span v-if="config.github_team">/{{ config.github_team }}</span>
                  </span>
                </div>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <button
                @click="testConfig(config)"
                class="p-2 text-text-2 hover:text-brand hover:bg-brand/10 rounded-lg transition-colors"
                title="Test Configuration"
              >
                <TestTube class="w-5 h-5" />
              </button>
              <button
                @click="openEditModal(config)"
                class="p-2 text-text-2 hover:text-brand hover:bg-brand/10 rounded-lg transition-colors"
                title="Edit"
              >
                <Edit2 class="w-5 h-5" />
              </button>
              <button
                @click="deleteConfig(config)"
                class="p-2 text-text-2 hover:text-danger hover:bg-danger/10 rounded-lg transition-colors"
                title="Delete"
              >
                <Trash2 class="w-5 h-5" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Add/Edit Modal -->
    <div
      v-if="showAddModal || showEditModal"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      @click.self="showAddModal = false; showEditModal = false"
    >
      <div class="bg-bg-surface-1 rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div class="p-6 border-b border-border-1">
          <h2 class="text-xl font-semibold text-text-1">
            {{ editingConfig ? 'Edit Provider' : 'Add Provider' }}
          </h2>
        </div>

        <div class="p-6 space-y-6">
          <!-- Provider Type -->
          <div v-if="!editingConfig">
            <label class="block text-sm font-medium text-text-2 mb-2">
              Provider Type *
            </label>
            <select
              v-model="form.provider_type"
              @change="onProviderTypeChange"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
            >
              <option value="github">GitHub (OAuth2)</option>
              <option value="google">Google (OIDC)</option>
              <option value="oidc">Generic OIDC (Keycloak, ZITADEL, Authentik, etc.)</option>
            </select>
          </div>

          <!-- Provider Key -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-2">
              Provider Key *
              <span class="text-text-4 font-normal ml-1">(used in URLs, e.g., "github", "oidc-keycloak")</span>
            </label>
            <input
              v-model="form.provider_key"
              :disabled="!!editingConfig"
              type="text"
              placeholder="e.g., github"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 disabled:opacity-50"
            />
            <p class="text-xs text-text-3 mt-1">
              Lowercase letters, numbers, and hyphens only. Cannot be changed after creation.
            </p>
          </div>

          <!-- Display Name -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-2">
              Display Name
            </label>
            <input
              v-model="form.display_name"
              type="text"
              :placeholder="form.provider_type === 'github' ? 'GitHub' : form.provider_type === 'google' ? 'Google' : 'Single Sign-On'"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
            />
          </div>

          <!-- OIDC-specific: Issuer URL -->
          <div v-if="isOidc">
            <label class="block text-sm font-medium text-text-2 mb-2">
              Issuer URL *
              <span class="text-text-4 font-normal ml-1">(e.g., https://accounts.google.com)</span>
            </label>
            <input
              v-model="form.issuer_url"
              type="url"
              placeholder="https://"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
            />
          </div>

          <!-- Client ID -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-2">
              Client ID *
            </label>
            <input
              v-model="form.client_id"
              type="text"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
            />
          </div>

          <!-- Client Secret -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-2">
              Client Secret {{ editingConfig ? '(leave blank to keep current)' : '*' }}
            </label>
            <input
              v-model="form.client_secret"
              type="password"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
            />
          </div>

          <!-- Scopes -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-2">
              Scopes
            </label>
            <div class="flex flex-wrap gap-2 mb-2">
              <span
                v-for="(scope, index) in form.scopes"
                :key="index"
                class="inline-flex items-center px-2 py-1 bg-brand/10 text-brand rounded text-sm"
              >
                {{ scope }}
                <button
                  @click="form.scopes.splice(index, 1)"
                  class="ml-1 hover:text-brand/80"
                >
                  ×
                </button>
              </span>
            </div>
            <input
              type="text"
              placeholder="Add scope and press Enter"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
              @keydown.enter.prevent="($event.target as HTMLInputElement).value && (form.scopes || []).push(($event.target as HTMLInputElement).value); ($event.target as HTMLInputElement).value = ''"
            />
          </div>

          <!-- Google-specific: Allowed Domains -->
          <div v-if="form.provider_type === 'google'">
            <label class="block text-sm font-medium text-text-2 mb-2">
              Allowed Domains
              <span class="text-text-4 font-normal ml-1">(optional, restrict to specific email domains)</span>
            </label>
            <div class="flex flex-wrap gap-2 mb-2">
              <span
                v-for="(domain, index) in (form.allow_domains || [])"
                :key="index"
                class="inline-flex items-center px-2 py-1 bg-success/10 text-success rounded text-sm"
              >
                {{ domain }}
                <button
                  @click="(form.allow_domains || []).splice(index, 1)"
                  class="ml-1 hover:text-success/80"
                >
                  ×
                </button>
              </span>
            </div>
            <input
              type="text"
              placeholder="e.g., company.com"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
              @keydown.enter.prevent="($event.target as HTMLInputElement).value && ((form.allow_domains || (form.allow_domains = [])).push(($event.target as HTMLInputElement).value)); ($event.target as HTMLInputElement).value = ''"
            />
          </div>

          <!-- GitHub-specific: Org/Team restrictions -->
          <div v-if="isGithub" class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-text-2 mb-2">
                Required Organization
                <span class="text-text-4 font-normal ml-1">(optional)</span>
              </label>
              <input
                v-model="form.github_org"
                type="text"
                placeholder="e.g., myorg"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-text-2 mb-2">
                Required Team
                <span class="text-text-4 font-normal ml-1">(optional, within the organization)</span>
              </label>
              <input
                v-model="form.github_team"
                type="text"
                placeholder="e.g., developers"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
              />
            </div>
          </div>

          <!-- OIDC-specific: Groups claim and role mappings -->
          <div v-if="isOidc" class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-text-2 mb-2">
                Groups Claim
                <span class="text-text-4 font-normal ml-1">(claim name in ID token containing user groups)</span>
              </label>
              <input
                v-model="form.groups_claim"
                type="text"
                placeholder="groups"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
              />
            </div>
          </div>

          <!-- Settings -->
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer">
              <input
                v-model="form.is_active"
                type="checkbox"
                class="w-5 h-5 text-brand rounded"
              />
              <div>
                <div class="font-medium text-text-1">Active</div>
                <div class="text-xs text-text-3">Show on login page</div>
              </div>
            </label>

            <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer">
              <input
                v-model="form.auto_provision"
                type="checkbox"
                class="w-5 h-5 text-brand rounded"
              />
              <div>
                <div class="font-medium text-text-1">Auto-Provision</div>
                <div class="text-xs text-text-3">Create new users automatically</div>
              </div>
            </label>

            <div class="p-3 border border-border-1 rounded-lg">
              <label class="block text-sm font-medium text-text-2 mb-1">
                Default Role
              </label>
              <select
                v-model="form.default_role"
                class="w-full px-2 py-1 border border-border-2 rounded bg-bg-surface-1 text-text-1 text-sm"
              >
                <option value="member">Member</option>
                <option value="team_admin">Team Admin</option>
                <option value="org_admin">Org Admin</option>
              </select>
            </div>
          </div>

          <!-- Callback URL Info -->
          <div class="bg-brand/10 border border-brand/20 rounded-lg p-4">
            <div class="flex items-start gap-3">
              <HelpCircle class="w-5 h-5 text-brand flex-shrink-0 mt-0.5" />
              <div>
                <h4 class="font-medium text-brand">Callback URL</h4>
                <p class="text-sm text-brand/80 mt-1">
                  Configure this redirect URL in your {{ form.provider_type === 'github' ? 'GitHub OAuth app' : form.provider_type === 'google' ? 'Google Cloud Console' : 'OIDC provider' }}:
                </p>
                <code class="block mt-2 px-3 py-2 bg-brand/20 rounded text-sm text-brand break-all">
                  {{ callbackUrl }}
                </code>
              </div>
            </div>
          </div>
        </div>

        <div class="p-6 border-t border-border-1 flex justify-end gap-3">
          <button
            @click="showAddModal = false; showEditModal = false"
            class="px-4 py-2 text-text-2 hover:bg-bg-surface-2 rounded-lg transition-colors"
          >
            Cancel
          </button>
          <button
            @click="saveConfig"
            :disabled="loading"
            class="px-4 py-2 bg-brand text-white rounded-lg hover:bg-brand/90 disabled:opacity-50 transition-colors"
          >
            {{ loading ? 'Saving...' : (editingConfig ? 'Update' : 'Create') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Test Result Modal -->
    <div
      v-if="showTestModal"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      @click.self="showTestModal = false"
    >
      <div class="bg-bg-surface-1 rounded-lg shadow-xl max-w-lg w-full">
        <div class="p-6 border-b border-border-1">
          <h2 class="text-xl font-semibold text-text-1">
            Test {{ testingConfig?.display_name || testingConfig?.provider_key }}
          </h2>
        </div>

        <div class="p-6">
          <div v-if="!testResult" class="flex items-center justify-center py-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-brand"></div>
            <span class="ml-3 text-text-2">Testing configuration...</span>
          </div>

          <div v-else>
            <div
              class="flex items-center gap-3 p-4 rounded-lg mb-4"
              :class="testResult.success ? 'bg-success/10 text-success' : 'bg-danger/10 text-danger'"
            >
              <CheckCircle v-if="testResult.success" class="w-6 h-6" />
              <AlertCircle v-else class="w-6 h-6" />
              <span class="font-medium">{{ testResult.message }}</span>
            </div>

            <div v-if="testResult.details" class="bg-bg-surface-2 rounded-lg p-4 overflow-auto max-h-64">
              <pre class="text-sm text-text-2">{{ JSON.stringify(testResult.details, null, 2) }}</pre>
            </div>
          </div>
        </div>

        <div class="p-6 border-t border-border-1 flex justify-end">
          <button
            @click="showTestModal = false"
            class="px-4 py-2 bg-bg-surface-2 text-text-2 rounded-lg hover:bg-bg-surface-2/80 transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
