<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { Search, Hash, Lock, User, Users } from 'lucide-svelte'
  import { matchSorter } from 'match-sorter'
  import { chatStore } from '../../stores/chat'

  export let open = false

  interface QuickSwitcherItem {
    id: string
    type: 'channel' | 'dm' | 'team' | 'user'
    name: string
    subtitle?: string
    icon: string
  }

  const dispatch = createEventDispatcher<{
    close: void
    select: QuickSwitcherItem
  }>()

  let query = ''
  let selectedIndex = 0
  let inputRef: HTMLInputElement

  const RECENT_KEY = 'qs_recent_svelte'

  function readRecentIds(): string[] {
    try {
      const saved = localStorage.getItem(RECENT_KEY)
      return saved ? JSON.parse(saved) : []
    } catch {
      return []
    }
  }

  function writeRecentIds(ids: string[]) {
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(ids))
    } catch {
      // ignore
    }
  }

  function getRecentItems(all: QuickSwitcherItem[]): QuickSwitcherItem[] {
    const recentIds = readRecentIds()
    return recentIds
      .map((id) => all.find((item) => item.id === id))
      .filter((item): item is QuickSwitcherItem => item !== undefined)
  }

  $: allItems = ((): QuickSwitcherItem[] => {
    const items: QuickSwitcherItem[] = []

    // Channels
    for (const channel of $chatStore.channels) {
      const isPrivate = channel.channel_type === 'private'
      items.push({
        id: `channel-${channel.id}`,
        type: 'channel',
        name: channel.display_name || channel.name,
        subtitle: undefined,
        icon: isPrivate ? 'Lock' : 'Hash',
      })
    }

    // Users from members
    const seenUserIds = new Set<string>()
    for (const members of Object.values($chatStore.membersByTeam)) {
      for (const member of members) {
        const userId = member.user_id
        if (!userId || seenUserIds.has(userId)) continue
        seenUserIds.add(userId)
        items.push({
          id: `user-${userId}`,
          type: 'user',
          name: member.display_name || member.username,
          subtitle: `@${member.username}`,
          icon: 'User',
        })
      }
    }

    return items
  })()

  $: filteredItems = ((): QuickSwitcherItem[] => {
    if (!query.trim()) return []
    return matchSorter(allItems, query.trim(), {
      keys: ['name', 'subtitle'],
      threshold: matchSorter.rankings.CONTAINS,
    }).slice(0, 8)
  })()

  $: displayedItems = query.trim() ? filteredItems : getRecentItems(allItems).slice(0, 6)

  $: if (open) {
    query = ''
    selectedIndex = 0
    // Defer focus until after render
    requestAnimationFrame(() => {
      inputRef?.focus()
    })
  }

  function selectItem(item: QuickSwitcherItem) {
    const recentIds = [item.id, ...readRecentIds().filter((id) => id !== item.id)].slice(0, 10)
    writeRecentIds(recentIds)
    dispatch('select', item)
    dispatch('close')
  }

  function handleKeydown(e: KeyboardEvent) {
    const items = displayedItems
    if (!items.length) return

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        selectedIndex = (selectedIndex + 1) % items.length
        break
      case 'ArrowUp':
        e.preventDefault()
        selectedIndex = (selectedIndex - 1 + items.length) % items.length
        break
      case 'Enter':
        e.preventDefault()
        if (items[selectedIndex]) {
          selectItem(items[selectedIndex])
        }
        break
      case 'Escape':
        e.preventDefault()
        dispatch('close')
        break
    }
  }

  const iconMap: Record<string, typeof Hash> = {
    Hash,
    Lock,
    User,
    Users,
  }

  function getIcon(iconName: string) {
    return iconMap[iconName] ?? Hash
  }
</script>

<svelte:window
  on:keydown={(e) => {
    if (open && e.key === 'Escape') {
      e.preventDefault()
      dispatch('close')
    }
  }}
/>

{#if open}
  <div
    class="fixed inset-0 z-[60] flex items-start justify-center bg-black/50 pt-[20vh]"
    data-testid="quick-switcher"
    on:click={() => dispatch('close')}
    on:keydown={(e) => e.key === 'Escape' && dispatch('close')}
    role="button"
    tabindex="-1"
    aria-label="Close quick switcher"
  >
    <div
      class="mx-4 w-full max-w-lg overflow-hidden rounded-xl bg-white shadow-2xl"
      on:click|stopPropagation
      on:keydown={(e) => e.key === 'Escape' && dispatch('close')}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
    >
      <!-- Input -->
      <div class="flex items-center gap-3 border-b border-gray-200 px-4 py-3.5">
        <Search class="h-5 w-5 flex-shrink-0 text-gray-400" />
        <input
          bind:this={inputRef}
          bind:value={query}
          type="text"
          placeholder="Jump to..."
          class="flex-1 bg-transparent text-base placeholder-gray-400 outline-none"
          data-testid="quick-switcher-input"
          on:keydown={handleKeydown}
        />
        <kbd class="hidden rounded bg-gray-100 px-2 py-0.5 font-mono text-xs text-gray-400 sm:block">ESC</kbd>
      </div>

      <!-- Results -->
      <div class="max-h-[360px] overflow-y-auto py-1">
        {#if displayedItems.length === 0}
          <div class="px-4 py-8 text-center text-sm text-gray-400">
            {#if query.trim()}
              <p>No results for "{query}"</p>
            {:else}
              <p>Start typing to search channels and teams</p>
            {/if}
          </div>
        {:else}
          {#if !query.trim()}
            <div class="px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-gray-400">Recent</div>
          {/if}
          {#each displayedItems as item, index (item.id)}
            <div
              class="flex cursor-pointer items-center gap-3 px-4 py-2.5 transition-colors"
              class:bg-blue-50={selectedIndex === index}
              class:hover:bg-gray-50={selectedIndex !== index}
              on:click={() => selectItem(item)}
              on:mouseenter={() => (selectedIndex = index)}
              on:keydown={(e) => e.key === 'Enter' && selectItem(item)}
              role="button"
              tabindex="0"
            >
              <div class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-md bg-gray-100">
                <svelte:component this={getIcon(item.icon)} class="h-4 w-4 text-gray-600" />
              </div>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium">{item.name}</p>
                {#if item.subtitle}
                  <p class="truncate text-xs text-gray-400">{item.subtitle}</p>
                {/if}
              </div>
              {#if selectedIndex === index}
                <kbd class="flex-shrink-0 rounded bg-gray-100 px-1.5 py-0.5 font-mono text-xs text-gray-400">↵</kbd>
              {/if}
            </div>
          {/each}
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center gap-4 border-t border-gray-100 px-4 py-2 text-xs text-gray-400">
        <span class="flex items-center gap-1">
          <kbd class="rounded bg-gray-100 px-1.5 py-0.5 font-mono">↑↓</kbd>
          navigate
        </span>
        <span class="flex items-center gap-1">
          <kbd class="rounded bg-gray-100 px-1.5 py-0.5 font-mono">↵</kbd>
          select
        </span>
      </div>
    </div>
  </div>
{/if}
