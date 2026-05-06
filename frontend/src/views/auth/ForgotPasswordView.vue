<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import client from '../../api/client'
import AuthLayout from '../../layouts/AuthLayout.vue'
import BaseInput from '../../components/atomic/BaseInput.vue'
import BaseButton from '../../components/atomic/BaseButton.vue'
import TurnstileWidget from '../../components/auth/TurnstileWidget.vue'
import { useConfigStore } from '../../stores/config'

const router = useRouter()
const configStore = useConfigStore()

const email = ref('')
const website = ref('') // Honeypot field - should remain empty
const turnstileToken = ref('')
const loading = ref(false)
const error = ref('')
const success = ref(false)

// Turnstile configuration
const turnstileEnabled = ref(false)
const turnstileSiteKey = ref('')
const turnstileVerified = ref(false)

onMounted(async () => {
  configStore.loadConfig()
  
  // Fetch Turnstile configuration
  try {
    const response = await client.get('/auth/config')
    if (response.data.turnstile?.enabled) {
      turnstileEnabled.value = true
      turnstileSiteKey.value = response.data.turnstile.site_key
    }
  } catch {
    // Ignore errors - Turnstile will be disabled
  }
})

function onTurnstileVerify(token: string) {
  turnstileToken.value = token
  turnstileVerified.value = true
}

function onTurnstileError() {
  turnstileVerified.value = false
  error.value = 'Verification failed. Please try again.'
}

async function handleSubmit() {
  // Check honeypot
  if (website.value) {
    // Silently fail if honeypot is filled
    error.value = 'Invalid request'
    return
  }

  // Check Turnstile verification if enabled
  if (turnstileEnabled.value && !turnstileVerified.value) {
    error.value = 'Please complete the verification'
    return
  }

  loading.value = true
  error.value = ''
  try {
    await client.post('/auth/password/forgot', {
      email: email.value,
      'cf-turnstile-response': turnstileToken.value || undefined,
      website: website.value || undefined
    })
    success.value = true
  } catch (e: any) {
    // Always show generic error to prevent email enumeration
    error.value = e.response?.data?.message || 'Failed to send reset email. Please try again.'
    // Reset Turnstile on error
    turnstileVerified.value = false
    turnstileToken.value = ''
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <AuthLayout>
    <template #title>Reset your password</template>
    <template #subtitle>
      Remember your password? 
      <router-link to="/login" class="font-medium text-primary hover:text-blue-500">Sign in</router-link>
    </template>

    <!-- Success State -->
    <div v-if="success" class="text-center py-8">
      <div class="mx-auto flex items-center justify-center h-16 w-16 rounded-full bg-green-100 mb-6">
        <svg class="h-8 w-8 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
        </svg>
      </div>
      <h3 class="text-xl font-semibold text-gray-900 mb-2">
        Check your email
      </h3>
      <p class="text-gray-600 mb-6">
        If an account exists for <strong>{{ email }}</strong>, you will receive a password reset link.
        Please check your inbox and spam folder.
      </p>
      <div class="space-y-4">
        <BaseButton @click="router.push('/login')" variant="secondary" block>
          Back to Login
        </BaseButton>
      </div>
    </div>

    <!-- Forgot Password Form -->
    <form v-else class="space-y-6" @submit.prevent="handleSubmit">
      <div v-if="error" class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm">
        {{ error }}
      </div>

      <!-- Honeypot field - hidden from humans -->
      <div class="honeypot-field" aria-hidden="true">
        <input
          type="text"
          name="website"
          v-model="website"
          tabindex="-1"
          autocomplete="off"
        />
      </div>

      <p class="text-sm text-gray-600">
        Enter your email address and we'll send you a link to reset your password.
      </p>

      <BaseInput
        id="email"
        type="email"
        label="Email address"
        v-model="email"
        required
        placeholder="you@example.com"
      />

      <!-- Turnstile Widget -->
      <TurnstileWidget
        v-if="turnstileEnabled && turnstileSiteKey"
        :site-key="turnstileSiteKey"
        @verify="onTurnstileVerify"
        @error="onTurnstileError"
      />

      <div>
        <BaseButton 
          type="submit" 
          block 
          :loading="loading"
          :disabled="turnstileEnabled && !turnstileVerified"
        >
          Send Reset Link
        </BaseButton>
      </div>
    </form>
  </AuthLayout>
</template>

<style scoped>
/* Hide honeypot field from humans */
.honeypot-field {
  position: absolute;
  left: -9999px;
  top: -9999px;
  opacity: 0;
  height: 0;
  width: 0;
  overflow: hidden;
}
</style>
