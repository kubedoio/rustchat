<script lang="ts">
  import { onMount } from 'svelte'
  import { CheckCircle, Key, Lock, Save, Users } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'
  import type { AuthConfig } from '../../../api/admin'

  const authForm = $state<AuthConfig>({
    enable_email_password: true,
    enable_sso: false,
    require_sso: false,
    allow_registration: true,
    enable_sign_in_with_email: true,
    enable_sign_in_with_username: true,
    enable_sign_up_with_email: true,
    enable_sign_up_with_gitlab: false,
    enable_sign_up_with_google: false,
    enable_sign_up_with_office365: false,
    enable_sign_up_with_openid: false,
    enable_user_creation: true,
    enable_open_server: false,
    enable_guest_accounts: false,
    enable_multifactor_authentication: false,
    enforce_multifactor_authentication: false,
    enable_saml: false,
    enable_ldap: false,
    password_min_length: 8,
    password_require_lowercase: true,
    password_require_uppercase: true,
    password_require_number: true,
    password_require_symbol: false,
    password_enable_forgot_link: true,
    session_length_hours: 24,
  })

  let saving = $state(false)
  let saved = $state(false)
  const authToggles = [
    ['password_require_lowercase', 'Require lowercase letter'],
    ['password_require_uppercase', 'Require uppercase letter'],
    ['password_require_number', 'Require number'],
    ['password_require_symbol', 'Require symbol'],
    ['password_enable_forgot_link', 'Enable forgot password link'],
    ['enable_multifactor_authentication', 'Enable multi-factor authentication'],
  ] as const

  onMount(async () => {
    await adminStore.fetchConfig()
    if ($adminStore.config?.authentication) {
      Object.assign(authForm, $adminStore.config.authentication)
    }
  })

  async function saveSettings() {
    saving = true
    saved = false
    try {
      saved = await adminStore.updateConfig('authentication', authForm)
    } finally {
      saving = false
    }
  }

  function setAuthToggle(key: (typeof authToggles)[number][0], checked: boolean) {
    authForm[key] = checked
  }
</script>

<section class="space-y-6" data-testid="admin-security-settings-page">
  <div class="flex flex-col gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Authentication controls</p>
      <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Security Settings</h1>
      <p class="mt-2 text-sm text-text-3">Configure sign-in methods, password policy, sessions, and registration.</p>
    </div>
    <div class="flex items-center gap-3">
      {#if saved}
        <span class="inline-flex items-center gap-1 text-sm font-medium text-success"><CheckCircle class="h-4 w-4" /> Saved</span>
      {/if}
      <button type="button" onclick={saveSettings} disabled={saving} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground disabled:opacity-50">
        <Save class="h-4 w-4" />
        {saving ? 'Saving...' : 'Save Changes'}
      </button>
    </div>
  </div>

  {#if $adminStore.error}
    <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">{$adminStore.error}</div>
  {/if}

  <div class="space-y-5">
    <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
      <div class="mb-5 flex items-center gap-2">
        <Key class="h-5 w-5 text-text-4" />
        <h2 class="text-lg font-semibold text-text-1">Authentication Methods</h2>
      </div>
      <div class="space-y-3">
        <label class="flex items-center justify-between gap-4 rounded-r-2 bg-bg-surface-2 p-4">
          <span><span class="block font-semibold text-text-1">Email & Password</span><span class="text-sm text-text-3">Allow users to sign in with email and password</span></span>
          <input type="checkbox" bind:checked={authForm.enable_email_password} class="h-5 w-5 rounded text-brand" />
        </label>
        <label class="flex items-center justify-between gap-4 rounded-r-2 bg-bg-surface-2 p-4">
          <span><span class="block font-semibold text-text-1">Single Sign-On</span><span class="text-sm text-text-3">Enable login via external identity providers</span></span>
          <input type="checkbox" bind:checked={authForm.enable_sso} class="h-5 w-5 rounded text-brand" />
        </label>
        {#if authForm.enable_sso}
          <label class="flex items-center justify-between gap-4 rounded-r-2 border border-warning/20 bg-warning/10 p-4">
            <span><span class="block font-semibold text-warning">Require SSO</span><span class="text-sm text-warning">Disable password login and require SSO only</span></span>
            <input type="checkbox" bind:checked={authForm.require_sso} class="h-5 w-5 rounded text-warning" />
          </label>
        {/if}
      </div>
    </section>

    <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
      <div class="mb-5 flex items-center gap-2">
        <Lock class="h-5 w-5 text-text-4" />
        <h2 class="text-lg font-semibold text-text-1">Password Policy</h2>
      </div>
      <div class="grid grid-cols-1 gap-5 md:grid-cols-2">
        <div>
          <label for="password-length" class="mb-1 block text-sm font-medium text-text-2">Minimum Length</label>
          <input id="password-length" type="number" min="6" max="32" bind:value={authForm.password_min_length} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
        <div>
          <label for="session-length" class="mb-1 block text-sm font-medium text-text-2">Session Length (hours)</label>
          <input id="session-length" type="number" min="1" max="720" bind:value={authForm.session_length_hours} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
      </div>
      <div class="mt-5 grid grid-cols-1 gap-3 sm:grid-cols-2">
        {#each authToggles as item}
          <label class="flex items-center gap-3 rounded-r-2 bg-bg-surface-2 p-3 text-sm text-text-2">
            <input
              type="checkbox"
              checked={Boolean(authForm[item[0]])}
              onchange={(event) => setAuthToggle(item[0], event.currentTarget.checked)}
              class="h-4 w-4 rounded text-brand"
            />
            {item[1]}
          </label>
        {/each}
      </div>
    </section>

    <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
      <div class="mb-5 flex items-center gap-2">
        <Users class="h-5 w-5 text-text-4" />
        <h2 class="text-lg font-semibold text-text-1">Registration</h2>
      </div>
      <label class="flex items-center justify-between gap-4 rounded-r-2 bg-bg-surface-2 p-4">
        <span><span class="block font-semibold text-text-1">Allow Public Registration</span><span class="text-sm text-text-3">Anyone can create an account</span></span>
        <input type="checkbox" bind:checked={authForm.allow_registration} class="h-5 w-5 rounded text-brand" />
      </label>
    </section>
  </div>
</section>
