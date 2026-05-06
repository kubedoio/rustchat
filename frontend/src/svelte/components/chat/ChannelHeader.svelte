<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, fly } from 'svelte/transition'
  import { Hash, Lock, MoreVertical, Info, Users, Search, ClipboardList, Pin, Bookmark, Phone, PanelLeft } from 'lucide-svelte'
  import ConnectionIndicator from '../ui/ConnectionIndicator.svelte'
  import NotificationsDropdown from '../ui/NotificationsDropdown.svelte'
  import { callsStore } from '../../stores/calls.svelte'
  import type { SvelteChatChannel, SvelteChatMember } from '../../stores/chat'

  export let channel: SvelteChatChannel | null = null
  export let members: SvelteChatMember[] = []
  export let onToggleInfo: (() => void) | undefined = undefined
  export let onToggleMembers: (() => void) | undefined = undefined
  export let onToggleMobileSidebar: (() => void) | undefined = undefined

  const dispatch = createEventDispatcher<{
    toggleInfo: void
    search: void
    toggleActivity: void
    togglePinned: void
    toggleSaved: void
    startCall: void
    toggleMembers: void
    toggleMobileSidebar: void
  }>()

  let showMenu = false

  $: activeCall = callsStore.currentCall
  $: isInCall = activeCall?.channelId === channel?.id
  $: hasActiveCall = callsStore.activeCalls.has(channel?.id ?? '') && !isInCall

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

  function handleTogglePinned() {
    dispatch('togglePinned')
    showMenu = false
  }

  function handleToggleSaved() {
    dispatch('toggleSaved')
    showMenu = false
  }

  function handleMenuKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && showMenu) {
      showMenu = false
    }
  }

  function handleStartCall() {
    dispatch('startCall')
    showMenu = false
  }

  function handleToggleMembers() {
    onToggleMembers?.()
    dispatch('toggleMembers')
  }

  function handleToggleMobileSidebar() {
    onToggleMobileSidebar?.()
    dispatch('toggleMobileSidebar')
  }
</script>

<svelte:window onkeydown={handleMenuKeydown} />

<header
  class="sticky top-0 z-10 flex h-14 shrink-0 items-center justify-between border-b border-border-1 bg-bg-surface-1/95 px-3 backdrop-blur-sm sm:px-4"
>
  <!-- Left: Channel Info -->
  <div class="flex min-w-0 items-center gap-2.5">
    <!-- Mobile sidebar toggle -->
    <button
      class="lg:hidden flex h-9 w-9 items-center justify-center rounded-r-2 text-text-2 transition-standard hover:bg-bg-surface-2"
      onclick={handleToggleMobileSidebar}
      aria-label="Toggle sidebar"
      title="Toggle sidebar"
    >
      <PanelLeft class="h-5 w-5" />
    </button>

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
        {#if channel?.header}
          <p class="hidden sm:block text-xs text-text-3 truncate max-w-[300px]">{channel.header}</p>
        {/if}
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
    <!-- Members button -->
    <button
      class="hidden md:flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
      onclick={handleToggleMembers}
      aria-label="Members"
      title="Members"
    >
      <Users class="h-4 w-4" />
    </button>

    <!-- Call buttons -->
    {#if isInCall}
      <button
        class="flex h-11 w-11 items-center justify-center rounded-r-2 bg-success/10 text-success transition-standard"
        onclick={() => callsStore.toggleExpanded()}
        aria-label="Show call"
        title="Show call"
      >
        <Phone class="h-4 w-4" />
      </button>
    {:else if hasActiveCall}
      <button
        class="flex h-11 w-11 items-center justify-center rounded-r-2 bg-success/10 text-success animate-pulse transition-standard"
        onclick={() => channel?.id && callsStore.joinCall(channel.id)}
        aria-label="Join call"
        title="Join call"
      >
        <Phone class="h-4 w-4" />
      </button>
    {:else}
      <button
        class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
        onclick={handleStartCall}
        aria-label="Start audio call"
        title="Start audio call"
      >
        <Phone class="h-4 w-4" />
      </button>
    {/if}

    <button
      data-testid="search-button"
      onclick={() => dispatch('search')}
      class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
      aria-label="Search"
      title="Search"
    >
      <Search class="h-4 w-4" />
    </button>

    <ConnectionIndicator />

    <NotificationsDropdown />

    <button
      onclick={handleTogglePinned}
      class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
      aria-label="Pinned messages"
      title="Pinned messages"
    >
      <Pin class="h-4 w-4" />
    </button>

    <button
      onclick={handleToggleSaved}
      class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
      aria-label="Saved messages"
      title="Saved messages"
    >
      <Bookmark class="h-4 w-4" />
    </button>

    <button
      onclick={handleToggleActivity}
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
        onclick={() => (showMenu = !showMenu)}
        class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
        class:bg-bg-surface-2={showMenu}
        title="More options"
        aria-label="More options"
      >
        <MoreVertical class="h-4 w-4" />
      </button>

      {#if showMenu}
        <!-- Click outside -->
        <div class="fixed inset-0 z-10" onclick={() => (showMenu = false)} role="presentation" transition:fade={{ duration: 100 }}></div>
        <div
          class="absolute right-0 top-full z-20 mt-2 w-48 origin-top-right rounded-r-2 border border-border-1 bg-bg-surface-1 py-1 shadow-2xl"
          role="menu"
          transition:fly={{ duration: 150, y: -5 }}
        >
          <button
            data-testid="channel-details-button"
            onclick={handleToggleInfo}
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm text-text-2 transition-standard hover:bg-bg-surface-2"
            role="menuitem"
          >
            <Info class="h-4 w-4" />
            Channel Details
          </button>

          <button
            onclick={handleToggleMembers}
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm text-text-2 transition-standard hover:bg-bg-surface-2"
            role="menuitem"
          >
            <Users class="h-4 w-4" />
            Members
          </button>

          <button
            onclick={handleTogglePinned}
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm text-text-2 transition-standard hover:bg-bg-surface-2"
            role="menuitem"
          >
            <Pin class="h-4 w-4" />
            Pinned Messages
          </button>

          <button
            onclick={handleToggleSaved}
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm text-text-2 transition-standard hover:bg-bg-surface-2"
            role="menuitem"
          >
            <Bookmark class="h-4 w-4" />
            Saved Messages
          </button>

          <button
            onclick={handleStartCall}
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm text-text-2 transition-standard hover:bg-bg-surface-2"
            role="menuitem"
          >
            <Phone class="h-4 w-4" />
            Start Call
          </button>
        </div>
      {/if}
    </div>
  </div>
</header>
