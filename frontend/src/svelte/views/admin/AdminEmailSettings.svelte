<script lang="ts">
  import { onMount } from 'svelte'
  import { Mail } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  onMount(() => {
    void adminStore.fetchMailProviders()
  })
</script>

<section class="space-y-5" data-testid="admin-email-page">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Delivery</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Email & SMTP</h1>
    <p class="mt-2 text-sm text-text-3">Provider read path for the Svelte admin console.</p>
  </div>

  <div class="grid grid-cols-1 gap-5 lg:grid-cols-2">
    {#each $adminStore.mailProviders as provider (provider.id)}
      <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <div class="flex items-start justify-between gap-3">
          <div>
            <Mail class="h-5 w-5 text-brand" />
            <h2 class="mt-4 font-semibold text-text-1">{provider.from_name || provider.host}</h2>
            <p class="mt-1 text-sm text-text-3">{provider.from_address}</p>
          </div>
          <span class="rounded-full border border-border-1 bg-bg-surface-2 px-2 py-1 text-xs text-text-2">
            {provider.provider_type}{provider.is_default ? ' · default' : ''}
          </span>
        </div>
        <p class="mt-4 text-sm text-text-3">{provider.host}:{provider.port} · {provider.tls_mode}</p>
      </article>
    {:else}
      <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-10 text-center text-sm text-text-3 shadow-1 lg:col-span-2">
        {$adminStore.loading ? 'Loading mail providers...' : 'No mail providers found.'}
      </div>
    {/each}
  </div>
</section>
