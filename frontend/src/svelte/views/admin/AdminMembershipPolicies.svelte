<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertCircle, Building2, CheckCircle, Edit3, Globe, Hash, Plus, RefreshCw, Save, Search, Shield, Trash2, Users, X } from 'lucide-svelte'
  import { adminStore, type AutoMembershipPolicyAudit, type PolicyWithTargets } from '../../stores/admin'
  import type { PolicyScopeType, PolicySourceType, PolicyTargetType, RoleMode } from '../../../api/membershipPolicies'

  let searchQuery = $state('')
  let filterScope = $state<'all' | 'global' | 'team'>('all')
  let filterEnabled = $state<'all' | 'true' | 'false'>('all')
  let auditOpen = $state(false)
  let auditPolicy = $state<PolicyWithTargets | null>(null)
  let auditLogs = $state<AutoMembershipPolicyAudit[]>([])
  let editorOpen = $state(false)
  let editingPolicy = $state<PolicyWithTargets | null>(null)
  let editorError = $state('')
  const editor = $state({
    name: '',
    description: '',
    scope_type: 'global' as PolicyScopeType,
    source_type: 'all_users' as PolicySourceType,
    enabled: true,
    priority: 100,
    source_config: '{}',
    targets: '',
  })

  const filteredPolicies = $derived(
    $adminStore.membershipPolicies.filter((policy) => {
      const matchesSearch = !searchQuery.trim()
        || policy.name.toLowerCase().includes(searchQuery.toLowerCase())
        || Boolean(policy.description?.toLowerCase().includes(searchQuery.toLowerCase()))
      return matchesSearch
    }),
  )

  onMount(() => {
    void fetchPolicies()
  })

  async function fetchPolicies() {
    await adminStore.fetchMembershipPolicies({
      scope_type: filterScope === 'all' ? undefined : filterScope,
      enabled: filterEnabled === 'all' ? undefined : filterEnabled === 'true',
    })
  }

  function sourceLabel(source: string) {
    return {
      all_users: 'All Users',
      auth_service: 'Auth Service',
      group: 'Group',
      role: 'Role',
      org: 'Organization',
    }[source] ?? source
  }

  function targetSummary(policy: PolicyWithTargets) {
    const teams = policy.targets.filter((target) => target.target_type === 'team').length
    const channels = policy.targets.filter((target) => target.target_type === 'channel').length
    const parts = []
    if (teams) parts.push(`${teams} team${teams > 1 ? 's' : ''}`)
    if (channels) parts.push(`${channels} channel${channels > 1 ? 's' : ''}`)
    return parts.join(', ') || 'No targets'
  }

  async function togglePolicy(policy: PolicyWithTargets) {
    await adminStore.updateMembershipPolicy(policy.id, { enabled: !policy.enabled })
  }

  async function deletePolicy(policy: PolicyWithTargets) {
    if (!window.confirm(`Delete "${policy.name}"? This action cannot be undone.`)) return
    await adminStore.deleteMembershipPolicy(policy.id)
  }

  async function openAudit(policy: PolicyWithTargets) {
    auditPolicy = policy
    auditOpen = true
    auditLogs = await adminStore.fetchPolicyAudit(policy.id)
  }

  function resetEditor() {
    editingPolicy = null
    editorError = ''
    editor.name = ''
    editor.description = ''
    editor.scope_type = 'global'
    editor.source_type = 'all_users'
    editor.enabled = true
    editor.priority = 100
    editor.source_config = '{}'
    editor.targets = ''
  }

  function openCreate() {
    resetEditor()
    editorOpen = true
  }

  function openEdit(policy: PolicyWithTargets) {
    editingPolicy = policy
    editorError = ''
    editor.name = policy.name
    editor.description = policy.description || ''
    editor.scope_type = policy.scope_type
    editor.source_type = policy.source_type
    editor.enabled = policy.enabled
    editor.priority = policy.priority
    editor.source_config = JSON.stringify(policy.source_config || {}, null, 2)
    editor.targets = policy.targets
      .map((target) => `${target.target_type}:${target.target_id}:${target.role_mode || 'member'}`)
      .join('\n')
    editorOpen = true
  }

  function parseTargets() {
    return editor.targets
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        const [target_type, target_id, role_mode = 'member'] = line.split(':')
        if (!['team', 'channel'].includes(target_type) || !target_id) {
          throw new Error('Targets must use team:<id>:member or channel:<id>:admin format.')
        }
        return {
          target_type: target_type as PolicyTargetType,
          target_id,
          role_mode: (role_mode === 'admin' ? 'admin' : 'member') as RoleMode,
        }
      })
  }

  async function savePolicy() {
    editorError = ''
    let sourceConfig: Record<string, unknown>
    try {
      sourceConfig = editor.source_config.trim() ? JSON.parse(editor.source_config) : {}
      const targets = parseTargets()
      if (!editor.name.trim()) {
        throw new Error('Policy name is required.')
      }
      const payload = {
        name: editor.name.trim(),
        description: editor.description.trim() || undefined,
        scope_type: editor.scope_type,
        source_type: editor.source_type,
        enabled: editor.enabled,
        priority: Number(editor.priority) || 100,
        source_config: sourceConfig,
        targets,
      }
      const saved = editingPolicy
        ? await adminStore.updateMembershipPolicy(editingPolicy.id, payload)
        : await adminStore.createMembershipPolicy(payload)
      if (saved) {
        editorOpen = false
        resetEditor()
      }
    } catch (error) {
      editorError = error instanceof Error ? error.message : 'Failed to save policy.'
    }
  }
</script>

<section class="space-y-6" data-testid="admin-membership-policies-page">
  <div class="flex flex-col gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Policy automation</p>
      <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Membership Policies</h1>
      <p class="mt-2 text-sm text-text-3">Manage automatic team and channel membership rules.</p>
    </div>
    <div class="flex gap-2">
      <button type="button" onclick={openCreate} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground">
        <Plus class="h-4 w-4" /> New Policy
      </button>
      <button type="button" onclick={fetchPolicies} class="inline-flex items-center gap-2 rounded-r-2 border border-border-2 px-4 py-2 text-sm font-semibold text-text-2">
        <RefreshCw class="h-4 w-4 {$adminStore.loading ? 'animate-spin' : ''}" /> Refresh
      </button>
    </div>
  </div>

  {#if $adminStore.error}
    <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
      <AlertCircle class="mr-2 inline h-4 w-4" /> {$adminStore.error}
    </div>
  {/if}

  <div class="flex flex-wrap gap-3 rounded-r-3 border border-border-1 bg-bg-surface-1 p-4 shadow-1">
    <div class="relative min-w-64 flex-1">
      <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-4" />
      <input bind:value={searchQuery} placeholder="Search policies..." class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 py-2 pl-9 pr-3 text-sm text-text-1" />
    </div>
    <select bind:value={filterScope} onchange={fetchPolicies} class="rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
      <option value="all">All Scopes</option>
      <option value="global">Global</option>
      <option value="team">Team</option>
    </select>
    <select bind:value={filterEnabled} onchange={fetchPolicies} class="rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
      <option value="all">All Status</option>
      <option value="true">Enabled</option>
      <option value="false">Disabled</option>
    </select>
  </div>

  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    {#if $adminStore.loading && !filteredPolicies.length}
      <div class="p-10 text-center text-text-3"><RefreshCw class="mx-auto mb-3 h-8 w-8 animate-spin" /> Loading policies...</div>
    {:else if !filteredPolicies.length}
      <div class="p-10 text-center text-text-3"><Shield class="mx-auto mb-3 h-12 w-12 text-text-4" /> No policies found.</div>
    {:else}
      <div class="divide-y divide-border-1">
        {#each filteredPolicies as policy (policy.id)}
          <article class="p-5 transition-standard hover:bg-bg-surface-2/60">
            <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-3">
                  {#if policy.scope_type === 'global'}<Globe class="h-5 w-5 text-text-4" />{:else}<Building2 class="h-5 w-5 text-text-4" />{/if}
                  <h2 class="font-semibold text-text-1">{policy.name}</h2>
                  <span class="rounded-full px-2 py-0.5 text-xs font-semibold {policy.enabled ? 'bg-success/10 text-success' : 'bg-bg-surface-2 text-text-3'}">
                    {policy.enabled ? 'Enabled' : 'Disabled'}
                  </span>
                  {#if policy.scope_type === 'team'}
                    <span class="rounded-full bg-brand/10 px-2 py-0.5 text-xs font-semibold text-brand">Team</span>
                  {/if}
                </div>
                {#if policy.description}
                  <p class="mt-2 text-sm text-text-3">{policy.description}</p>
                {/if}
                <div class="mt-3 flex flex-wrap items-center gap-4 text-sm text-text-3">
                  <span class="inline-flex items-center gap-1"><Users class="h-4 w-4" /> Applies to: {sourceLabel(policy.source_type)}</span>
                  <span class="inline-flex items-center gap-1"><Hash class="h-4 w-4" /> {targetSummary(policy)}</span>
                  <span>Priority: {policy.priority}</span>
                </div>
              </div>
              <div class="flex items-center gap-2">
                <button type="button" onclick={() => openAudit(policy)} class="rounded-r-2 border border-border-1 p-2 text-text-3 hover:text-brand" title="View audit log">
                  <AlertCircle class="h-4 w-4" />
                </button>
                <button type="button" onclick={() => togglePolicy(policy)} class="rounded-r-2 border border-border-1 p-2 {policy.enabled ? 'text-success' : 'text-text-3'}" title={policy.enabled ? 'Disable' : 'Enable'}>
                  <CheckCircle class="h-4 w-4" />
                </button>
                <button type="button" onclick={() => openEdit(policy)} class="rounded-r-2 border border-border-1 p-2 text-text-3 hover:text-brand" title="Edit">
                  <Edit3 class="h-4 w-4" />
                </button>
                <button type="button" onclick={() => deletePolicy(policy)} class="rounded-r-2 border border-danger/20 p-2 text-danger" title="Delete">
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </div>

  {#if auditOpen}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" role="dialog" aria-modal="true" aria-labelledby="policy-audit-title">
      <div class="max-h-[80vh] w-full max-w-4xl overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl">
        <div class="flex items-center justify-between border-b border-border-1 px-5 py-4">
          <div>
            <h2 id="policy-audit-title" class="font-semibold text-text-1">Audit Log: {auditPolicy?.name}</h2>
            <p class="text-xs text-text-3">Recent policy application events</p>
          </div>
          <button type="button" onclick={() => (auditOpen = false)} class="rounded-r-2 border border-border-1 px-3 py-1.5 text-sm text-text-2">Close</button>
        </div>
        <div class="max-h-[60vh] overflow-auto p-5">
          {#if !auditLogs.length}
            <div class="py-10 text-center text-sm text-text-3">No audit entries found.</div>
          {:else}
            <table class="min-w-full divide-y divide-border-1 text-sm">
              <thead class="text-left text-xs uppercase tracking-[0.18em] text-text-3">
                <tr><th class="pb-3">Time</th><th class="pb-3">User</th><th class="pb-3">Target</th><th class="pb-3">Action</th><th class="pb-3">Status</th></tr>
              </thead>
              <tbody class="divide-y divide-border-1">
                {#each auditLogs as log (log.id)}
                  <tr>
                    <td class="py-3 text-text-3">{new Date(log.created_at).toLocaleString()}</td>
                    <td class="py-3 font-mono text-xs">{log.user_id.slice(0, 8)}...</td>
                    <td class="py-3 text-text-2">{log.target_type} {log.target_id.slice(0, 8)}...</td>
                    <td class="py-3 capitalize text-text-2">{log.action}</td>
                    <td class="py-3"><span class="rounded-full px-2 py-1 text-xs font-semibold {log.status === 'success' ? 'bg-success/10 text-success' : log.status === 'failed' ? 'bg-danger/10 text-danger' : 'bg-warning/10 text-warning'}">{log.status}</span></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  {#if editorOpen}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" role="dialog" aria-modal="true" aria-labelledby="policy-editor-title">
      <form class="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl" onsubmit={(event) => { event.preventDefault(); void savePolicy() }}>
        <div class="flex items-center justify-between border-b border-border-1 px-5 py-4">
          <div>
            <h2 id="policy-editor-title" class="font-semibold text-text-1">{editingPolicy ? 'Edit Membership Policy' : 'Create Membership Policy'}</h2>
            <p class="text-xs text-text-3">Use target lines like `team:team-id:member` or `channel:channel-id:admin`.</p>
          </div>
          <button type="button" onclick={() => (editorOpen = false)} class="rounded-r-2 p-1.5 text-text-3 hover:bg-bg-surface-2"><X class="h-4 w-4" /></button>
        </div>
        <div class="space-y-4 overflow-y-auto p-5">
          {#if editorError}
            <div class="rounded-r-2 border border-danger/20 bg-danger/10 p-3 text-sm text-danger">{editorError}</div>
          {/if}
          <div>
            <label for="policy-name" class="mb-1 block text-xs font-medium text-text-2">Name</label>
            <input id="policy-name" bind:value={editor.name} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" required />
          </div>
          <div>
            <label for="policy-description" class="mb-1 block text-xs font-medium text-text-2">Description</label>
            <textarea id="policy-description" rows="2" bind:value={editor.description} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1"></textarea>
          </div>
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div>
              <label for="policy-scope" class="mb-1 block text-xs font-medium text-text-2">Scope</label>
              <select id="policy-scope" bind:value={editor.scope_type} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
                <option value="global">Global</option>
                <option value="team">Team</option>
              </select>
            </div>
            <div>
              <label for="policy-source" class="mb-1 block text-xs font-medium text-text-2">Source</label>
              <select id="policy-source" bind:value={editor.source_type} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
                <option value="all_users">All Users</option>
                <option value="auth_service">Auth Service</option>
                <option value="group">Group</option>
                <option value="role">Role</option>
                <option value="org">Organization</option>
              </select>
            </div>
            <div>
              <label for="policy-priority" class="mb-1 block text-xs font-medium text-text-2">Priority</label>
              <input id="policy-priority" type="number" bind:value={editor.priority} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
            </div>
            <label class="flex items-center gap-3 rounded-r-2 bg-bg-surface-2 p-3 text-sm font-medium text-text-2">
              <input type="checkbox" bind:checked={editor.enabled} class="h-4 w-4 rounded text-brand" />
              Enabled
            </label>
          </div>
          <div>
            <label for="policy-source-config" class="mb-1 block text-xs font-medium text-text-2">Source Config JSON</label>
            <textarea id="policy-source-config" rows="5" bind:value={editor.source_config} class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 font-mono text-sm text-text-1"></textarea>
          </div>
          <div>
            <label for="policy-targets" class="mb-1 block text-xs font-medium text-text-2">Targets</label>
            <textarea id="policy-targets" rows="5" bind:value={editor.targets} placeholder="team:team-id:member&#10;channel:channel-id:admin" class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 font-mono text-sm text-text-1"></textarea>
          </div>
        </div>
        <div class="flex justify-end gap-2 border-t border-border-1 px-5 py-4">
          <button type="button" onclick={() => (editorOpen = false)} class="rounded-r-2 border border-border-1 px-3 py-2 text-sm font-semibold text-text-2">Cancel</button>
          <button type="submit" disabled={$adminStore.loading} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-3 py-2 text-sm font-semibold text-brand-foreground disabled:opacity-50">
            <Save class="h-4 w-4" /> {editingPolicy ? 'Save Changes' : 'Create Policy'}
          </button>
        </div>
      </form>
    </div>
  {/if}
</section>
