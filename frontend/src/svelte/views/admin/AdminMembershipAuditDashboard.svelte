<script lang="ts">
  import { onMount } from 'svelte'
  import { Activity, AlertTriangle, CheckCircle, FileText, Filter, RefreshCw, TrendingUp, XCircle } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  const today = new Date().toISOString().slice(0, 10)
  const weekAgo = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10)

  let showFilters = $state(false)
  const filters = $state({
    status: '',
    action: '',
    from_date: weekAgo,
    to_date: today,
  })

  const summary = $derived($adminStore.membershipAuditSummary)
  const hasFailures = $derived(Boolean(summary && summary.failed_operations_24h > 0))

  onMount(() => {
    void refresh()
  })

  function failureRateClass(rate = 0) {
    if (rate < 5) return 'text-success'
    if (rate < 15) return 'text-warning'
    return 'text-danger'
  }

  async function refresh() {
    await adminStore.fetchMembershipAuditDashboard(filters)
  }
</script>

<section class="space-y-6" data-testid="admin-membership-audit-page">
  <div class="flex flex-col gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Membership audit</p>
      <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Audit Dashboard</h1>
      <p class="mt-2 text-sm text-text-3">Monitor membership policy execution, failures, and recent audit records.</p>
    </div>
    <button type="button" onclick={refresh} disabled={$adminStore.loading} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground disabled:opacity-50">
      <RefreshCw class="h-4 w-4 {$adminStore.loading ? 'animate-spin' : ''}" /> Refresh
    </button>
  </div>

  {#if $adminStore.error}
    <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
      {$adminStore.error}
    </div>
  {/if}

  <div class="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
    <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-text-3">Total Operations (24h)</p>
          <p class="mt-2 text-3xl font-semibold text-text-1">{summary?.total_operations_24h ?? 0}</p>
        </div>
        <div class="rounded-r-2 bg-brand/10 p-3 text-brand"><Activity class="h-6 w-6" /></div>
      </div>
    </article>
    <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-text-3">Successful (24h)</p>
          <p class="mt-2 text-3xl font-semibold text-success">{summary?.successful_operations_24h ?? 0}</p>
        </div>
        <div class="rounded-r-2 bg-success/10 p-3 text-success"><CheckCircle class="h-6 w-6" /></div>
      </div>
    </article>
    <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-text-3">Failed (24h)</p>
          <p class="mt-2 text-3xl font-semibold text-danger">{summary?.failed_operations_24h ?? 0}</p>
        </div>
        <div class="rounded-r-2 bg-danger/10 p-3 text-danger"><XCircle class="h-6 w-6" /></div>
      </div>
      {#if hasFailures}
        <p class="mt-2 text-sm text-danger">{summary?.policies_with_failures ?? 0} policies affected</p>
      {/if}
    </article>
    <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-text-3">Failure Rate (24h)</p>
          <p class="mt-2 text-3xl font-semibold {failureRateClass(summary?.failure_rate_24h)}">{(summary?.failure_rate_24h ?? 0).toFixed(1)}%</p>
        </div>
        <div class="rounded-r-2 bg-warning/10 p-3 text-warning"><TrendingUp class="h-6 w-6" /></div>
      </div>
    </article>
  </div>

  {#if summary && summary.failure_rate_24h > 15}
    <div class="flex items-start gap-3 rounded-r-2 border border-danger/20 bg-danger/10 p-4 text-danger">
      <AlertTriangle class="mt-0.5 h-5 w-5 shrink-0" />
      <div>
        <h2 class="text-sm font-semibold">High Failure Rate Detected</h2>
        <p class="mt-1 text-sm">The membership policy failure rate is {summary.failure_rate_24h.toFixed(1)}% in the last 24 hours.</p>
      </div>
    </div>
  {/if}

  {#if $adminStore.membershipRecentFailures.length}
    <section class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
      <div class="border-b border-border-1 px-5 py-4">
        <h2 class="flex items-center gap-2 font-semibold text-text-1"><AlertTriangle class="h-5 w-5 text-danger" /> Recent Failures</h2>
      </div>
      <div class="divide-y divide-border-1">
        {#each $adminStore.membershipRecentFailures.slice(0, 5) as failure (failure.id)}
          <article class="px-5 py-4">
            <div class="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
              <div>
                <p class="font-semibold text-text-1">{failure.policy_name || 'Unknown Policy'}</p>
                <p class="mt-1 text-xs text-text-3">User: @{failure.username || failure.user_id.slice(0, 8)} · Target: {failure.target_type} {failure.target_id.slice(0, 8)}...</p>
                {#if failure.error_message}
                  <p class="mt-1 font-mono text-xs text-danger">{failure.error_message}</p>
                {/if}
              </div>
              <span class="text-xs text-text-4">{new Date(failure.created_at).toLocaleTimeString()}</span>
            </div>
          </article>
        {/each}
      </div>
    </section>
  {/if}

  {#if $adminStore.membershipPolicyFailureStats.length}
    <section class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
      <div class="border-b border-border-1 px-5 py-4">
        <h2 class="font-semibold text-text-1">Policies with Failures</h2>
      </div>
      <div class="overflow-x-auto">
        <table class="min-w-full divide-y divide-border-1 text-sm">
          <thead class="bg-bg-surface-2 text-left text-xs uppercase tracking-[0.18em] text-text-3">
            <tr><th class="px-5 py-3">Policy</th><th class="px-5 py-3 text-right">Total</th><th class="px-5 py-3 text-right">Failed</th><th class="px-5 py-3 text-right">Rate</th><th class="px-5 py-3">Last Error</th></tr>
          </thead>
          <tbody class="divide-y divide-border-1">
            {#each $adminStore.membershipPolicyFailureStats as stat (stat.policy_id)}
              <tr>
                <td class="px-5 py-4 font-semibold text-text-1">{stat.policy_name}</td>
                <td class="px-5 py-4 text-right text-text-2">{stat.total_operations}</td>
                <td class="px-5 py-4 text-right font-semibold text-danger">{stat.failed_operations}</td>
                <td class="px-5 py-4 text-right {failureRateClass(stat.failure_rate)}">{stat.failure_rate.toFixed(1)}%</td>
                <td class="max-w-xs truncate px-5 py-4 text-xs text-text-3">{stat.last_error_message || '-'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/if}

  <section class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <div class="flex items-center justify-between border-b border-border-1 px-5 py-4">
      <h2 class="flex items-center gap-2 font-semibold text-text-1"><FileText class="h-5 w-5" /> Audit Logs</h2>
      <button type="button" onclick={() => (showFilters = !showFilters)} class="inline-flex items-center gap-1 text-sm text-text-2 hover:text-text-1">
        <Filter class="h-4 w-4" /> Filters
      </button>
    </div>

    {#if showFilters}
      <div class="grid grid-cols-1 gap-4 border-b border-border-1 bg-bg-surface-2 px-5 py-4 md:grid-cols-4">
        <div>
          <label for="audit-status" class="mb-1 block text-xs font-medium text-text-2">Status</label>
          <select id="audit-status" bind:value={filters.status} onchange={refresh} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
            <option value="">All</option><option value="success">Success</option><option value="failed">Failed</option><option value="pending">Pending</option>
          </select>
        </div>
        <div>
          <label for="audit-action" class="mb-1 block text-xs font-medium text-text-2">Action</label>
          <select id="audit-action" bind:value={filters.action} onchange={refresh} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
            <option value="">All</option><option value="add">Add</option><option value="remove">Remove</option><option value="skip">Skip</option>
          </select>
        </div>
        <div>
          <label for="audit-from" class="mb-1 block text-xs font-medium text-text-2">From</label>
          <input id="audit-from" type="date" bind:value={filters.from_date} onchange={refresh} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
        <div>
          <label for="audit-to" class="mb-1 block text-xs font-medium text-text-2">To</label>
          <input id="audit-to" type="date" bind:value={filters.to_date} onchange={refresh} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
      </div>
    {/if}

    <div class="overflow-x-auto">
      <table class="min-w-full divide-y divide-border-1 text-sm">
        <thead class="bg-bg-surface-2 text-left text-xs uppercase tracking-[0.18em] text-text-3">
          <tr><th class="px-5 py-3">Time</th><th class="px-5 py-3">Policy</th><th class="px-5 py-3">User</th><th class="px-5 py-3">Target</th><th class="px-5 py-3">Action</th><th class="px-5 py-3">Status</th></tr>
        </thead>
        <tbody class="divide-y divide-border-1">
          {#each $adminStore.membershipAuditLogs as log (log.id)}
            <tr>
              <td class="px-5 py-4 text-text-3">{new Date(log.created_at).toLocaleString()}</td>
              <td class="px-5 py-4 font-semibold text-text-1">{log.policy_name || 'Unknown'}</td>
              <td class="px-5 py-4 text-text-2">@{log.username || log.user_id.slice(0, 8)}</td>
              <td class="px-5 py-4 text-text-2">{log.target_type} {log.target_id.slice(0, 8)}...</td>
              <td class="px-5 py-4 capitalize text-text-2">{log.action}</td>
              <td class="px-5 py-4"><span class="rounded-full px-2 py-1 text-xs font-semibold {log.status === 'success' ? 'bg-success/10 text-success' : log.status === 'failed' ? 'bg-danger/10 text-danger' : 'bg-warning/10 text-warning'}">{log.status}</span></td>
            </tr>
          {:else}
            <tr><td colspan="6" class="px-5 py-10 text-center text-text-3">No audit logs found.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
</section>
