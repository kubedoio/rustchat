<script lang="ts">
  import { onMount } from 'svelte'

  type AuthPolicy = {
    password_min_length?: number
    passwordMinLength?: number
    password_require_uppercase?: boolean
    passwordRequireUppercase?: boolean
    password_require_lowercase?: boolean
    passwordRequireLowercase?: boolean
    password_require_number?: boolean
    passwordRequireNumber?: boolean
    password_require_symbol?: boolean
    passwordRequireSymbol?: boolean
  }

  let token = ''
  let password = ''
  let confirmPassword = ''
  let loading = false
  let validating = true
  let error = ''
  let success = false
  let tokenValid = false
  let userEmail = ''
  let authPolicy: AuthPolicy | null = null

  $: isSetup = window.location.pathname.includes('set-password')
  $: minPasswordLength = authPolicy?.password_min_length ?? authPolicy?.passwordMinLength ?? 8
  $: requireUppercase = authPolicy?.password_require_uppercase ?? authPolicy?.passwordRequireUppercase ?? false
  $: requireLowercase = authPolicy?.password_require_lowercase ?? authPolicy?.passwordRequireLowercase ?? false
  $: requireNumber = authPolicy?.password_require_number ?? authPolicy?.passwordRequireNumber ?? false
  $: requireSymbol = authPolicy?.password_require_symbol ?? authPolicy?.passwordRequireSymbol ?? false
  $: passwordErrors = getPasswordErrors()
  $: passwordsMatch = password === confirmPassword && password !== ''
  $: canSubmit = tokenValid && password.length > 0 && passwordsMatch && passwordErrors.length === 0

  function apiUrl(path: string) {
    const baseUrl = import.meta.env.VITE_API_URL || '/api/v1'
    return `${baseUrl}${path}`
  }

  function navigate(path: string) {
    window.history.pushState({}, '', path)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }

  async function readError(response: Response, fallback: string) {
    try {
      const data = await response.json()
      return data?.message ?? data?.error ?? fallback
    } catch {
      return fallback
    }
  }

  function getPasswordErrors() {
    const errors: string[] = []

    if (password.length < minPasswordLength) {
      errors.push(`At least ${minPasswordLength} characters`)
    }
    if (requireUppercase && !/[A-Z]/.test(password)) {
      errors.push('One uppercase letter')
    }
    if (requireLowercase && !/[a-z]/.test(password)) {
      errors.push('One lowercase letter')
    }
    if (requireNumber && !/[0-9]/.test(password)) {
      errors.push('One number')
    }
    if (requireSymbol && !/[^a-zA-Z0-9]/.test(password)) {
      errors.push('One special character')
    }

    return errors
  }

  async function loadAuthPolicy() {
    try {
      const response = await fetch(apiUrl('/auth/policy'))
      if (response.ok) {
        authPolicy = await response.json()
      }
    } catch {
      authPolicy = authPolicy ?? null
    }
  }

  async function validateToken() {
    const params = new URLSearchParams(window.location.search)
    token = params.get('token') ?? ''

    if (!token) {
      error = 'Invalid or missing reset token'
      validating = false
      return
    }

    await loadAuthPolicy()

    try {
      const response = await fetch(apiUrl('/auth/password/validate'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token }),
      })

      if (!response.ok) {
        throw new Error(await readError(response, 'This link has expired or is invalid. Please request a new one.'))
      }

      const data = await response.json()
      if (data.valid) {
        tokenValid = true
        userEmail = data.email ?? ''
      } else {
        error = 'This link has expired or is invalid. Please request a new one.'
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'This link has expired or is invalid. Please request a new one.'
    } finally {
      validating = false
    }
  }

  async function handleSubmit() {
    if (!canSubmit) return

    loading = true
    error = ''

    try {
      const response = await fetch(apiUrl('/auth/password/reset'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          token,
          new_password: password,
        }),
      })

      if (!response.ok) {
        throw new Error(await readError(response, 'Failed to reset password. Please try again.'))
      }

      success = true
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to reset password. Please try again.'
    } finally {
      loading = false
    }
  }

  onMount(() => {
    void validateToken()
  })
</script>

<svelte:head>
  <title>{isSetup ? 'Set password' : 'Reset password'} - RustChat</title>
</svelte:head>

<main class="min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
  <section class="sm:mx-auto sm:w-full sm:max-w-md" aria-labelledby="reset-password-title">
    <h1 id="reset-password-title" class="text-center text-3xl font-bold tracking-tight text-gray-900">
      {isSetup ? 'Set your password' : 'Reset your password'}
    </h1>
    <p class="mt-2 text-center text-sm text-gray-600">
      {isSetup ? 'Create a secure password for your account' : 'Enter your new password below'}
    </p>
  </section>

  <section class="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
    <div class="bg-white px-4 py-8 shadow sm:rounded-lg sm:px-10">
      {#if validating}
        <div class="text-center py-8" aria-live="polite">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto"></div>
          <p class="mt-4 text-gray-600">Validating your link...</p>
        </div>
      {:else if !tokenValid}
        <div class="text-center py-8">
          <div class="mx-auto flex items-center justify-center h-16 w-16 rounded-full bg-red-100 mb-6">
            <svg class="h-8 w-8 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </div>
          <h2 class="text-xl font-semibold text-gray-900 mb-2">Link expired or invalid</h2>
          <p class="text-gray-600 mb-6">{error}</p>
          <button type="button" class="flex w-full justify-center rounded-md border border-transparent bg-indigo-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-indigo-700" on:click={() => navigate('/forgot-password')}>
            Request New Link
          </button>
        </div>
      {:else if success}
        <div class="text-center py-8">
          <div class="mx-auto flex items-center justify-center h-16 w-16 rounded-full bg-green-100 mb-6">
            <svg class="h-8 w-8 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
            </svg>
          </div>
          <h2 class="text-xl font-semibold text-gray-900 mb-2">
            {isSetup ? 'Password set successfully!' : 'Password reset successfully!'}
          </h2>
          <p class="text-gray-600 mb-6">
            Your password has been {isSetup ? 'set' : 'reset'}. You can now sign in with your new password.
          </p>
          <button type="button" class="flex w-full justify-center rounded-md border border-transparent bg-indigo-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-indigo-700" on:click={() => navigate('/login')}>
            Sign In
          </button>
        </div>
      {:else}
        <form class="space-y-6" on:submit|preventDefault={handleSubmit}>
          {#if error}
            <div class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm" role="alert">
              {error}
            </div>
          {/if}

          {#if userEmail}
            <div class="bg-blue-50 border border-blue-200 rounded-md p-4">
              <p class="text-sm text-blue-700">Setting password for: <strong>{userEmail}</strong></p>
            </div>
          {/if}

          <div>
            <label for="password" class="block text-sm font-medium text-gray-700">New password</label>
            <input id="password" name="password" type="password" autocomplete="new-password" required placeholder="Enter your new password" bind:value={password} class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm" />
          </div>

          <div>
            <label for="confirm-password" class="block text-sm font-medium text-gray-700">Confirm password</label>
            <input id="confirm-password" name="confirm-password" type="password" autocomplete="new-password" required placeholder="Confirm your new password" bind:value={confirmPassword} class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm" />
          </div>

          <div class="text-xs text-gray-500 space-y-1">
            <p>Password must contain:</p>
            <ul class="list-disc list-inside">
              <li class:text-green-600={password.length >= minPasswordLength}>At least {minPasswordLength} characters</li>
              {#if requireUppercase}
                <li class:text-green-600={/[A-Z]/.test(password)}>An uppercase letter</li>
              {/if}
              {#if requireLowercase}
                <li class:text-green-600={/[a-z]/.test(password)}>A lowercase letter</li>
              {/if}
              {#if requireNumber}
                <li class:text-green-600={/[0-9]/.test(password)}>A number</li>
              {/if}
              {#if requireSymbol}
                <li class:text-green-600={/[^a-zA-Z0-9]/.test(password)}>A symbol</li>
              {/if}
              <li class:text-green-600={passwordsMatch && confirmPassword !== ''}>Passwords match</li>
            </ul>
          </div>

          <button
            type="submit"
            disabled={loading || !canSubmit}
            class="flex w-full justify-center rounded-md border border-transparent bg-indigo-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {loading ? 'Saving...' : isSetup ? 'Set Password' : 'Reset Password'}
          </button>
        </form>
      {/if}
    </div>
  </section>
</main>
