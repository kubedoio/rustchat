<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { Search, X, Hash, Clock, MessageSquare } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { format } from 'date-fns'
  import { searchStore, type SearchFilter } from '../../stores/search'
  import { chatStore } from '../../stores/chat'

  export let open = false

  const dispatch = createEventDispatcher<{ close: void }>()

  const filters: SearchFilter[] = ['messages', 'files', 'channels', 'users']
  const filterLabels: Record<SearchFilter, string> = {
    messages: 'Messages',
    files: 'Files',
    channels: 'Channels',
    users: 'Users',
  }

  let query = ''

  $: if (!open) {
    query = ''
    searchStore.clearSearch()
  }

  function handleClose() {
    query = ''
    searchStore.clearSearch()
    dispatch('close')
  }

  function handleFilterClick(filter: SearchFilter) {
    searchStore.performSearch(query, filter)
  }

  function getChannelName(channelId: string): string {
    const channel = $chatStore.channels.find((c) => c.id === channelId)
    return channel?.display_name || channel?.name || 'Unknown'
  }

  function handleResultClick() {
    dispatch('close')
  }

  function handleRecentClick(search: string) {
    query = search
    searchStore.performSearch(search, $searchStore.filter)
  }

  function onInput() {
    searchStore.performSearch(query, $searchStore.filter)
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (open && e.key === 'Escape') {
      e.preventDefault()
      handleClose()
    }
  }}
/>

{#if open}
  <div class="fixed inset-0 z-50 flex items-start justify-center pt-[10vh]" data-testid="search-modal" role="dialog" aria-modal="true">
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/50"
      onclick={handleClose}
      onkeydown={(e) => e.key === 'Escape' && handleClose()}
      role="button"
      tabindex="-1"
      aria-label="Close search"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal -->
    <div
      class="relative mx-4 w-full max-w-2xl overflow-hidden rounded-xl bg-white shadow-2xl"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Search Input -->
      <div class="flex items-center border-b border-gray-200 px-4 py-3">
        <Search class="mr-3 h-5 w-5 text-gray-400" />
        <input
          bind:value={query}
          oninput={onInput}
          type="text"
          placeholder="Search messages, files, and more..."
          class="flex-1 bg-transparent text-lg text-gray-900 placeholder-gray-400 outline-none"
          data-testid="search-input"
          autofocus
        />
        <div class="flex items-center space-x-2">
          <kbd class="hidden rounded bg-gray-100 px-2 py-1 text-xs text-gray-500 sm:block">ESC</kbd>
          <button onclick={handleClose} class="rounded p-1 hover:bg-gray-100" aria-label="Close">
            <X class="h-5 w-5 text-gray-400" />
          </button>
        </div>
      </div>

      <!-- Filter Tabs -->
      <div class="flex border-b border-gray-200 px-4">
        {#each filters as filter}
          <button
            class="px-3 py-2 text-sm font-medium capitalize transition-colors"
            class:text-brand={$searchStore.filter === filter}
            class:border-b-2={$searchStore.filter === filter}
            class:border-brand={$searchStore.filter === filter}
            class:text-gray-500={$searchStore.filter !== filter}
            class:hover:text-gray-700={$searchStore.filter !== filter}
            onclick={() => handleFilterClick(filter)}
          >
            {filterLabels[filter]}
          </button>
        {/each}
      </div>

      <!-- Results Area -->
      <div class="max-h-[60vh] overflow-y-auto">
        {#if $searchStore.loading}
          <div class="p-8 text-center text-gray-500">
            <div class="mx-auto h-8 w-8 animate-spin rounded-full border-2 border-brand border-t-transparent"></div>
            <p class="mt-2">Searching...</p>
          </div>
        {:else if $searchStore.error}
          <div class="p-8 text-center text-red-500">
            {$searchStore.error}
          </div>
        {:else if $searchStore.results.length > 0}
          <div class="py-2">
            <div class="px-4 py-2 text-xs font-semibold uppercase text-gray-500">
              {$searchStore.results.length} Results
            </div>
            {#each $searchStore.results as result (result.id)}
              <div
                onclick={handleResultClick}
                onkeydown={(e) => e.key === 'Enter' && handleResultClick()}
                class="cursor-pointer border-b border-gray-100 px-4 py-3 last:border-0 hover:bg-gray-50"
                role="button"
                tabindex="0"
              >
                <div class="mb-1 flex items-center space-x-2 text-xs text-gray-500">
                  <Hash class="h-3 w-3" />
                  <span>{getChannelName(result.channel_id)}</span>
                  <span>•</span>
                  <Clock class="h-3 w-3" />
                  <span>{format(new Date(result.created_at), 'MMM d, yyyy')}</span>
                </div>
                <div class="line-clamp-2 text-sm text-gray-900">
                  {result.message}
                </div>
                {#if result.username}
                  <div class="mt-1 text-xs text-gray-400">
                    @{result.username}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {:else if query.trim().length >= 2 && !$searchStore.loading}
          <div class="p-8 text-center text-gray-500">
            <MessageSquare class="mx-auto mb-3 h-12 w-12 opacity-50" />
            <p>No results found for "{query}"</p>
          </div>
        {:else}
          <div class="p-4">
            {#if $searchStore.recentSearches.length > 0}
              <div class="mb-2 text-xs font-semibold uppercase text-gray-500">Recent Searches</div>
              <div class="space-y-1">
                {#each $searchStore.recentSearches as search}
                  <button
                    onclick={() => handleRecentClick(search)}
                    class="flex w-full items-center rounded-lg px-3 py-2 text-sm text-gray-700 hover:bg-gray-100"
                  >
                    <Clock class="mr-2 h-4 w-4 text-gray-400" />
                    {search}
                  </button>
                {/each}
              </div>
            {:else}
              <div class="py-6 text-center text-gray-500">
                <Search class="mx-auto mb-3 h-12 w-12 opacity-50" />
                <p>Start typing to search messages</p>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between border-t border-gray-200 bg-gray-50 px-4 py-2 text-xs text-gray-500">
        <span>Search powered by PostgreSQL full-text search</span>
        <span>↵ to select</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .line-clamp-2 {
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
