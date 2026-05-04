<script lang="ts">
  import { onMount } from 'svelte'
  import { Building2, Hash } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'

  onMount(() => {
    void adminStore.fetchTeams({ per_page: 25 })
    void adminStore.fetchChannels({ per_page: 25 })
  })
</script>

<section class="space-y-5" data-testid="admin-teams-page">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
    <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Workspace Structure</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Teams & Channels</h1>
    <p class="mt-2 text-sm text-text-3">Initial Svelte read path for team and channel administration.</p>
  </div>

  <div class="grid grid-cols-1 gap-5 xl:grid-cols-2">
    <section class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
      <div class="flex items-center gap-2 border-b border-border-1 px-5 py-4 text-sm font-semibold text-text-1">
        <Building2 class="h-4 w-4 text-brand" />
        Teams
      </div>
      <div class="divide-y divide-border-1">
        {#each $adminStore.teams.items as team (team.id)}
          <article class="px-5 py-4">
            <div class="flex items-start justify-between gap-3">
              <div>
                <h2 class="font-semibold text-text-1">{team.display_name || team.name}</h2>
                <p class="mt-1 text-sm text-text-3">{team.description || 'No description'}</p>
              </div>
              <span class="rounded-full border border-border-1 bg-bg-surface-2 px-2 py-1 text-xs text-text-2">
                {team.is_public ? 'Public' : 'Private'}
              </span>
            </div>
            <p class="mt-3 text-xs text-text-3">{team.members_count} members · {team.channels_count} channels</p>
          </article>
        {:else}
          <p class="px-5 py-10 text-center text-sm text-text-3">
            {$adminStore.loading ? 'Loading teams...' : 'No teams found.'}
          </p>
        {/each}
      </div>
    </section>

    <section class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
      <div class="flex items-center gap-2 border-b border-border-1 px-5 py-4 text-sm font-semibold text-text-1">
        <Hash class="h-4 w-4 text-brand" />
        Channels
      </div>
      <div class="divide-y divide-border-1">
        {#each $adminStore.channels.items as channel (channel.id)}
          <article class="px-5 py-4">
            <div class="flex items-start justify-between gap-3">
              <div>
                <h2 class="font-semibold text-text-1">{channel.display_name || channel.name}</h2>
                <p class="mt-1 text-sm text-text-3">{channel.purpose || 'No purpose set'}</p>
              </div>
              <span class="rounded-full border border-border-1 bg-bg-surface-2 px-2 py-1 text-xs text-text-2">
                {channel.channel_type}
              </span>
            </div>
            <p class="mt-3 text-xs text-text-3">{channel.members_count} members</p>
          </article>
        {:else}
          <p class="px-5 py-10 text-center text-sm text-text-3">
            {$adminStore.loading ? 'Loading channels...' : 'No channels found.'}
          </p>
        {/each}
      </div>
    </section>
  </div>
</section>
