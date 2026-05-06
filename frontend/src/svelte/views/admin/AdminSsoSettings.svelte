<script lang="ts">
  import { onMount } from 'svelte'
  import { KeyRound } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  onMount(() => {
    void adminStore.fetchSsoConfigs()
  })
</script>

<section class="space-y-5" data-testid="admin-sso-page">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Federated identity</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">SSO / OAuth</h1>
    <p class="mt-2 text-sm text-text-3">Provider list route for the migrated admin shell.</p>
  </div>

  <div class="grid grid-cols-1 gap-5 lg:grid-cols-2">
    {#each $adminStore.ssoConfigs as config (config.id)}
      <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <div class="flex items-start justify-between gap-3">
          <div>
            <KeyRound class="h-5 w-5 text-brand" />
            <h2 class="mt-4 font-semibold text-text-1">{config.display_name || config.provider_key}</h2>
            <p class="mt-1 text-sm text-text-3">{config.issuer_url || config.provider_type}</p>
          </div>
          <span class="rounded-full border border-border-1 bg-bg-surface-2 px-2 py-1 text-xs text-text-2">
            {config.is_active ? 'Active' : 'Inactive'}
          </span>
        </div>
        <p class="mt-4 text-sm text-text-3">Default role: {config.default_role || 'member'}</p>
      </article>
    {:else}
      <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-10 text-center text-sm text-text-3 shadow-1 lg:col-span-2">
        {$adminStore.loading ? 'Loading SSO providers...' : 'No SSO providers found.'}
      </div>
    {/each}
  </div>
</section>
