<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte'
  import { Bell, BellOff, Hash, ChevronRight } from 'lucide-svelte'
  import { chatStore } from '../../stores/chat'

  export let open: boolean | undefined = undefined
  export let onClose: (() => void) | undefined = undefined

  const dispatch = createEventDispatcher<{ close: void }>()

  let internalOpen = false

  $: isOpen = open !== undefined ? open : internalOpen

  $: unreadChannels = ($chatStore.channels ?? [])
    .map((channel) => {
      const unread = $chatStore.unreadCounts?.[channel.id] ?? 0
      const mentions = $chatStore.mentionCounts?.[channel.id] ?? 0
      return { ...channel, unread, mentions }
    })
    .filter((c) => c.unread > 0)
    .sort((a, b) => {
      if (a.mentions > 0 && b.mentions === 0) return -1
      if (b.mentions > 0 && a.mentions === 0) return 1
      return b.unread - a.unread
    })

  $: totalUnread = unreadChannels.reduce((sum, c) => sum + c.unread, 0)

  function handleToggle() {
    if (open !== undefined) {
      dispatch('close')
    } else {
      internalOpen = !internalOpen
    }
  }

  function close() {
    if (open !== undefined) {
      dispatch('close')
    } else {
      internalOpen = false
    }
    onClose?.()
  }

  async function markChannelRead(channelId: string) {
    await chatStore.markChannelRead(channelId)
    close()
  }

  async function markAllAsRead() {
    const channels = unreadChannels.map((c) => c.id)
    await Promise.all(channels.map((id) => chatStore.markChannelRead(id)))
    close()
  }

  function selectChannel(channelId: string) {
    chatStore.selectChannel(channelId)
    markChannelRead(channelId)
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      close()
    }
  }

  onMount(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeydown)
  })
</script>

<div class="relative">
  {#if open === undefined}
    <button
      data-testid="notifications-trigger"
      on:click={handleToggle}
      class="relative flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
      class:bg-bg-surface-2={isOpen}
      aria-label="Notifications"
      title="Notifications"
    >
      <Bell class="h-4 w-4" />
      {#if totalUnread > 0}
        <span
          class="absolute -top-0.5 -right-0.5 flex h-4 min-w-[16px] items-center justify-center rounded-full bg-danger px-1 text-[10px] font-bold text-white"
        >
          {totalUnread > 99 ? '99+' : totalUnread}
        </span>
      {/if}
    </button>
  {/if}

  {#if isOpen}
    <div
      class="absolute right-0 top-full z-40 mt-1 w-80 origin-top-right overflow-hidden rounded-r-2 border border-border-1 bg-bg-surface-1 shadow-2xl"
    >
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-border-1 bg-bg-surface-2/50 px-4 py-3">
        <h3 class="text-sm font-semibold text-text-1">Unread Activity</h3>
        {#if unreadChannels.length > 0}
          <button
            class="text-xs font-medium text-brand hover:text-brand-hover"
            on:click={markAllAsRead}
          >
            Mark all as read
          </button>
        {/if}
      </div>

      <!-- Channel list -->
      <div class="max-h-80 overflow-y-auto custom-scrollbar-thin">
        {#if unreadChannels.length === 0}
          <div class="flex flex-col items-center justify-center py-8 text-text-3">
            <BellOff class="mb-2 h-10 w-10 opacity-50" />
            <p class="text-sm">All caught up!</p>
          </div>
        {:else}
          <div class="divide-y divide-border-1">
            {#each unreadChannels as channel (channel.id)}
              <button
                class="flex w-full items-center gap-3 px-4 py-3 text-left transition-standard hover:bg-bg-surface-2"
                on:click={() => selectChannel(channel.id)}
              >
                <Hash class="h-5 w-5 shrink-0 text-text-3" />
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium text-text-1">
                    {channel.display_name || channel.name}
                  </p>
                </div>
                <div class="flex shrink-0 items-center gap-1.5">
                  {#if channel.mentions > 0}
                    <span
                      class="flex h-[18px] min-w-[18px] items-center justify-center rounded-full bg-danger px-1 text-[10px] font-bold text-white"
                    >
                      {channel.mentions}
                    </span>
                  {:else if channel.unread > 0}
                    <span
                      class="flex h-[18px] min-w-[18px] items-center justify-center rounded-full border border-border-1 bg-bg-surface-2 px-1 text-[10px] font-bold text-text-3"
                    >
                      {channel.unread > 99 ? '99+' : channel.unread}
                    </span>
                  {/if}
                  <ChevronRight class="h-4 w-4 text-text-4" />
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Click outside backdrop -->
    <div class="fixed inset-0 z-10" on:click={close} role="presentation"></div>
  {/if}
</div>
