<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi, SvelteHttpError } from '../../stores/http'
  import { authStore } from '../../stores/auth'
  import { canCreateTeam } from '@/features/permissions/capabilities'

  interface Props {
    open?: boolean
  }

  let { open = false }: Props = $props()

  const dispatch = createEventDispatcher<{ close: void }>()

  let name = $state('')
  let displayName = $state('')
  let description = $state('')
  let loading = $state(false)
  let error = $state('')

  const canCreate = $derived(canCreateTeam($authStore.user?.role))

  function resetForm() {
    name = ''
    displayName = ''
    description = ''
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

    if (!canCreate) {
      error = 'You do not have permission to create teams'
      return
    }

    if (!name.trim()) {
      error = 'Team name is required'
      return
    }

    loading = true
    error = ''

    try {
      await svelteApi.post('/teams', {
        name: name.trim().toLowerCase().replace(/\s+/g, '-'),
        display_name: displayName.trim() || name.trim(),
        description: description.trim() || undefined,
      })

      resetForm()
      dispatch('close')
    } catch (e: unknown) {
      if (e instanceof SvelteHttpError && e.data && typeof e.data === 'object' && 'message' in e.data) {
        error = String(e.data.message)
      } else if (e instanceof Error) {
        error = e.message
      } else {
        error = 'Failed to create team'
      }
    } finally {
      loading = false
    }
  }
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && handleClose()} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4"
    data-testid="create-team-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      onclick={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close create team modal"
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
        <h2 class="text-lg font-semibold text-text-1">Create Team</h2>
        <button
          type="button"
          onclick={handleClose}
          class="flex h-10 w-10 items-center justify-center rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
          aria-label="Close"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      {#if !canCreate}
        <div class="p-6 space-y-4">
          <div class="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800">
            You do not have permission to create teams.
          </div>
          <div class="flex justify-end pt-2">
            <button
              type="button"
              onclick={handleClose}
              class="px-4 py-2 text-sm font-medium text-text-2 bg-bg-surface-2 rounded-lg hover:bg-bg-surface-1 border border-border-2 transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      {:else}
        <form onsubmit={handleSubmit} class="p-6 space-y-4">
          <!-- Error -->
          {#if error}
            <div class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
              {error}
            </div>
          {/if}

          <!-- Name -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-1">
              Team Name
            </label>
            <input
              type="text"
              bind:value={name}
              placeholder="e.g., engineering"
              required
              disabled={loading}
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
              placeholder="e.g., Engineering Team"
              disabled={loading}
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
            />
          </div>

          <!-- Description -->
          <div>
            <label class="block text-sm font-medium text-text-2 mb-1">
              Description
            </label>
            <textarea
              bind:value={description}
              rows="3"
              class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent resize-none text-sm disabled:opacity-50"
              placeholder="What's this team about?"
              disabled={loading}
            ></textarea>
          </div>

          <!-- Actions -->
          <div class="flex justify-end space-x-3 pt-4">
            <button
              type="button"
              onclick={handleClose}
              disabled={loading}
              class="px-4 py-2 text-sm font-medium text-text-2 bg-bg-surface-2 rounded-lg hover:bg-bg-surface-1 border border-border-2 transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading}
              class="px-4 py-2 text-sm font-medium text-brand-foreground bg-brand rounded-lg hover:bg-brand-hover transition-colors disabled:opacity-50"
            >
              {loading ? 'Creating...' : 'Create Team'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}
