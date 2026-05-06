<script lang="ts">
  import { Plus } from 'lucide-svelte'
  import { chatStore } from '../../stores/chat'
  import { uiStore } from '../../stores/ui'

  export let onSelectTeam: ((teamId: string) => void) | undefined = undefined
  export let onCreateTeam: (() => void) | undefined = undefined

  function getInitials(name: string) {
    return name.split(' ').map((n) => n[0]).filter(Boolean).join('').toUpperCase().slice(0, 2)
  }

  $: teams = $chatStore.teams ?? []
  $: currentTeamId = $chatStore.currentTeamId
</script>

<div class="flex flex-col items-center w-[var(--team-rail-width)] h-full bg-bg-surface-2 border-r border-border-1 py-3 gap-2 shrink-0">
  {#each teams as team (team.id)}
    <button
      class="relative w-10 h-10 rounded-r-2 flex items-center justify-center text-sm font-bold transition-standard
        {currentTeamId === team.id ? 'bg-brand text-brand-foreground shadow-1' : 'bg-bg-surface-1 text-text-1 border border-border-1 hover:bg-bg-surface-1/80'}"
      onclick={() => onSelectTeam?.(team.id)}
      title={team.display_name || team.name}
    >
      {getInitials(team.display_name || team.name)}
      {#if currentTeamId !== team.id && ($uiStore.unreadCountsByTeam?.[team.id] ?? 0) > 0}
        <span class="absolute -top-0.5 -right-0.5 w-3 h-3 bg-danger rounded-full border-2 border-bg-surface-2"></span>
      {/if}
      {#if currentTeamId === team.id}
        <span class="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-6 bg-brand rounded-r-full"></span>
      {/if}
    </button>
  {/each}

  <button
    class="w-10 h-10 rounded-r-2 flex items-center justify-center border border-dashed border-border-2 text-text-3 hover:text-brand hover:border-brand transition-standard mt-auto"
    onclick={() => onCreateTeam?.()}
    title="Add Team"
  >
    <Plus class="w-5 h-5" />
  </button>
</div>
