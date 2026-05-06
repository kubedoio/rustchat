<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X, Search } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi } from '../../stores/http'
  import { authStore } from '../../stores/auth'
  import { chatStore } from '../../stores/chat'
  import type { SvelteChatMember } from '../../stores/chat'

  export let open = false

  const dispatch = createEventDispatcher<{
    close: void
    select: string
  }>()

  let search = ''
  let loading = false
  let error = ''

  $: currentTeamId = (() => {
    const channel = $chatStore.channels.find((c) => c.id === $chatStore.currentChannelId)
    return channel?.team_id ?? $chatStore.teams[0]?.id ?? null
  })()

  $: if (open && currentTeamId) {
    if (!$chatStore.membersByTeam[currentTeamId]?.length) {
      chatStore.fetchMembers(currentTeamId)
    }
  }

  $: members = currentTeamId ? ($chatStore.membersByTeam[currentTeamId] ?? []) : []

  $: filteredMembers = ((): SvelteChatMember[] => {
    if (!members.length) return []
    const searchLower = search.toLowerCase()
    return members.filter((m) => {
      if (m.user_id === $authStore.user?.id) return false
      return (
        m.username.toLowerCase().includes(searchLower) ||
        (m.display_name && m.display_name.toLowerCase().includes(searchLower))
      )
    })
  })()

  function getInitials(name: string) {
    return name
      .split(' ')
      .map((n) => n[0])
      .filter(Boolean)
      .join('')
      .toUpperCase()
      .slice(0, 2)
  }

  async function startDM(member: SvelteChatMember) {
    if (!currentTeamId) return
    loading = true
    error = ''
    try {
      const { data } = await svelteApi.post<{ id: string }>('/channels', {
        team_id: currentTeamId,
        name: `dm_${member.user_id}`,
        display_name: member.display_name || member.username,
        channel_type: 'direct',
        target_user_id: member.user_id,
      })
      dispatch('select', data.id)
      handleClose()
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : 'Failed to start direct message'
      error = message
    } finally {
      loading = false
    }
  }

  function handleClose() {
    search = ''
    error = ''
    dispatch('close')
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose()
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center" data-testid="direct-message-modal" role="dialog" aria-modal="true">
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-bg-app/70 backdrop-blur-sm"
      on:click={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close modal"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal -->
    <div
      class="relative mx-4 flex max-h-[80vh] w-full max-w-md flex-col overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-border-1 px-6 py-4">
        <div>
          <h2 class="text-xl font-semibold text-brand">Direct Messages</h2>
          <p class="text-xs text-text-3">Start a private conversation in the current workspace.</p>
        </div>
        <button
          on:click={handleClose}
          class="rounded-r-2 p-1 transition-standard hover:bg-bg-surface-2"
          aria-label="Close"
        >
          <X class="h-5 w-5 text-text-3" />
        </button>
      </div>

      <!-- Search -->
      <div class="border-b border-border-1 px-6 py-4 bg-bg-surface-2/45">
        <div class="relative">
          <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-3" />
          <input
            bind:value={search}
            type="text"
            placeholder="Search for a team member..."
            class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 py-2 pl-10 pr-4 text-sm text-text-1 outline-none transition-standard focus:border-brand/50 focus:ring-2 focus:ring-brand/20"
            autofocus
          />
        </div>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-2 custom-scrollbar min-h-[200px]">
        <!-- Error -->
        {#if error}
          <div class="m-4 rounded-r-2 border border-danger/20 bg-danger/5 p-3 text-sm text-danger">
            {error}
          </div>
        {/if}

        <!-- Loading -->
        {#if $chatStore.loading && members.length === 0}
          <div class="flex flex-col items-center justify-center py-12 text-text-3">
            <div class="mb-4 h-8 w-8 animate-spin rounded-full border-4 border-brand/20 border-t-brand"></div>
            <p class="text-sm">Loading members...</p>
          </div>
        {:else if filteredMembers.length > 0}
          <!-- Members List -->
          <div class="space-y-1">
            {#each filteredMembers as member (member.user_id)}
              <button
                on:click={() => startDM(member)}
                disabled={loading}
                class="group flex w-full items-center rounded-r-2 border border-transparent px-4 py-3 text-left transition-standard hover:border-border-1 hover:bg-bg-surface-2/70"
              >
                <!-- Avatar -->
                <div
                  class="mr-4 flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-brand/10 text-sm font-bold text-brand"
                >
                  {getInitials(member.display_name || member.username)}
                </div>

                <!-- Name -->
                <div class="flex-1 min-w-0">
                  <p class="truncate text-sm font-semibold text-text-1 transition-standard group-hover:text-brand">
                    {member.display_name || member.username}
                  </p>
                  <p class="truncate text-xs text-text-3">
                    @{member.username}
                  </p>
                </div>
              </button>
            {/each}
          </div>
        {:else}
          <!-- Empty State -->
          <div class="flex flex-col items-center justify-center px-6 py-12 text-center text-text-3">
            <div class="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-bg-surface-2">
              <Search class="h-6 w-6 text-text-4" />
            </div>
            <p class="text-sm font-medium">No team members found</p>
            <p class="text-xs mt-1">Try a different search or invite someone to the team!</p>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex justify-end border-t border-border-1 bg-bg-surface-2/45 px-6 py-4">
        <button
          type="button"
          on:click={handleClose}
          disabled={loading}
          class="inline-flex items-center rounded-r-2 border border-border-2 bg-bg-surface-2 px-4 py-2 text-sm font-medium text-text-2 transition-standard hover:bg-bg-surface-1 disabled:opacity-50"
        >
          Cancel
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: var(--color-border-1);
    border-radius: 4px;
  }
</style>
