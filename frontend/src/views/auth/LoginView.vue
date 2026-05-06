<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAuthStore } from '../../stores/auth'
import AuthLayout from '../../layouts/AuthLayout.vue'
import BaseInput from '../../components/atomic/BaseInput.vue'
import BaseButton from '../../components/atomic/BaseButton.vue'
import api from '../../api/client'
import { useConfigStore } from '../../stores/config'
import type { SsoProviderInfo } from '../../api/admin'

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
  } catch (e: any) {
    error.value = e.response?.data?.error || e.response?.data?.message || 'Failed to login'
  } finally {
    loading.value = false
  }
}

function loginWithSSO(provider: SsoProviderInfo) {
  // Include redirect_uri to return to home after login
  const redirectUri = encodeURIComponent('/')
  // Detect mobile devices (iOS/Android)
  const isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent)
  const mobileParam = isMobile ? '&mobile=true' : ''
  window.location.href = `${provider.login_url}?redirect_uri=${redirectUri}${mobileParam}`
}

function getProviderIcon(providerType: string): string {
  const icons: Record<string, string> = {
    github: `<svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>`,
    google: `<svg class="w-5 h-5" viewBox="0 0 24 24"><path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/><path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/><path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/><path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/></svg>`,
    oidc: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7h3a2 2 0 012 2v7a2 2 0 01-2 2h-3M9 7H6a2 2 0 00-2 2v7a2 2 0 002 2h3m3-9l3-3 3 3m-3 3v6"/></svg>`,
    saml: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/></svg>`,
  }
  const icon: string | undefined = icons[providerType]
  if (icon !== undefined) {
    return icon
  }
  // Fallback to OIDC icon
  return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7h3a2 2 0 012 2v7a2 2 0 01-2 2h-3M9 7H6a2 2 0 00-2 2v7a2 2 0 002 2h3m3-9l3-3 3 3m-3 3v6"/></svg>`
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
        Or <router-link to="/register" class="font-medium text-indigo-600 hover:text-indigo-500">create a new account</router-link>
      </span>
    </template>

    <!-- SSO Buttons -->
    <div v-if="showSsoButtons" class="mb-6">
      <div class="space-y-3">
        <button
          v-for="provider in ssoProviders"
          :key="provider.id"
          @click="loginWithSSO(provider)"
          class="w-full flex items-center justify-center gap-3 px-4 py-2.5 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors text-gray-700 font-medium"
        >
          <span v-html="getProviderIcon(provider.provider_type)"></span>
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
      <div v-if="error" class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm">
        {{ error }}
      </div>

      <BaseInput
        id="email"
        type="email"
        label="Email address"
        v-model="email"
        required
        placeholder="you@example.com"
      />

      <BaseInput
        id="password"
        type="password"
        label="Password"
        v-model="password"
        required
      />

      <div class="flex items-center justify-between">
        <div class="flex items-center">
          <input id="remember-me" name="remember-me" type="checkbox" class="h-4 w-4 text-indigo-600 focus:ring-indigo-500 border-gray-300 rounded cursor-pointer">
          <label for="remember-me" class="ml-2 block text-sm text-gray-900 cursor-pointer">
            Remember me
          </label>
        </div>

        <div class="text-sm">
          <router-link to="/forgot-password" class="font-medium text-indigo-600 hover:text-indigo-500">
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
