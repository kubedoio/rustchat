<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { Hash, Lock, MoreVertical, Info, Users, Search, ClipboardList } from 'lucide-svelte'
  import ConnectionIndicator from '../ui/ConnectionIndicator.svelte'
  import NotificationsDropdown from '../ui/NotificationsDropdown.svelte'
  import type { SvelteChatChannel, SvelteChatMember } from '../../stores/chat'

  export let channel: SvelteChatChannel | null = null
  export let members: SvelteChatMember[] = []
  export let onToggleInfo: (() => void) | undefined = undefined

  const dispatch = createEventDispatcher<{ toggleInfo: void; search: void; toggleActivity: void }>()

  let showMenu = false

  function channelTypeLabel(type: string): string {
    const labels: Record<string, string> = {
      public: 'PUBLIC CHANNEL',
      private: 'PRIVATE CHANNEL',
      direct: 'DIRECT MESSAGE',
      group: 'GROUP MESSAGE',
    }
    return labels[type] || 'CHANNEL'
  }

  function channelPrefix(type: string): string {
    return type === 'public' || type === 'private' ? '#' : ''
  }

  $: isDirect = channel?.channel_type === 'direct'
  $: otherMember = isDirect ? members[0] ?? null : null

  function handleToggleInfo() {
    onToggleInfo?.()
    dispatch('toggleInfo')
    showMenu = false
  }

  function handleToggleActivity() {
    dispatch('toggleActivity')
  }
</script>

<header
  class="sticky top-0 z-10 flex h-14 shrink-0 items-center justify-between border-b border-border-1 bg-bg-surface-1/95 px-3 backdrop-blur-sm sm:px-4"
>
  <!-- Left: Channel Info -->
  <div class="flex min-w-0 items-center gap-2.5">
    <div class="flex min-w-0 items-center gap-2">
      <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-r-1 bg-brand/10 text-brand">
        {#if channel?.channel_type === 'private'}
          <Lock class="h-4 w-4" />
        {:else}
          <Hash class="h-4 w-4" />
        {/if}
      </div>
      <div class="min-w-0">
        <div class="truncate text-[10px] font-semibold uppercase tracking-[0.18em] text-brand/70">
          {channelTypeLabel(channel?.channel_type ?? '')}
        </div>
        <h1 class="truncate text-sm font-semibold text-brand sm:text-base">
          {channelPrefix(channel?.channel_type ?? '')}{channel?.display_name || channel?.name || 'No channel selected'}
        </h1>
      </div>
    </div>

    <!-- DM presence chip -->
    {#if isDirect && otherMember?.presence}
      <span
        class="hidden sm:inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium
        {otherMember.presence === 'online'
          ? 'border-success/30 bg-success/10 text-success'
          : otherMember.presence === 'away'
            ? 'border-warning/30 bg-warning/10 text-warning'
            : otherMember.presence === 'dnd'
              ? 'border-danger/30 bg-danger/10 text-danger'
              : 'border-border-2 bg-bg-surface-2 text-text-4'}"
      >
        <span
          class="h-1.5 w-1.5 rounded-full
          {otherMember.presence === 'online'
            ? 'bg-success'
            : otherMember.presence === 'away'
              ? 'bg-warning'
              : otherMember.presence === 'dnd'
                ? 'bg-danger'
                : 'bg-text-4'}"
        ></span>
        {otherMember.presence === 'online'
          ? 'Online'
          : otherMember.presence === 'away'
            ? 'Away'
            : otherMember.presence === 'dnd'
              ? 'Do not disturb'
              : 'Offline'}
      </span>
    {/if}
  </div>

  <!-- Right: Actions -->
  <div class="flex shrink-0 items-center gap-0.5 rounded-r-3 border border-border-1 bg-bg-surface-2/70 p-1 sm:gap-1">
    <button
      data-testid="search-button"
      on:click={() => dispatch('search')}
      class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
      aria-label="Search"
      title="Search"
    >
      <Search class="h-4 w-4" />
    </button>

    <ConnectionIndicator />

    <NotificationsDropdown />

    <button
      on:click={handleToggleActivity}
      class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
      aria-label="Activity feed"
      title="Activity feed"
    >
      <ClipboardList class="h-4 w-4" />
    </button>

    <!-- More Options Menu -->
    <div class="relative">
      <button
        data-testid="channel-header-menu"
        on:click={() => (showMenu = !showMenu)}
        class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
        class:bg-bg-surface-2={showMenu}
        title="More options"
        aria-label="More options"
      >
        <MoreVertical class="h-4 w-4" />
      </button>

      {#if showMenu}
        <div
          class="absolute right-0 top-full z-20 mt-2 w-48 origin-top-right rounded-r-2 border border-border-1 bg-bg-surface-1 py-1 shadow-2xl"
        >
          <button
            data-testid="channel-details-button"
            on:click={handleToggleInfo}
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm text-text-2 transition-standard hover:bg-bg-surface-2"
          >
            <Info class="h-4 w-4" />
            Channel Details
          </button>

          <button
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm text-text-2 transition-standard hover:bg-bg-surface-2"
          >
            <Users class="h-4 w-4" />
            Members
          </button>
        </div>

        <!-- Click outside -->
        <div class="fixed inset-0 z-10" on:click={() => (showMenu = false)} role="presentation"></div>
      {/if}
    </div>
  </div>
</header>
