<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertCircle, CheckCircle, Download, Save, Scale, Trash2 } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  const form = $state({
    message_retention_days: 0,
    file_retention_days: 0,
  })
  let saving = $state(false)
  let saved = $state(false)
  let exporting = $state(false)
  let exportStarted = $state(false)

  onMount(async () => {
    await adminStore.fetchConfig()
    if ($adminStore.config?.compliance) {
      Object.assign(form, $adminStore.config.compliance)
    }
  })

  async function saveSettings() {
    saving = true
    saved = false
    try {
      saved = await adminStore.updateConfig('compliance', form)
    } finally {
      saving = false
    }
  }

  async function startExport() {
    exporting = true
    exportStarted = false
    try {
      exportStarted = await adminStore.startComplianceExport()
    } finally {
      exporting = false
    }
  }
</script>

<section class="space-y-6" data-testid="admin-compliance-settings-page">
  <div class="flex flex-col gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Retention and export</p>
      <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Compliance & Retention</h1>
      <p class="mt-2 text-sm text-text-3">Configure data retention policies and start compliance exports.</p>
    </div>
    <div class="flex items-center gap-3">
      {#if saved}
        <span class="inline-flex items-center gap-1 text-sm font-medium text-success"><CheckCircle class="h-4 w-4" /> Saved</span>
      {/if}
      <button type="button" onclick={saveSettings} disabled={saving} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground disabled:opacity-50">
        <Save class="h-4 w-4" /> {saving ? 'Saving...' : 'Save Changes'}
      </button>
    </div>
  </div>

  {#if $adminStore.error}
    <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
      <AlertCircle class="mr-2 inline h-4 w-4" /> {$adminStore.error}
    </div>
  {/if}

  <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <div class="mb-5 flex items-center gap-2">
      <Scale class="h-5 w-5 text-text-4" />
      <h2 class="text-lg font-semibold text-text-1">Global Retention Policy</h2>
    </div>
    <div class="grid grid-cols-1 gap-5 md:grid-cols-2">
      <div>
        <label for="message-retention" class="mb-1 block text-sm font-medium text-text-2">Message Retention (days)</label>
        <input id="message-retention" type="number" min="0" bind:value={form.message_retention_days} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        <p class="mt-1 text-xs text-text-3">0 keeps messages forever.</p>
      </div>
      <div>
        <label for="file-retention" class="mb-1 block text-sm font-medium text-text-2">File Retention (days)</label>
        <input id="file-retention" type="number" min="0" bind:value={form.file_retention_days} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        <p class="mt-1 text-xs text-text-3">0 keeps files forever.</p>
      </div>
    </div>

    {#if form.message_retention_days > 0 || form.file_retention_days > 0}
      <div class="mt-6 flex items-start gap-3 rounded-r-2 border border-warning/20 bg-warning/10 p-4 text-warning">
        <Trash2 class="mt-0.5 h-5 w-5 shrink-0" />
        <div>
          <p class="font-semibold">Data deletion warning</p>
          <p class="mt-1 text-sm">Data older than the configured retention period may be permanently deleted.</p>
        </div>
      </div>
    {/if}
  </section>

  <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <div class="mb-4 flex items-center gap-2">
      <Download class="h-5 w-5 text-text-4" />
      <h2 class="text-lg font-semibold text-text-1">Compliance Export</h2>
    </div>
    <p class="mb-4 text-sm text-text-3">Generate an export of system data for audit or legal review. The backend handles the export asynchronously.</p>
    {#if exportStarted}
      <div class="mb-4 rounded-r-2 border border-success/20 bg-success/10 p-3 text-sm text-success">Compliance export started successfully.</div>
    {/if}
    <button type="button" onclick={startExport} disabled={exporting} class="rounded-r-2 border border-border-2 px-4 py-2 text-sm font-semibold text-text-2 transition-standard hover:bg-bg-surface-2 disabled:opacity-50">
      {exporting ? 'Starting Export...' : 'Start Compliance Export'}
    </button>
  </section>
</section>
