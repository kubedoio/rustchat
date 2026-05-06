<script lang="ts">
  import { onMount } from 'svelte'
  import { Search, Users } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  let search = $state('')

  onMount(() => {
    void adminStore.fetchUsers({ per_page: 25, status: 'all' })
  })

  function submitSearch(event: SubmitEvent) {
    event.preventDefault()
    void adminStore.fetchUsers({ per_page: 25, status: 'all', search: search.trim() })
  }

  function roleClass(role: string) {
    if (role === 'system_admin') return 'border-danger/20 bg-danger/10 text-danger'
    if (role === 'org_admin') return 'border-brand/20 bg-brand/10 text-brand'
    return 'border-border-1 bg-bg-surface-2 text-text-2'
  }
</script>

<section class="space-y-5" data-testid="admin-users-page">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
      <div>
        <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Identity</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Users</h1>
        <p class="mt-2 text-sm text-text-3">Read-only user management route for the Svelte admin migration.</p>
      </div>
      <form class="flex w-full gap-2 md:w-80" onsubmit={submitSearch}>
        <label class="sr-only" for="admin-user-search">Search users</label>
        <input
          id="admin-user-search"
          bind:value={search}
          class="min-w-0 flex-1 rounded-r-2 border border-border-1 bg-bg-surface-2 px-3 py-2 text-sm text-text-1 outline-none focus:border-brand"
          placeholder="Search users"
        />
        <button type="submit" class="rounded-r-2 bg-brand px-3 py-2 text-sm font-semibold text-brand-foreground">
          <Search class="h-4 w-4" />
        </button>
      </form>
    </div>
  </div>

  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <div class="flex items-center gap-2 border-b border-border-1 px-5 py-4 text-sm font-semibold text-text-1">
      <Users class="h-4 w-4 text-brand" />
      {$adminStore.users.total || $adminStore.users.items.length} users
    </div>
    <div class="overflow-x-auto">
      <table class="min-w-full divide-y divide-border-1 text-sm">
        <thead class="bg-bg-surface-2 text-left text-[11px] uppercase tracking-[0.18em] text-text-3">
          <tr>
            <th class="px-5 py-3 font-semibold">User</th>
            <th class="px-5 py-3 font-semibold">Role</th>
            <th class="px-5 py-3 font-semibold">Status</th>
            <th class="px-5 py-3 font-semibold">Last Login</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-1">
          {#each $adminStore.users.items as user (user.id)}
            <tr class="hover:bg-bg-surface-2/60">
              <td class="px-5 py-4">
                <p class="font-semibold text-text-1">{user.display_name || user.username}</p>
                <p class="text-xs text-text-3">{user.email}</p>
              </td>
              <td class="px-5 py-4">
                <span class="{roleClass(user.role)} rounded-full border px-2 py-1 text-xs font-semibold">{user.role}</span>
              </td>
              <td class="px-5 py-4 text-text-2">{user.is_active ? 'Active' : 'Inactive'}</td>
              <td class="px-5 py-4 text-text-3">{user.last_login_at ? new Date(user.last_login_at).toLocaleString() : 'Never'}</td>
            </tr>
          {:else}
            <tr>
              <td colspan="4" class="px-5 py-10 text-center text-text-3">
                {$adminStore.loading ? 'Loading users...' : 'No users found.'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</section>
