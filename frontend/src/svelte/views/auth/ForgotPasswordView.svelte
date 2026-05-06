<script lang="ts">
  import TurnstileWidget from '../../components/auth/TurnstileWidget.svelte'

  let email = ''
  let website = ''
  let loading = false
  let error = ''
  let success = false
  let turnstileEnabled = false
  let turnstileSiteKey = ''
  let turnstileToken = ''
  let turnstileWidget: { reset: () => void } | undefined

  $: turnstileReady = !turnstileEnabled || turnstileToken.length > 0

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

  async function loadAuthConfig() {
    try {
      const response = await fetch(apiUrl('/auth/config'))
      if (!response.ok) return
      const data = await response.json()
      turnstileEnabled = Boolean(data.turnstile?.enabled)
      turnstileSiteKey = data.turnstile?.site_key ?? ''
    } catch {
      turnstileEnabled = false
      turnstileSiteKey = ''
    }
  }

  async function handleSubmit() {
    if (website) {
      error = 'Invalid request'
      return
    }

    if (!turnstileReady) {
      error = 'Please complete the verification'
      return
    }

    loading = true
    error = ''

    try {
      const response = await fetch(apiUrl('/auth/password/forgot'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          email,
          'cf-turnstile-response': turnstileToken || undefined,
          website: website || undefined,
        }),
      })

      if (!response.ok) {
        throw new Error(await readError(response, 'Failed to send reset email. Please try again.'))
      }

      success = true
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to send reset email. Please try again.'
      turnstileToken = ''
      turnstileWidget?.reset()
    } finally {
      loading = false
    }
  }

  loadAuthConfig()
</script>

<svelte:head>
  <title>Reset password - RustChat</title>
</svelte:head>

<main class="min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
  <section class="sm:mx-auto sm:w-full sm:max-w-md" aria-labelledby="forgot-password-title">
    <h1 id="forgot-password-title" class="text-center text-3xl font-bold tracking-tight text-gray-900">
      Reset your password
    </h1>
    <p class="mt-2 text-center text-sm text-gray-600">
      Remember your password?
      <a href="/login" class="font-medium text-primary hover:text-blue-500" on:click|preventDefault={() => navigate('/login')}>Sign in</a>
    </p>
  </section>

  <section class="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
    <div class="bg-white px-4 py-8 shadow sm:rounded-lg sm:px-10">
      {#if success}
        <div class="text-center py-8">
          <div class="mx-auto flex items-center justify-center h-16 w-16 rounded-full bg-green-100 mb-6">
            <svg class="h-8 w-8 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
            </svg>
          </div>
          <h2 class="text-xl font-semibold text-gray-900 mb-2">Check your email</h2>
          <p class="text-gray-600 mb-6">
            If an account exists for <strong>{email}</strong>, you will receive a password reset link. Please check your inbox and spam folder.
          </p>
          <button type="button" class="flex w-full justify-center rounded-md border border-gray-300 bg-white py-2 px-4 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50" on:click={() => navigate('/login')}>
            Back to Login
          </button>
        </div>
      {:else}
        <form class="space-y-6" on:submit|preventDefault={handleSubmit}>
          {#if error}
            <div class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm" role="alert">
              {error}
            </div>
          {/if}

          <div class="honeypot-field" aria-hidden="true">
            <label for="website">Website</label>
            <input id="website" type="text" name="website" bind:value={website} tabindex="-1" autocomplete="off" />
          </div>

          <p class="text-sm text-gray-600">
            Enter your email address and we'll send you a link to reset your password.
          </p>

          <div>
            <label for="email" class="block text-sm font-medium text-gray-700">Email address</label>
            <input id="email" name="email" type="email" autocomplete="email" required placeholder="you@example.com" bind:value={email} class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm" />
          </div>

          {#if turnstileEnabled && turnstileSiteKey}
            <TurnstileWidget
              bind:this={turnstileWidget}
              siteKey={turnstileSiteKey}
              onVerify={(token) => {
                turnstileToken = token
              }}
              onError={() => {
                turnstileToken = ''
                error = 'Verification failed. Please try again.'
              }}
              onExpired={() => {
                turnstileToken = ''
              }}
            />
          {/if}

          <button
            type="submit"
            disabled={loading || !turnstileReady}
            class="flex w-full justify-center rounded-md border border-transparent bg-indigo-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {loading ? 'Sending...' : 'Send Reset Link'}
          </button>
        </form>
      {/if}
    </div>
  </section>
</main>

<style>
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
