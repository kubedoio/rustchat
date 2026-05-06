<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertTriangle, CheckCircle, Edit2, Eye, FileText, Plus, Save, Trash2, Users, X } from 'lucide-svelte'
  import { adminStore, type TermsOfService } from '../../stores/admin'

  let formOpen = $state(false)
  let previewOpen = $state(false)
  let pendingOpen = $state(false)
  let editingTerms = $state<TermsOfService | null>(null)
  let previewTerms = $state<TermsOfService | null>(null)
  const form = $state({
    version: '',
    title: '',
    content: '',
    summary: '',
    effective_date: new Date().toISOString().slice(0, 10),
  })

  const currentTerms = $derived($adminStore.terms.find((terms) => terms.is_active) ?? null)

  onMount(() => {
    void adminStore.fetchTerms()
  })

  function formatDate(date: string) {
    return new Date(date).toLocaleDateString()
  }

  function resetForm() {
    editingTerms = null
    form.version = ''
    form.title = ''
    form.content = ''
    form.summary = ''
    form.effective_date = new Date().toISOString().slice(0, 10)
  }

  function openCreate() {
    resetForm()
    formOpen = true
  }

  function openEdit(terms: TermsOfService) {
    editingTerms = terms
    form.version = terms.version
    form.title = terms.title
    form.content = terms.content
    form.summary = terms.summary || ''
    form.effective_date = new Date(terms.effective_date).toISOString().slice(0, 10)
    formOpen = true
  }

  function openPreview(terms: TermsOfService) {
    previewTerms = terms
    previewOpen = true
  }

  async function saveTerms() {
    const ok = editingTerms
      ? await adminStore.updateTerms(editingTerms.id, form)
      : await adminStore.createTerms(form)
    if (ok) {
      formOpen = false
      resetForm()
    }
  }

  async function activateTerms(terms: TermsOfService) {
    if (!window.confirm(`Activate "${terms.title}"? Users will need to accept the active terms.`)) return
    await adminStore.activateTerms(terms.id)
  }

  async function deleteTerms(terms: TermsOfService) {
    if (terms.is_active) {
      window.alert('Cannot delete active terms.')
      return
    }
    if (!window.confirm(`Delete "${terms.title}"? This action cannot be undone.`)) return
    await adminStore.deleteTerms(terms.id)
  }
</script>

<section class="space-y-5" data-testid="admin-terms-settings-page">
  <div class="flex flex-col gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Legal acceptance</p>
      <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Terms of Service</h1>
      <p class="mt-2 text-sm text-text-3">Manage terms versions and track user acceptance status.</p>
    </div>
    <button type="button" onclick={openCreate} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground">
      <Plus class="h-4 w-4" /> New Terms Version
    </button>
  </div>

  {#if $adminStore.error}
    <div class="flex items-center gap-2 rounded-r-2 border border-danger/20 bg-danger/10 p-3 text-sm text-danger" role="alert">
      <AlertTriangle class="h-4 w-4" /> {$adminStore.error}
    </div>
  {/if}

  {#if $adminStore.termsStats}
    <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
      <article class="rounded-r-2 border border-border-1 bg-bg-surface-1 p-4 shadow-1">
        <div class="mb-1 flex items-center gap-2 text-text-3"><Users class="h-3.5 w-3.5" /><span class="text-[10px] uppercase tracking-wider">Total Users</span></div>
        <p class="text-2xl font-semibold text-text-1">{$adminStore.termsStats.total_users}</p>
      </article>
      <article class="rounded-r-2 border border-border-1 bg-bg-surface-1 p-4 shadow-1">
        <div class="mb-1 flex items-center gap-2 text-success"><CheckCircle class="h-3.5 w-3.5" /><span class="text-[10px] uppercase tracking-wider">Accepted</span></div>
        <p class="text-2xl font-semibold text-success">{$adminStore.termsStats.accepted_count}</p>
      </article>
      <article class="rounded-r-2 border border-border-1 bg-bg-surface-1 p-4 shadow-1">
        <div class="mb-1 flex items-center gap-2 text-warning"><AlertTriangle class="h-3.5 w-3.5" /><span class="text-[10px] uppercase tracking-wider">Pending</span></div>
        <p class="text-2xl font-semibold text-warning">{$adminStore.termsStats.pending_count}</p>
        {#if ($adminStore.termsStats.pending_users?.length ?? 0) > 0}
          <button type="button" onclick={() => (pendingOpen = true)} class="mt-1 text-[10px] text-brand hover:underline">View users</button>
        {/if}
      </article>
      <article class="rounded-r-2 border border-border-1 bg-bg-surface-1 p-4 shadow-1">
        <div class="mb-1 flex items-center gap-2 text-brand"><FileText class="h-3.5 w-3.5" /><span class="text-[10px] uppercase tracking-wider">Acceptance Rate</span></div>
        <p class="text-2xl font-semibold text-brand">{$adminStore.termsStats.acceptance_rate.toFixed(1)}%</p>
      </article>
    </div>
  {/if}

  {#if currentTerms}
    <section class="rounded-r-3 border border-brand/20 bg-brand/5 p-5">
      <div class="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div class="flex items-start gap-3">
          <div class="rounded-r-2 bg-brand/10 p-2 text-brand"><CheckCircle class="h-4 w-4" /></div>
          <div>
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="font-semibold text-text-1">{currentTerms.title}</h2>
              <span class="rounded bg-brand px-1.5 py-0.5 text-[9px] font-semibold text-brand-foreground">Active</span>
            </div>
            <p class="mt-1 text-xs text-text-3">Version {currentTerms.version} · Effective {formatDate(currentTerms.effective_date)}</p>
            {#if currentTerms.summary}<p class="mt-2 text-sm text-text-2">{currentTerms.summary}</p>{/if}
          </div>
        </div>
        <div class="flex gap-2">
          <button type="button" onclick={() => openPreview(currentTerms)} class="rounded-r-2 border border-border-1 p-2 text-text-3 hover:text-brand"><Eye class="h-4 w-4" /></button>
          <button type="button" onclick={() => openEdit(currentTerms)} class="rounded-r-2 border border-border-1 p-2 text-text-3 hover:text-brand"><Edit2 class="h-4 w-4" /></button>
        </div>
      </div>
    </section>
  {/if}

  <section class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <div class="border-b border-border-1 px-5 py-4">
      <h2 class="font-semibold text-text-1">All Versions</h2>
    </div>
    {#if $adminStore.loading && !$adminStore.terms.length}
      <div class="p-10 text-center text-text-3">Loading terms...</div>
    {:else if !$adminStore.terms.length}
      <div class="p-10 text-center text-text-3">No terms of service defined yet.</div>
    {:else}
      <div class="divide-y divide-border-1">
        {#each $adminStore.terms as terms (terms.id)}
          <article class="flex flex-col gap-4 px-5 py-4 transition-standard hover:bg-bg-surface-2/60 md:flex-row md:items-center md:justify-between">
            <div class="flex items-start gap-3">
              <div class="rounded-r-2 p-2 {terms.is_active ? 'bg-brand/10 text-brand' : 'bg-bg-surface-2 text-text-3'}"><FileText class="h-4 w-4" /></div>
              <div>
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="text-sm font-semibold text-text-1">{terms.title}</h3>
                  {#if terms.is_active}<span class="rounded bg-brand px-1.5 py-0.5 text-[9px] font-semibold text-brand-foreground">Active</span>{/if}
                </div>
                <p class="mt-1 text-xs text-text-3">Version {terms.version} · Created {formatDate(terms.created_at)}</p>
              </div>
            </div>
            <div class="flex items-center gap-1">
              {#if !terms.is_active}
                <button type="button" onclick={() => activateTerms(terms)} class="rounded-r-2 px-2 py-1 text-xs font-semibold text-brand hover:bg-brand/10">Activate</button>
              {/if}
              <button type="button" onclick={() => openPreview(terms)} class="rounded-r-2 p-2 text-text-3 hover:text-brand"><Eye class="h-4 w-4" /></button>
              <button type="button" onclick={() => openEdit(terms)} class="rounded-r-2 p-2 text-text-3 hover:text-brand"><Edit2 class="h-4 w-4" /></button>
              <button type="button" onclick={() => deleteTerms(terms)} class="rounded-r-2 p-2 text-text-3 hover:text-danger"><Trash2 class="h-4 w-4" /></button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>

  {#if formOpen}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" role="dialog" aria-modal="true" aria-labelledby="terms-form-title">
      <form class="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl" onsubmit={(event) => { event.preventDefault(); void saveTerms() }}>
        <div class="flex items-center justify-between border-b border-border-1 px-5 py-4">
          <h2 id="terms-form-title" class="font-semibold text-text-1">{editingTerms ? 'Edit Terms' : 'Create Terms Version'}</h2>
          <button type="button" onclick={() => (formOpen = false)} class="rounded-r-2 p-1.5 text-text-3 hover:bg-bg-surface-2"><X class="h-4 w-4" /></button>
        </div>
        <div class="space-y-4 overflow-y-auto p-5">
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div>
              <label for="terms-version" class="mb-1 block text-xs font-medium text-text-2">Version</label>
              <input id="terms-version" bind:value={form.version} disabled={Boolean(editingTerms)} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 disabled:opacity-50" required />
            </div>
            <div>
              <label for="terms-effective" class="mb-1 block text-xs font-medium text-text-2">Effective Date</label>
              <input id="terms-effective" type="date" bind:value={form.effective_date} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" required />
            </div>
          </div>
          <div>
            <label for="terms-title" class="mb-1 block text-xs font-medium text-text-2">Title</label>
            <input id="terms-title" bind:value={form.title} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" required />
          </div>
          <div>
            <label for="terms-summary" class="mb-1 block text-xs font-medium text-text-2">Summary</label>
            <input id="terms-summary" bind:value={form.summary} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
          </div>
          <div>
            <label for="terms-content" class="mb-1 block text-xs font-medium text-text-2">Content</label>
            <textarea id="terms-content" rows="10" bind:value={form.content} class="w-full resize-none rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 font-mono text-sm text-text-1" required></textarea>
          </div>
        </div>
        <div class="flex justify-end gap-2 border-t border-border-1 px-5 py-4">
          <button type="button" onclick={() => (formOpen = false)} class="rounded-r-2 border border-border-1 px-3 py-2 text-sm font-semibold text-text-2">Cancel</button>
          <button type="submit" disabled={$adminStore.loading} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-3 py-2 text-sm font-semibold text-brand-foreground disabled:opacity-50">
            <Save class="h-4 w-4" /> {editingTerms ? 'Save Changes' : 'Create Version'}
          </button>
        </div>
      </form>
    </div>
  {/if}

  {#if previewOpen && previewTerms}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" role="dialog" aria-modal="true" aria-labelledby="terms-preview-title">
      <div class="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl">
        <div class="flex items-center justify-between border-b border-border-1 px-5 py-4">
          <div><h2 id="terms-preview-title" class="font-semibold text-text-1">{previewTerms.title}</h2><p class="text-xs text-text-3">Version {previewTerms.version}</p></div>
          <button type="button" onclick={() => (previewOpen = false)} class="rounded-r-2 p-1.5 text-text-3 hover:bg-bg-surface-2"><X class="h-4 w-4" /></button>
        </div>
        <div class="overflow-y-auto p-5">
          {#if previewTerms.summary}<div class="mb-4 rounded-r-2 bg-bg-surface-2 p-3 text-sm text-text-2">{previewTerms.summary}</div>{/if}
          <pre class="whitespace-pre-wrap font-sans text-sm leading-6 text-text-1">{previewTerms.content}</pre>
        </div>
      </div>
    </div>
  {/if}

  {#if pendingOpen && $adminStore.termsStats?.pending_users}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" role="dialog" aria-modal="true" aria-labelledby="pending-users-title">
      <div class="max-h-[80vh] w-full max-w-lg overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl">
        <div class="flex items-center justify-between border-b border-border-1 px-5 py-4">
          <div><h2 id="pending-users-title" class="font-semibold text-text-1">Users Pending Acceptance</h2><p class="text-xs text-text-3">{$adminStore.termsStats.pending_users.length} users have not accepted the terms.</p></div>
          <button type="button" onclick={() => (pendingOpen = false)} class="rounded-r-2 p-1.5 text-text-3 hover:bg-bg-surface-2"><X class="h-4 w-4" /></button>
        </div>
        <div class="max-h-[60vh] divide-y divide-border-1 overflow-y-auto">
          {#each $adminStore.termsStats.pending_users as user (user.id)}
            <article class="flex items-center gap-3 px-5 py-3">
              <div class="flex h-8 w-8 items-center justify-center rounded-full bg-brand/10 text-xs font-bold text-brand">{(user.username?.charAt(0) || 'U').toUpperCase()}</div>
              <div class="min-w-0 flex-1"><p class="text-sm font-semibold text-text-1">{user.display_name || user.username}</p><p class="truncate text-xs text-text-3">{user.email}</p></div>
              <span class="text-xs text-text-4">{formatDate(user.created_at)}</span>
            </article>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</section>
