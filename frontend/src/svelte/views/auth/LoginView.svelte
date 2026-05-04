<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { authStore } from '../../stores/auth'
  import { configStore } from '../../stores/config'
  import { svelteApi } from '../../stores/http'

  type SsoProviderInfo = {
    id: string
    provider_type: string
    display_name: string
    login_url: string
  }

  let email = ''
  let password = ''
  let loading = false
  let error = ''
  let siteName = 'RustChat'
  let enableSso = false
  let requireSso = false
  let ssoProviders: SsoProviderInfo[] = []

  $: showSsoButtons = enableSso && ssoProviders.length > 0
  $: showPasswordLogin = !requireSso

  function apiUrl(path: string) {
    const baseUrl = import.meta.env.VITE_API_URL || '/api/v1'
    return `${baseUrl}${path}`
  }

  function navigate(path: string) {
    window.history.pushState({}, '', path)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }

  function getErrorMessage(errorValue: unknown, fallback: string) {
    if (errorValue && typeof errorValue === 'object') {
      const data = (errorValue as { data?: { error?: string; message?: string }; message?: string }).data
      return data?.error ?? data?.message ?? (errorValue as { message?: string }).message ?? fallback
    }
    return fallback
  }

  async function loadSsoProviders() {
    try {
      const response = await svelteApi.get<SsoProviderInfo[]>('/oauth2/providers', { authenticated: false })
      ssoProviders = response.data
    } catch {
      ssoProviders = []
    }
  }

  async function loadPublicConfig() {
    try {
      const response = await fetch(apiUrl('/site/info'))
      if (!response.ok) return
      const data = await response.json()
      siteName = data.site_name ?? siteName
      enableSso = data.enable_sso ?? enableSso
      requireSso = data.require_sso ?? requireSso
    } catch {
      // Public config is optional during the Svelte migration foundation work.
    }
  }

  async function initialize() {
    await configStore.loadConfig()
    const config = get(configStore)

    siteName = config.siteConfig.site_name ?? siteName
    enableSso = config.authConfig?.enable_sso ?? enableSso
    requireSso = config.authConfig?.require_sso ?? requireSso

    if (!config.configLoaded) {
      await loadPublicConfig()
    }

    if (enableSso) {
      await loadSsoProviders()
    }
  }

  async function handleLogin() {
    loading = true
    error = ''

    try {
      await authStore.login({ email, password })
      window.location.href = '/'
    } catch (e) {
      error = getErrorMessage(e, 'Failed to login')
    } finally {
      loading = false
    }
  }

  function loginWithSSO(provider: SsoProviderInfo) {
    const redirectUri = encodeURIComponent('/')
    const isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent)
    const mobileParam = isMobile ? '&mobile=true' : ''
    window.location.href = `${provider.login_url}?redirect_uri=${redirectUri}${mobileParam}`
  }

  onMount(() => {
    void initialize()
  })
</script>

<svelte:head>
  <title>Sign in - {siteName}</title>
</svelte:head>

<main class="min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
  <section class="sm:mx-auto sm:w-full sm:max-w-md" aria-labelledby="login-title">
    <h1 id="login-title" class="text-center text-3xl font-bold tracking-tight text-gray-900">
      {requireSso && showSsoButtons ? 'Sign in with SSO' : `Sign in to ${siteName}`}
    </h1>
    <p class="mt-2 text-center text-sm text-gray-600">
      {#if requireSso && showSsoButtons}
        SSO authentication is required for this server
      {:else}
        Or
        <a href="/register" class="font-medium text-indigo-600 hover:text-indigo-500" on:click|preventDefault={() => navigate('/register')}>create a new account</a>
      {/if}
    </p>
  </section>

  <section class="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
    <div class="bg-white px-4 py-8 shadow sm:rounded-lg sm:px-10">
      {#if showSsoButtons}
        <div class="mb-6 space-y-3">
          {#each ssoProviders as provider (provider.id)}
            <button
              type="button"
              class="w-full flex items-center justify-center gap-3 px-4 py-2.5 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors text-gray-700 font-medium"
              on:click={() => loginWithSSO(provider)}
            >
              <span aria-hidden="true" class="h-5 w-5 rounded-full bg-gray-100"></span>
              <span>Continue with {provider.display_name}</span>
            </button>
          {/each}
        </div>

        {#if showPasswordLogin}
          <div class="relative my-6" aria-hidden="true">
            <div class="absolute inset-0 flex items-center">
              <div class="w-full border-t border-gray-300"></div>
            </div>
            <div class="relative flex justify-center text-sm leading-5">
              <span class="px-2 bg-white text-gray-500 font-medium">Or continue with email</span>
            </div>
          </div>
        {/if}
      {/if}

      {#if showPasswordLogin}
        <form class="space-y-6" on:submit|preventDefault={handleLogin}>
          {#if error}
            <div class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md text-sm" role="alert">
              {error}
            </div>
          {/if}

          <div>
            <label for="email" class="block text-sm font-medium text-gray-700">Email address</label>
            <input
              id="email"
              name="email"
              type="email"
              autocomplete="email"
              required
              placeholder="you@example.com"
              bind:value={email}
              class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
            />
          </div>

          <div>
            <label for="password" class="block text-sm font-medium text-gray-700">Password</label>
            <input
              id="password"
              name="password"
              type="password"
              autocomplete="current-password"
              required
              bind:value={password}
              class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
            />
          </div>

          <div class="flex items-center justify-between">
            <div class="flex items-center">
              <input id="remember-me" name="remember-me" type="checkbox" class="h-4 w-4 text-indigo-600 focus:ring-indigo-500 border-gray-300 rounded cursor-pointer" />
              <label for="remember-me" class="ml-2 block text-sm text-gray-900 cursor-pointer">Remember me</label>
            </div>

            <a href="/forgot-password" class="text-sm font-medium text-indigo-600 hover:text-indigo-500" on:click|preventDefault={() => navigate('/forgot-password')}>Forgot your password?</a>
          </div>

          <button
            type="submit"
            disabled={loading}
            class="flex w-full justify-center rounded-md border border-transparent bg-indigo-600 py-3 px-4 text-base font-medium text-white shadow-md transition-all duration-200 hover:bg-indigo-700 hover:shadow-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {loading ? 'Signing in...' : 'Sign in to your account'}
          </button>
        </form>
      {/if}
    </div>
  </section>
</main>
