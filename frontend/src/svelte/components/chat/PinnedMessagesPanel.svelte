<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { Pin, X, ExternalLink } from 'lucide-svelte'
  import { format } from 'date-fns'
  import { svelteApi } from '../../stores/http'

  interface PanelMessage {
    id: string
    channelId: string
    username: string
    timestamp: string
    content: string
    files?: { length: number }[] | null
  }

  let { channelId, open }: { channelId: string; open: boolean } = $props()

  const dispatch = createEventDispatcher<{ close: void; jump: string }>()

  let pinnedMessages = $state<PanelMessage[]>([])
  let loading = $state(false)

  async function loadPinnedMessages() {
    loading = true
    try {
      const { data } = await svelteApi.get<{ messages: unknown[] }>(`/channels/${channelId}/posts?is_pinned=true`)
      pinnedMessages = data.messages.map((m) => {
        const msg = m as Record<string, unknown>
        return {
          id: String(msg.id ?? ''),
          channelId: String(msg.channel_id ?? msg.channelId ?? ''),
          username: String(msg.username ?? ''),
          timestamp: String(msg.created_at ?? msg.timestamp ?? new Date().toISOString()),
          content: String(msg.message ?? msg.content ?? ''),
          files: Array.isArray(msg.files) ? (msg.files as { length: number }[]) : null,
        }
      })
    } catch (e) {
      console.error('Failed to fetch pinned messages', e)
    } finally {
      loading = false
    }
  }

  $effect(() => {
    if (open && channelId) {
      void loadPinnedMessages()
    }
  })

  async function handleUnpin(message: PanelMessage) {
    try {
      await svelteApi.delete(`/posts/${message.id}/pin`)
      pinnedMessages = pinnedMessages.filter((m) => m.id !== message.id)
    } catch (e) {
      console.error('Failed to unpin message', e)
    }
  }

  function jumpToMessage(message: PanelMessage) {
    dispatch('jump', message.id)
  }
</script>

{#if open}
  <aside class="h-full bg-bg-surface-1 flex flex-col" data-testid="pinned-messages-panel" transition:fly={{ x: 300, duration: 250, easing: cubicOut }}>
    <!-- Header -->
    <div class="h-12 border-b border-border-1 flex items-center justify-between px-4">
      <div class="flex items-center space-x-2">
        <Pin class="w-5 h-5 text-text-3 fill-current" />
        <span class="font-semibold text-text-1">Pinned Items</span>
      </div>
      <button
        class="p-1 hover:bg-gray-100 rounded transition-colors"
        aria-label="Close pinned messages"
        onclick={() => dispatch('close')}
      >
        <X class="w-5 h-5 text-text-4" />
      </button>
    </div>

    <!-- Pinned List -->
    <div class="flex-1 overflow-y-auto p-0">
      {#if loading}
        <div class="text-center py-8 text-text-3">
          <div class="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full mx-auto mb-2"></div>
          Loading pinned items...
        </div>
      {:else if pinnedMessages.length === 0}
        <div class="text-center py-8 text-text-3 px-4">
          <div class="mb-2 text-text-4">
            <Pin class="w-12 h-12 mx-auto mb-3 opacity-20" />
            No pinned items yet
          </div>
          <div class="text-xs">Pin important messages to find them easily here</div>
        </div>
      {:else}
        <div class="divide-y divide-border-1">
          {#each pinnedMessages as message (message.id)}
            <div class="px-4 py-4 hover:bg-bg-surface-2 transition-colors group relative">
              <div class="flex items-start justify-between mb-1">
                <div class="flex items-center space-x-2">
                  <span class="font-bold text-sm text-text-1">{message.username}</span>
                  <span class="text-[10px] text-text-4">{format(new Date(message.timestamp), 'MMM d, h:mm a')}</span>
                </div>
                <!-- Actions -->
                <div class="flex items-center space-x-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button
                    class="p-1 hover:bg-gray-200 rounded text-text-4 hover:text-red-500 transition-colors"
                    title="Unpin"
                    onclick={() => handleUnpin(message)}
                  >
                    <Pin class="w-3.5 h-3.5 rotate-45" />
                  </button>
                  <button
                    class="p-1 hover:bg-gray-200 rounded text-text-4 hover:text-blue-500 transition-colors"
                    title="Jump to message"
                    onclick={() => jumpToMessage(message)}
                  >
                    <ExternalLink class="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
              <div class="text-sm text-gray-700 line-clamp-4 mt-1">
                {message.content}
              </div>
              {#if message.files && message.files.length > 0}
                <div class="mt-2 text-[10px] text-blue-500 flex items-center">
                  <span class="mr-1">{message.files.length} attachment{message.files.length > 1 ? 's' : ''}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </aside>
{/if}
