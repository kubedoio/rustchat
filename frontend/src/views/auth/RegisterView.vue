<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import client from '../../api/client'
import AuthLayout from '../../layouts/AuthLayout.vue'
import BaseInput from '../../components/atomic/BaseInput.vue'
import BaseButton from '../../components/atomic/BaseButton.vue'
import { useConfigStore } from '../../stores/config'
import { useAuthStore } from '../../stores/auth'

const router = useRouter()
const configStore = useConfigStore()
const authStore = useAuthStore()

const username = ref('')
const email = ref('')
const password = ref('')
const loading = ref(false)
const error = ref('')

onMounted(() => {
  authStore.getAuthPolicy()
})

async function handleRegister() {
  loading.value = true
  error.value = ''
  try {
    await client.post('/auth/register', {
      username: username.value,
      email: email.value,
      password: password.value,
      display_name: username.value
    })
    // Auto login or redirect to login? Let's redirect for now.
    router.push('/login?registered=true')
  } catch (e: any) {
    error.value = e.response?.data?.message || 'Failed to register'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <AuthLayout>
    <template #title>Create your {{ configStore.siteConfig.site_name }} account</template>
    <template #subtitle>
      Already have an account? <router-link to="/login" class="font-medium text-primary hover:text-blue-500">Sign in</router-link>
    </template>

    <form class="space-y-6" @submit.prevent="handleRegister">
      <div v-if="error" class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm">
        {{ error }}
      </div>

      <BaseInput
        id="username"
        label="Username"
        v-model="username"
        required
        placeholder="jdoe"
      />

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

      <div v-if="authStore.authPolicy" class="text-xs text-gray-500 space-y-1">
        <p>Password must contain:</p>
        <ul class="list-disc list-inside">
          <li :class="{ 'text-green-600': password.length >= authStore.authPolicy.password_min_length }">
            At least {{ authStore.authPolicy.password_min_length }} characters
          </li>
          <li v-if="authStore.authPolicy.password_require_uppercase" :class="{ 'text-green-600': /[A-Z]/.test(password) }">
            An uppercase letter
          </li>
          <li v-if="authStore.authPolicy.password_require_number" :class="{ 'text-green-600': /[0-9]/.test(password) }">
            A number
          </li>
          <li v-if="authStore.authPolicy.password_require_symbol" :class="{ 'text-green-600': /[^a-zA-Z0-9]/.test(password) }">
            A symbol
          </li>
        </ul>
      </div>

      <div>
        <BaseButton type="submit" block :loading="loading">
          Create Account
        </BaseButton>
      </div>
    </form>
  </AuthLayout>
</template>
