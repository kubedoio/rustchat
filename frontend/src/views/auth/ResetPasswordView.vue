<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import client from '../../api/client'
import AuthLayout from '../../layouts/AuthLayout.vue'
import BaseInput from '../../components/atomic/BaseInput.vue'
import BaseButton from '../../components/atomic/BaseButton.vue'
import { useAuthStore } from '../../stores/auth'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const token = ref('')
const isSetup = computed(() => route.path.includes('set-password'))

const password = ref('')
const confirmPassword = ref('')
const loading = ref(false)
const validating = ref(true)
const error = ref('')
const success = ref(false)
const tokenValid = ref(false)
const userEmail = ref('')

// Password validation
const passwordErrors = computed(() => {
  const errors: string[] = []
  const pwd = password.value
  const policy = authStore.authPolicy
  
  if (!policy) return errors
  
  const minLength = policy.password_min_length ?? policy.passwordMinLength ?? 8
  if (pwd.length < minLength) {
    errors.push(`At least ${minLength} characters`)
  }
  if (policy.password_require_uppercase && !/[A-Z]/.test(pwd)) {
    errors.push('One uppercase letter')
  }
  if (policy.password_require_lowercase && !/[a-z]/.test(pwd)) {
    errors.push('One lowercase letter')
  }
  if (policy.password_require_number && !/[0-9]/.test(pwd)) {
    errors.push('One number')
  }
  if (policy.password_require_symbol && !/[^a-zA-Z0-9]/.test(pwd)) {
    errors.push('One special character')
  }
  
  return errors
})

const passwordsMatch = computed(() => {
  return password.value === confirmPassword.value && password.value !== ''
})

const canSubmit = computed(() => {
  return tokenValid.value && 
         password.value.length > 0 && 
         passwordsMatch.value && 
         passwordErrors.value.length === 0
})

onMounted(async () => {
  token.value = route.query.token as string || ''
  
  if (!token.value) {
    error.value = 'Invalid or missing reset token'
    validating.value = false
    return
  }
  
  // Load auth policy for password validation
  await authStore.getAuthPolicy()
  
  // Validate token
  try {
    const response = await client.post('/auth/password/validate', {
      token: token.value
    })
    
    if (response.data.valid) {
      tokenValid.value = true
      userEmail.value = response.data.email || ''
    } else {
      error.value = 'This link has expired or is invalid. Please request a new one.'
    }
  } catch (e: any) {
    error.value = 'This link has expired or is invalid. Please request a new one.'
  } finally {
    validating.value = false
  }
})

async function handleSubmit() {
  if (!canSubmit.value) return
  
  loading.value = true
  error.value = ''
  
  try {
    await client.post('/auth/password/reset', {
      token: token.value,
      new_password: password.value
    })
    success.value = true
  } catch (e: any) {
    error.value = e.response?.data?.message || 'Failed to reset password. Please try again.'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <AuthLayout>
    <template #title>
      {{ isSetup ? 'Set your password' : 'Reset your password' }}
    </template>
    <template #subtitle>
      {{ isSetup ? 'Create a secure password for your account' : 'Enter your new password below' }}
    </template>

    <!-- Validating State -->
    <div v-if="validating" class="text-center py-8">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto"></div>
      <p class="mt-4 text-gray-600">Validating your link...</p>
    </div>

    <!-- Invalid Token State -->
    <div v-else-if="!tokenValid" class="text-center py-8">
      <div class="mx-auto flex items-center justify-center h-16 w-16 rounded-full bg-red-100 mb-6">
        <svg class="h-8 w-8 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
        </svg>
      </div>
      <h3 class="text-xl font-semibold text-gray-900 mb-2">
        Link expired or invalid
      </h3>
      <p class="text-gray-600 mb-6">
        {{ error }}
      </p>
      <BaseButton @click="router.push('/forgot-password')" block>
        Request New Link
      </BaseButton>
    </div>

    <!-- Success State -->
    <div v-else-if="success" class="text-center py-8">
      <div class="mx-auto flex items-center justify-center h-16 w-16 rounded-full bg-green-100 mb-6">
        <svg class="h-8 w-8 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
        </svg>
      </div>
      <h3 class="text-xl font-semibold text-gray-900 mb-2">
        {{ isSetup ? 'Password set successfully!' : 'Password reset successfully!' }}
      </h3>
      <p class="text-gray-600 mb-6">
        Your password has been {{ isSetup ? 'set' : 'reset' }}. You can now sign in with your new password.
      </p>
      <BaseButton @click="router.push('/login')" block>
        Sign In
      </BaseButton>
    </div>

    <!-- Reset Password Form -->
    <form v-else class="space-y-6" @submit.prevent="handleSubmit">
      <div v-if="error" class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm">
        {{ error }}
      </div>

      <div v-if="userEmail" class="bg-blue-50 border border-blue-200 rounded-md p-4">
        <p class="text-sm text-blue-700">
          Setting password for: <strong>{{ userEmail }}</strong>
        </p>
      </div>

      <BaseInput
        id="password"
        type="password"
        label="New password"
        v-model="password"
        required
        placeholder="Enter your new password"
      />

      <BaseInput
        id="confirm-password"
        type="password"
        label="Confirm password"
        v-model="confirmPassword"
        required
        placeholder="Confirm your new password"
      />

      <!-- Password Requirements -->
      <div v-if="authStore.authPolicy" class="text-xs text-gray-500 space-y-1">
        <p>Password must contain:</p>
        <ul class="list-disc list-inside">
          <li :class="{ 'text-green-600': password.length >= (authStore.authPolicy.password_min_length ?? authStore.authPolicy.passwordMinLength ?? 8) }">
            At least {{ authStore.authPolicy.password_min_length ?? authStore.authPolicy.passwordMinLength ?? 8 }} characters
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
          <li :class="{ 'text-green-600': passwordsMatch && confirmPassword !== '' }">
            Passwords match
          </li>
        </ul>
      </div>

      <div>
        <BaseButton 
          type="submit" 
          block 
          :loading="loading"
          :disabled="!canSubmit"
        >
          {{ isSetup ? 'Set Password' : 'Reset Password' }}
        </BaseButton>
      </div>
    </form>
  </AuthLayout>
</template>
