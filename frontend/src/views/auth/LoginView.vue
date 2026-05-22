<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAuthStore } from '../../features/auth/stores/authStore'
import AuthLayout from '../../layouts/AuthLayout.vue'
import BaseInput from '../../components/atomic/BaseInput.vue'
import BaseButton from '../../components/atomic/BaseButton.vue'
import api from '../../api/client'
import { useConfigStore } from '../../features/config/stores/configStore'
import type { SsoProviderInfo } from '../../api/admin'
import { getApiErrorMessage } from '@/core/errors/errorUtils'
import { normalizeSafeHttpUrl } from '../../utils/safeUrl'

const auth = useAuthStore()
const configStore = useConfigStore()

const email = ref('')
const password = ref('')
const loading = ref(false)
const error = ref('')
const ssoProviders = ref<SsoProviderInfo[]>([])

// Computed properties for auth configuration
const enableSso = computed(() => configStore.config?.authentication?.enable_sso ?? false)
const requireSso = computed(() => configStore.config?.authentication?.require_sso ?? false)
const showSsoButtons = computed(() => enableSso.value && ssoProviders.value.length > 0)
const showPasswordLogin = computed(() => !requireSso.value)

onMounted(async () => {
  // Load config first to check SSO settings
  await configStore.loadConfig()

  // Only fetch providers if SSO is enabled
  if (enableSso.value) {
    try {
      const response = await api.get<SsoProviderInfo[]>('/oauth2/providers')
      ssoProviders.value = response.data
    } catch {
      // SSO not configured, ignore
    }
  }
})

async function handleLogin() {
  loading.value = true
  error.value = ''
  try {
    await auth.login({ email: email.value, password: password.value })
    // Use full page reload to ensure all stores (Teams, Channels, etc.)
    // are initialized cleanly with the new auth state.
    window.location.href = '/'
  } catch (e: unknown) {
    error.value = getApiErrorMessage(e) || 'Failed to login'
  } finally {
    loading.value = false
  }
}

function loginWithSSO(provider: SsoProviderInfo) {
  const loginUrl = normalizeSafeHttpUrl(provider.login_url)
  if (!loginUrl) {
    error.value = 'Invalid SSO provider URL'
    return
  }

  // Detect mobile devices (iOS/Android)
  const isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent)
  const url = new URL(loginUrl)
  url.searchParams.set('redirect_uri', '/')
  if (isMobile) {
    url.searchParams.set('mobile', 'true')
  }
  window.location.href = url.toString()
}

function getProviderIconLabel(providerType: string): string {
  const labels: Record<string, string> = {
    github: 'GH',
    google: 'G',
    oidc: 'ID',
    saml: 'SSO',
  }
  return labels[providerType] ?? 'ID'
}

function getProviderIconClass(providerType: string): string {
  const classes: Record<string, string> = {
    github: 'bg-gray-900 text-white',
    google: 'bg-white text-blue-600 border border-gray-300',
    oidc: 'bg-indigo-100 text-indigo-700',
    saml: 'bg-emerald-100 text-emerald-700',
  }
  return classes[providerType] ?? classes.oidc
}
</script>

<template>
  <AuthLayout>
    <template #title>
      <span v-if="requireSso && showSsoButtons">Sign in with SSO</span>
      <span v-else>Sign in to {{ configStore.siteConfig.site_name }}</span>
    </template>
    <template #subtitle>
      <span v-if="requireSso && showSsoButtons">
        SSO authentication is required for this server
      </span>
      <span v-else>
        Or
        <router-link to="/register" class="font-medium text-indigo-600 hover:text-indigo-500"
          >create a new account</router-link
        >
      </span>
    </template>

    <!-- SSO Buttons -->
    <div v-if="showSsoButtons" class="mb-6">
      <div class="space-y-3">
        <button
          v-for="provider in ssoProviders"
          :key="provider.id"
          class="w-full flex items-center justify-center gap-3 px-4 py-2.5 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors text-gray-700 font-medium"
          @click="loginWithSSO(provider)"
        >
          <span
            class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-[10px] font-semibold"
            :class="getProviderIconClass(provider.provider_type)"
            aria-hidden="true"
          >
            {{ getProviderIconLabel(provider.provider_type) }}
          </span>
          <span>Continue with {{ provider.display_name }}</span>
        </button>
      </div>

      <!-- Divider - only show if password login is also available -->
      <div v-if="showPasswordLogin" class="relative my-6">
        <div class="absolute inset-0 flex items-center">
          <div class="w-full border-t border-gray-300"></div>
        </div>
        <div class="relative flex justify-center text-sm leading-5">
          <span class="px-2 bg-white text-gray-500 font-medium">Or continue with email</span>
        </div>
      </div>
    </div>

    <!-- Password Login Form -->
    <form v-if="showPasswordLogin" class="space-y-6" @submit.prevent="handleLogin">
      <div
        v-if="error"
        class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm"
      >
        {{ error }}
      </div>

      <BaseInput
        id="email"
        v-model="email"
        type="email"
        label="Email address"
        required
        placeholder="you@example.com"
      />

      <BaseInput id="password" v-model="password" type="password" label="Password" required />

      <div class="flex items-center justify-between">
        <div class="flex items-center">
          <input
            id="remember-me"
            name="remember-me"
            type="checkbox"
            class="h-4 w-4 text-indigo-600 focus:ring-indigo-500 border-gray-300 rounded cursor-pointer"
          />
          <label for="remember-me" class="ml-2 block text-sm text-gray-900 cursor-pointer">
            Remember me
          </label>
        </div>

        <div class="text-sm">
          <router-link
            to="/forgot-password"
            class="font-medium text-indigo-600 hover:text-indigo-500"
          >
            Forgot your password?
          </router-link>
        </div>
      </div>

      <div class="pt-2">
        <BaseButton
          type="submit"
          block
          :loading="loading"
          class="py-3 text-base shadow-md hover:shadow-lg transition-all duration-200 ring-offset-2 hover:ring-2 hover:ring-indigo-500"
        >
          Sign in to your account
        </BaseButton>
      </div>
    </form>
  </AuthLayout>
</template>
