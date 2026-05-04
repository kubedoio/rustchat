<script lang="ts">
  import { onMount } from 'svelte'
  import { Activity, Database, HardDrive, RadioTower, Server } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  onMount(() => {
    void adminStore.fetchHealth()
  })

  function statusClass(connected?: boolean) {
    return connected ? 'text-success' : 'text-danger'
  }
</script>

<section class="space-y-5" data-testid="admin-health-page">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Runtime</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">System Health</h1>
    <p class="mt-2 text-sm text-text-3">Dedicated health route for operational checks.</p>
  </div>

  {#if $adminStore.health}
    <div class="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-4">
      <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <Database class="h-5 w-5 text-brand" />
        <h2 class="mt-4 font-semibold text-text-1">Database</h2>
        <p class="mt-1 text-sm {statusClass($adminStore.health.database.connected)}">
          {$adminStore.health.database.connected ? `${$adminStore.health.database.latency_ms}ms latency` : 'Disconnected'}
        </p>
      </article>
      <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <HardDrive class="h-5 w-5 text-brand" />
        <h2 class="mt-4 font-semibold text-text-1">Storage</h2>
        <p class="mt-1 text-sm {statusClass($adminStore.health.storage.connected)}">{$adminStore.health.storage.type}</p>
      </article>
      <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <RadioTower class="h-5 w-5 text-brand" />
        <h2 class="mt-4 font-semibold text-text-1">Redis</h2>
        <p class="mt-1 text-sm {statusClass($adminStore.health.redis.connected)}">
          {$adminStore.health.redis.connected ? `${$adminStore.health.redis.latency_ms}ms latency` : 'Disconnected'}
        </p>
      </article>
      <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <Activity class="h-5 w-5 text-brand" />
        <h2 class="mt-4 font-semibold text-text-1">WebSocket</h2>
        <p class="mt-1 text-sm text-text-3">{$adminStore.health.websocket.active_connections} active connections</p>
      </article>
    </div>

    <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
      <div class="flex items-center gap-3">
        <Server class="h-5 w-5 text-brand" />
        <div>
          <h2 class="font-semibold text-text-1">Instance</h2>
          <p class="text-sm text-text-3">
            Version {$adminStore.health.version || 'unknown'} · {Math.floor(($adminStore.health.uptime_seconds || 0) / 3600)}h uptime
          </p>
        </div>
      </div>
    </section>
  {:else}
    <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-10 text-center text-sm text-text-3 shadow-1">
      {$adminStore.loading ? 'Loading health data...' : 'Health data is unavailable.'}
    </div>
  {/if}
</section>
