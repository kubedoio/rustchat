<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAuthStore } from '../../stores/auth'
import AuthLayout from '../../layouts/AuthLayout.vue'
import BaseInput from '../../components/atomic/BaseInput.vue'
import BaseButton from '../../components/atomic/BaseButton.vue'
import api from '../../api/client'
import { useConfigStore } from '../../stores/config'

interface SsoProvider {
  id: string
  name: string
  icon_url?: string
}

const auth = useAuthStore()
const configStore = useConfigStore()

const email = ref('')
const password = ref('')
const loading = ref(false)
const error = ref('')
const ssoProviders = ref<SsoProvider[]>([])

onMounted(async () => {
  try {
    const response = await api.get<SsoProvider[]>('/oauth2/providers')
    ssoProviders.value = response.data
  } catch {
    // SSO not configured, ignore
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
    error.value = e.response?.data?.message || 'Failed to login'
  } finally {
    loading.value = false
  }
}

function loginWithSSO(providerId: string) {
  window.location.href = `/api/v1/oauth2/${providerId}/login`
}
</script>

<template>
  <AuthLayout>
    <template #title>Sign in to {{ configStore.siteConfig.site_name }}</template>
    <template #subtitle>
      Or <router-link to="/register" class="font-medium text-indigo-600 hover:text-indigo-500 dark:text-indigo-400">create a new account</router-link>
    </template>

    <!-- SSO Buttons -->
    <div v-if="ssoProviders.length > 0" class="mb-6">
      <div class="space-y-3">
        <button
          v-for="provider in ssoProviders"
          :key="provider.id"
          @click="loginWithSSO(provider.id)"
          class="w-full flex items-center justify-center gap-2 px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-700 dark:text-gray-200 font-medium"
        >
          <img v-if="provider.icon_url" :src="provider.icon_url" :alt="provider.name" class="w-5 h-5" />
          <span>Continue with {{ provider.name }}</span>
        </button>
      </div>
      
      <div class="relative my-6">
        <div class="absolute inset-0 flex items-center">
          <div class="w-full border-t border-gray-300 dark:border-gray-600"></div>
        </div>
        <div class="relative flex justify-center text-sm leading-5">
          <span class="px-2 bg-white dark:bg-gray-800 text-gray-500 font-medium">Or continue with email</span>
        </div>
      </div>
    </div>

    <form class="space-y-6" @submit.prevent="handleLogin">
      <div v-if="error" class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400 px-4 py-3 rounded-md text-sm">
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
          <label for="remember-me" class="ml-2 block text-sm text-gray-900 dark:text-gray-300 cursor-pointer">
            Remember me
          </label>
        </div>

        <div class="text-sm">
          <a href="#" class="font-medium text-indigo-600 hover:text-indigo-500 dark:text-indigo-400">
            Forgot your password?
          </a>
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

