<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi, SvelteHttpError } from '../../stores/http'
  import { authStore } from '../../stores/auth'
  import { chatStore } from '../../stores/chat'
  import { canCreateChannel } from '@/features/permissions/capabilities'

  interface Props {
    open?: boolean
  }

  let { open = false }: Props = $props()

  const dispatch = createEventDispatcher<{ close: void }>()

  let name = $state('')
  let displayName = $state('')
  let channelType = $state<'public' | 'private'>('public')
  let purpose = $state('')
  let loading = $state(false)
  let error = $state('')

  const currentTeam = $derived($chatStore.teams[0] ?? null)
  const canCreateStandardChannel = $derived(canCreateChannel($authStore.user?.role))

  function resetForm() {
    name = ''
    displayName = ''
    channelType = 'public'
    purpose = ''
    error = ''
  }

  function handleClose() {
    resetForm()
    dispatch('close')
  }

  $effect(() => {
    if (!open) {
      resetForm()
    }
  })

  async function handleSubmit(event: Event) {
    event.preventDefault()

    if (!canCreateStandardChannel) {
      error = 'You do not have permission to create channels'
      return
    }

    if (!name.trim()) {
      error = 'Channel name is required'
      return
    }

    if (!currentTeam) {
      error = 'Please select a team first'
      return
    }

    loading = true
    error = ''

    try {
      await svelteApi.post('/channels', {
        team_id: currentTeam.id,
        name: name.trim().toLowerCase().replace(/\s+/g, '-'),
        display_name: displayName.trim() || name.trim(),
        channel_type: channelType,
        purpose: purpose.trim() || undefined,
      })

      resetForm()
      dispatch('close')
    } catch (e: unknown) {
      if (e instanceof SvelteHttpError && e.data && typeof e.data === 'object' && 'message' in e.data) {
        error = String(e.data.message)
      } else if (e instanceof Error) {
        error = e.message
      } else {
        error = 'Failed to create channel'
      }
    } finally {
      loading = false
    }
  }

</script>

<svelte:window on:keydown={(e) => open && e.key === 'Escape' && handleClose()} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4"
    data-testid="create-channel-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      on:click={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close create channel modal"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal -->
    <div
      class="relative bg-bg-surface-1 rounded-r-3 shadow-2xl ring-1 ring-border-1 w-full max-w-md overflow-hidden"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border-1">
        <h2 class="text-lg font-semibold text-text-1">Create Channel</h2>
        <button
          type="button"
          on:click={handleClose}
          class="flex h-10 w-10 items-center justify-center rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
          aria-label="Close"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      {#if !canCreateStandardChannel}
        <div class="p-6 space-y-4">
          <div class="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800">
            You do not have permission to create channels.
          </div>
          <div class="flex justify-end pt-2">
            <button
              type="button"
              on:click={handleClose}
              class="px-4 py-2 text-sm font-medium text-text-2 bg-bg-surface-2 rounded-lg hover:bg-bg-surface-1 border border-border-2 transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      {:else}
        <form on:submit={handleSubmit} class="p-6 space-y-4">
          <!-- No Team Warning -->
          {#if !currentTeam}
            <div class="p-3 bg-yellow-50 border border-yellow-200 rounded-lg text-yellow-600 text-sm">
              Please create or select a team first.
            </div>
          {/if}

          <!-- Error -->
          {#if error}
            <div class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
              {error}
            </div>
          {/if}

          <!-- Channel Type -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-2">
              Channel Type
            </label>
            <div class="flex space-x-4">
              <label class="flex items-center cursor-pointer">
                <input
                  type="radio"
                  bind:group={channelType}
                  value="public"
                  class="w-4 h-4 text-brand border-border-2 focus:ring-brand"
                />
                <span class="ml-2 text-sm text-text-1">
                  <span class="font-medium">Public</span> - Anyone can join
                </span>
              </label>
              <label class="flex items-center cursor-pointer">
                <input
                  type="radio"
                  bind:group={channelType}
                  value="private"
                  class="w-4 h-4 text-brand border-border-2 focus:ring-brand"
                />
                <span class="ml-2 text-sm text-text-1">
                  <span class="font-medium">Private</span> - Invite only
                </span>
              </label>
            </div>
          </div>

          <!-- Name -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-1">
              Channel Name
            </label>
            <input
              type="text"
              bind:value={name}
              placeholder="e.g., general"
              required
              disabled={loading || !currentTeam}
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
            />
          </div>

          <!-- Display Name -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-1">
              Display Name
            </label>
            <input
              type="text"
              bind:value={displayName}
              placeholder="e.g., General Discussion"
              disabled={loading || !currentTeam}
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
            />
          </div>

          <!-- Purpose -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-1">
              Purpose
            </label>
            <textarea
              bind:value={purpose}
              rows="2"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent resize-none text-sm disabled:opacity-50"
              placeholder="What's this channel about?"
              disabled={loading || !currentTeam}
            ></textarea>
          </div>

          <!-- Actions -->
          <div class="flex justify-end space-x-3 pt-4">
            <button
              type="button"
              on:click={handleClose}
              disabled={loading}
              class="px-4 py-2 text-sm font-medium text-text-2 bg-bg-surface-2 rounded-lg hover:bg-bg-surface-1 border border-border-2 transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || !currentTeam}
              class="px-4 py-2 text-sm font-medium text-brand-foreground bg-brand rounded-lg hover:bg-brand-hover transition-colors disabled:opacity-50"
            >
              {loading ? 'Creating...' : 'Create Channel'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}
