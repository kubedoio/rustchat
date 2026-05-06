<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X, Hash, ArrowRight } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi, SvelteHttpError } from '../../stores/http'
  import { chatStore } from '../../stores/chat'
  import type { SvelteChatChannel } from '../../stores/chat'

  interface JoinableChannel extends SvelteChatChannel {
    purpose?: string
    header?: string
  }

  interface Props {
    open?: boolean
  }

  let { open = false }: Props = $props()

  const dispatch = createEventDispatcher<{ close: void }>()

  let joinableChannels = $state<JoinableChannel[]>([])
  let loading = $state(false)
  let joining = $state<string | null>(null)
  let error = $state('')

  const currentTeam = $derived($chatStore.teams[0] ?? null)

  async function fetchJoinableChannels() {
    if (!currentTeam) return
    loading = true
    error = ''
    try {
      const { data } = await svelteApi.get<unknown[]>(`/channels?team_id=${encodeURIComponent(currentTeam.id)}&available_to_join=true`)
      joinableChannels = data.map((item: unknown) => {
        const c = typeof item === 'object' && item !== null ? (item as Record<string, unknown>) : {}
        return {
          id: String(c.id ?? ''),
          name: String(c.name ?? ''),
          display_name: typeof c.display_name === 'string' ? c.display_name : String(c.name ?? ''),
          team_id: String(c.team_id ?? ''),
          channel_type:
            c.channel_type === 'private' || c.channel_type === 'direct' || c.channel_type === 'group'
              ? c.channel_type
              : 'public',
          purpose: typeof c.purpose === 'string' ? c.purpose : undefined,
          header: typeof c.header === 'string' ? c.header : undefined,
        } as JoinableChannel
      })
    } catch (e: unknown) {
      console.error('Failed to fetch joinable channels', e)
      error = 'Failed to load channels'
    } finally {
      loading = false
    }
  }

  $effect(() => {
    if (open && currentTeam) {
      fetchJoinableChannels()
    }
  })

  async function joinChannel(channelId: string) {
    joining = channelId
    try {
      await svelteApi.post(`/channels/${channelId}/members`, {})
      await chatStore.fetchChannels()
      dispatch('close')
    } catch (e: unknown) {
      console.error('Failed to join channel:', e)
      if (e instanceof SvelteHttpError && e.data && typeof e.data === 'object' && 'message' in e.data) {
        error = String(e.data.message)
      } else if (e instanceof Error) {
        error = e.message
      } else {
        error = 'Failed to join channel'
      }
    } finally {
      joining = null
    }
  }

  function handleClose() {
    dispatch('close')
  }

</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && handleClose()} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4"
    data-testid="browse-channels-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-bg-app/70 backdrop-blur-sm"
      onclick={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close browse channels modal"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal -->
    <div
      class="relative mx-4 max-h-[80vh] w-full max-w-lg overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl flex flex-col"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-border-1 px-6 py-4 shrink-0">
        <div class="flex items-center space-x-3">
          <div class="flex h-10 w-10 items-center justify-center rounded-r-2 bg-brand/10 text-brand">
            <Hash class="h-5 w-5" />
          </div>
          <div>
            <h2 class="text-lg font-semibold text-brand">Browse Channels</h2>
            <p class="text-xs text-text-3">Discover spaces in this workspace and join the right conversation.</p>
          </div>
        </div>
        <button
          type="button"
          onclick={handleClose}
          class="rounded-r-2 p-1 transition-standard hover:bg-bg-surface-2"
          aria-label="Close"
        >
          <X class="h-5 w-5 text-text-3" />
        </button>
      </div>

      <!-- Content -->
      <div class="max-h-[60vh] overflow-y-auto p-6">
        {#if error}
          <div class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm mb-4">
            {error}
          </div>
        {/if}

        {#if loading}
          <div class="py-8 text-center text-text-3">
            <div class="inline-block h-8 w-8 animate-spin rounded-full border-2 border-brand border-t-transparent"></div>
            <p class="mt-2">Loading channels...</p>
          </div>
        {:else if joinableChannels.length === 0}
          <div class="py-8 text-center text-text-3">
            <Hash class="mx-auto mb-4 h-12 w-12 text-text-4" />
            <p>No new channels to join</p>
          </div>
        {:else}
          <div class="space-y-4">
            {#each joinableChannels as channel (channel.id)}
              <div
                class="flex items-center justify-between rounded-r-2 border border-border-1 bg-bg-surface-2/70 p-4 transition-standard hover:bg-bg-surface-2"
              >
                <div class="flex-1 min-w-0">
                  <p class="flex items-center truncate font-medium text-text-1">
                    <Hash class="mr-1 h-4 w-4 text-text-3" />
                    {channel.display_name || channel.name}
                  </p>
                  {#if channel.purpose}
                    <p class="mt-0.5 ml-5 truncate text-sm text-text-3">
                      {channel.purpose}
                    </p>
                  {/if}
                </div>
                <button
                  onclick={() => joinChannel(channel.id)}
                  disabled={joining === channel.id}
                  class="ml-4 flex items-center rounded-r-2 bg-brand px-3 py-1.5 text-sm font-medium text-brand-foreground transition-standard hover:opacity-90 disabled:opacity-50"
                >
                  <span>{joining === channel.id ? 'Joining...' : 'Join'}</span>
                  <ArrowRight class="w-4 h-4 ml-1" />
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
