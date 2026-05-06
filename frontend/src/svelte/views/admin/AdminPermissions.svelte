<script lang="ts">
  import { onMount } from 'svelte'
  import { Shield } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  onMount(() => {
    void adminStore.fetchPermissions()
  })

  const groupedPermissions = $derived(
    $adminStore.permissions.reduce<Record<string, typeof $adminStore.permissions>>((groups, permission) => {
      const category = permission.category || 'General'
      groups[category] = groups[category] ?? []
      groups[category].push(permission)
      return groups
    }, {}),
  )
</script>

<section class="space-y-5" data-testid="admin-permissions-page">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Access model</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Permissions</h1>
    <p class="mt-2 text-sm text-text-3">Read path for permission catalog parity.</p>
  </div>

  {#if Object.keys(groupedPermissions).length}
    <div class="grid grid-cols-1 gap-5 lg:grid-cols-2">
      {#each Object.entries(groupedPermissions) as [category, permissions] (category)}
        <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
          <div class="mb-4 flex items-center gap-2">
            <Shield class="h-4 w-4 text-brand" />
            <h2 class="font-semibold text-text-1">{category}</h2>
          </div>
          <ul class="space-y-3">
            {#each permissions as permission (permission.id)}
              <li class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-3">
                <p class="text-sm font-semibold text-text-1">{permission.id}</p>
                <p class="mt-1 text-xs text-text-3">{permission.description || 'No description'}</p>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {:else}
    <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-10 text-center text-sm text-text-3 shadow-1">
      {$adminStore.loading ? 'Loading permissions...' : 'No permissions found.'}
    </div>
  {/if}
</section>
