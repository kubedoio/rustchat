<script lang="ts">
  import { onMount } from 'svelte'
  import { adminStore } from '../../stores/admin'

  onMount(() => {
    void adminStore.fetchAuditLogs({ per_page: 50 })
  })
</script>

<section class="space-y-5" data-testid="admin-audit-page">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Administrative trail</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Audit Logs</h1>
    <p class="mt-2 text-sm text-text-3">Review recent administrative actions from the Svelte route shell.</p>
  </div>

  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <div class="overflow-x-auto">
      <table class="min-w-full divide-y divide-border-1 text-sm">
        <thead class="bg-bg-surface-2 text-left text-[11px] uppercase tracking-[0.18em] text-text-3">
          <tr>
            <th class="px-5 py-3 font-semibold">Action</th>
            <th class="px-5 py-3 font-semibold">Target</th>
            <th class="px-5 py-3 font-semibold">Actor IP</th>
            <th class="px-5 py-3 font-semibold">Created</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-1">
          {#each $adminStore.auditLogs as log (log.id)}
            <tr class="hover:bg-bg-surface-2/60">
              <td class="px-5 py-4 font-semibold text-text-1">{log.action}</td>
              <td class="px-5 py-4 text-text-2">{log.target_type}{log.target_id ? ` · ${log.target_id}` : ''}</td>
              <td class="px-5 py-4 text-text-3">{log.actor_ip || '—'}</td>
              <td class="px-5 py-4 text-text-3">{new Date(log.created_at).toLocaleString()}</td>
            </tr>
          {:else}
            <tr>
              <td colspan="4" class="px-5 py-10 text-center text-text-3">
                {$adminStore.loading ? 'Loading audit logs...' : 'No audit logs found.'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</section>
