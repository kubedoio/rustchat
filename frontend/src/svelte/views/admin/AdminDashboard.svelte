<script lang="ts">
  import { onMount } from 'svelte'
  import { Activity, AlertCircle, CheckCircle, HardDrive, MessageSquare, Server, Users } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  const statCards = [
    { key: 'total_users', label: 'Total Users', icon: Users, tone: 'brand' },
    { key: 'active_users', label: 'Active Users', icon: Activity, tone: 'secondary' },
    { key: 'total_teams', label: 'Teams', icon: Server, tone: 'neutral' },
    { key: 'messages_24h', label: 'Messages (24h)', icon: MessageSquare, tone: 'brand' },
    { key: 'active_connections', label: 'Connections', icon: Activity, tone: 'secondary' },
  ] as const

  onMount(() => {
    void adminStore.fetchOverview()
  })

  function statValue(key: string) {
    if (key === 'active_connections') {
      return $adminStore.health?.websocket.active_connections ?? '—'
    }

    return $adminStore.stats?.[key as keyof NonNullable<typeof $adminStore.stats>] ?? '—'
  }

  function statToneClass(tone: 'brand' | 'secondary' | 'neutral') {
    if (tone === 'brand') return 'border-brand/15 bg-brand/10 text-brand'
    if (tone === 'secondary') return 'border-secondary/15 bg-secondary/10 text-secondary'
    return 'border-border-1 bg-bg-surface-2 text-text-2'
  }

  function healthToneClass(healthy = false) {
    return healthy
      ? 'border-success/20 bg-success/10 text-success'
      : 'border-danger/20 bg-danger/10 text-danger'
  }
</script>

<div class="space-y-8" data-testid="admin-dashboard">
  <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Operations Console</p>
    <h1 class="mt-2 text-[30px] font-semibold tracking-[-0.04em] text-text-1">System Overview</h1>
    <p class="mt-2 max-w-2xl text-sm text-text-3">
      Monitor instance health, usage, and realtime capacity from the migrated Svelte admin shell.
    </p>
  </section>

  {#if $adminStore.error}
    <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
      {$adminStore.error}
    </div>
  {/if}

  <section class="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-5" aria-label="Admin stats">
    {#each statCards as stat (stat.key)}
      {@const Icon = stat.icon}
      <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <div class="mb-4 flex items-center justify-between gap-3">
          <div>
            <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-3">{stat.label}</p>
            <p class="mt-3 text-3xl font-semibold tracking-[-0.03em] text-text-1">{statValue(stat.key)}</p>
          </div>
          <div class="{statToneClass(stat.tone)} flex h-11 w-11 items-center justify-center rounded-r-2 border">
            <Icon class="h-5 w-5" />
          </div>
        </div>
        <p class="text-sm text-text-3">Current snapshot across the workspace.</p>
      </article>
    {/each}
  </section>

  <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <div class="mb-4">
      <h2 class="text-lg font-semibold text-text-1">System Health</h2>
      <p class="mt-1 text-sm text-text-3">Critical services stay readable at a glance.</p>
    </div>

    {#if $adminStore.health}
      <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
        <article class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
          <div class="flex items-center gap-3">
            <div class="{healthToneClass($adminStore.health.database.connected)} flex h-10 w-10 items-center justify-center rounded-full border">
              {#if $adminStore.health.database.connected}
                <CheckCircle class="h-5 w-5" />
              {:else}
                <AlertCircle class="h-5 w-5" />
              {/if}
            </div>
            <div>
              <p class="font-semibold text-text-1">Database</p>
              <p class="text-sm text-text-3">
                {$adminStore.health.database.connected ? `${$adminStore.health.database.latency_ms}ms latency` : 'Disconnected'}
              </p>
            </div>
          </div>
        </article>

        <article class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
          <div class="flex items-center gap-3">
            <div class="{healthToneClass($adminStore.health.storage.connected)} flex h-10 w-10 items-center justify-center rounded-full border">
              <HardDrive class="h-5 w-5" />
            </div>
            <div>
              <p class="font-semibold text-text-1">Storage</p>
              <p class="text-sm text-text-3">{$adminStore.health.storage.type}</p>
            </div>
          </div>
        </article>

        <article class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
          <div class="flex items-center gap-3">
            <div class="flex h-10 w-10 items-center justify-center rounded-full border border-secondary/15 bg-secondary/10 text-secondary">
              <Activity class="h-5 w-5" />
            </div>
            <div>
              <p class="font-semibold text-text-1">WebSocket</p>
              <p class="text-sm text-text-3">{$adminStore.health.websocket.active_connections} connections</p>
            </div>
          </div>
        </article>
      </div>
    {:else}
      <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 py-8 text-center text-sm text-text-3">
        {$adminStore.loading ? 'Loading health status...' : 'Health data is not available yet.'}
      </div>
    {/if}
  </section>
</div>
