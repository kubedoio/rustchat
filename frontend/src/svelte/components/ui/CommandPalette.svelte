<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import {
    Search,
    Hash,
    User,
    Settings,
    LogOut,
    HelpCircle,
    Bell,
    FileText,
  } from 'lucide-svelte'

  type CommandIconType =
    | 'channel'
    | 'user'
    | 'settings'
    | 'logout'
    | 'search'
    | 'help'
    | 'notifications'
    | 'file'

  interface Command {
    id: string
    name: string
    group: string
    iconType?: CommandIconType
    keywords?: string[]
  }

  interface Props {
    open?: boolean
    commands?: Command[]
    previewLimit?: number
  }

  const fallbackCommands: Command[] = [
    { id: 'settings', name: 'Settings', group: 'Actions', iconType: 'settings' },
    { id: 'search', name: 'Search', group: 'Actions', iconType: 'search' },
    { id: 'notifications', name: 'Notifications', group: 'Actions', iconType: 'notifications' },
    { id: 'help', name: 'Help & Support', group: 'Actions', iconType: 'help' },
  ]

  let { open = false, commands = fallbackCommands, previewLimit = 6 }: Props = $props()

  const dispatch = createEventDispatcher<{
    close: void
    select: string
  }>()

  let searchQuery = $state('')
  let selectedIndex = $state(0)
  let inputRef = $state<HTMLInputElement | null>(null)

  const filteredCommands = $derived(
    searchQuery.trim()
      ? commands.filter((c) =>
          [c.name, c.group, ...(c.keywords ?? [])].some((value) =>
            value.toLowerCase().includes(searchQuery.toLowerCase())
          )
        )
      : commands.slice(0, previewLimit)
  )

  $effect(() => {
    if (open) {
      searchQuery = ''
      selectedIndex = 0
      requestAnimationFrame(() => inputRef?.focus())
    }
  })

  $effect(() => {
    if (selectedIndex >= filteredCommands.length) {
      selectedIndex = Math.max(0, filteredCommands.length - 1)
    }
  })

  function handleClose() {
    dispatch('close')
  }

  function handleSelect(commandId: string) {
    dispatch('select', commandId)
    dispatch('close')
  }

  function handleKeydown(e: KeyboardEvent) {
    const items = filteredCommands
    if (!items.length) {
      if (e.key === 'Escape') {
        e.preventDefault()
        handleClose()
      }
      return
    }

    switch (e.key) {
      case 'Escape':
        e.preventDefault()
        handleClose()
        break
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
          handleSelect(items[selectedIndex].id)
        }
        break
    }
  }

  const iconMap: Record<CommandIconType, typeof Hash> = {
    channel: Hash,
    user: User,
    settings: Settings,
    logout: LogOut,
    search: Search,
    help: HelpCircle,
    notifications: Bell,
    file: FileText,
  }

  function getIconComponent(iconType: Command['iconType'] = 'search'): typeof Hash {
    return iconMap[iconType] ?? Search
  }
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && handleClose()} />

{#if open}
  <div
    class="fixed inset-0 z-50 overflow-y-auto p-4 sm:p-6 md:p-20"
    role="dialog"
    aria-modal="true"
    data-testid="command-palette"
    tabindex="-1"
    onclick={handleClose}
    onkeydown={(e) => e.key === 'Escape' && handleClose()}
  >
    <!-- Backdrop -->
    <div class="fixed inset-0 bg-black/25 backdrop-blur-sm transition-opacity"></div>

    <!-- Modal -->
    <div
      class="relative mx-auto max-w-xl transform divide-y divide-border-1 overflow-hidden rounded-xl bg-bg-surface-1 shadow-2xl ring-1 ring-text-1/5 transition-all"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.key === 'Escape' && handleClose()}
      role="document"
      tabindex="-1"
    >
      <div class="relative">
        <Search class="pointer-events-none absolute top-3.5 left-4 h-5 w-5 text-text-4" />
        <input
          bind:this={inputRef}
          type="text"
          class="h-12 w-full border-0 bg-transparent pl-11 pr-4 text-text-1 placeholder-text-4 focus:ring-0 sm:text-sm"
          placeholder="Search commands..."
          bind:value={searchQuery}
          onkeydown={handleKeydown}
        />
      </div>

      {#if filteredCommands.length > 0}
        <div class="max-h-96 scroll-py-3 overflow-y-auto p-3">
          {#each filteredCommands as item, index (item.id)}
            <button
              type="button"
              onclick={() => handleSelect(item.id)}
              onmouseenter={() => (selectedIndex = index)}
              class="group flex w-full cursor-default select-none rounded-xl p-3 text-left"
              class:bg-bg-surface-2={selectedIndex === index}
              aria-label={`Select ${item.name}`}
            >
              <div
                class="flex h-10 w-10 flex-none items-center justify-center rounded-lg"
                class:bg-bg-surface-1={selectedIndex === index}
                class:bg-bg-surface-2={selectedIndex !== index}
              >
                <svelte:component this={getIconComponent(item.iconType)} class="h-6 w-6 text-text-3" />
              </div>
              <div class="ml-4 flex-auto">
                <p
                  class="text-sm font-medium"
                  class:text-text-1={selectedIndex === index}
                  class:text-text-2={selectedIndex !== index}
                >
                  {item.name}
                </p>
                <p class="text-sm text-text-3">{item.group}</p>
              </div>
            </button>
          {/each}
        </div>
      {:else}
        <div class="py-14 px-6 text-center text-sm sm:px-14">
          <p class="mt-4 font-semibold text-text-1">No results found</p>
          <p class="mt-2 text-text-3">
            No commands found for this search term. Please try again.
          </p>
        </div>
      {/if}
    </div>
  </div>
{/if}
