<script lang="ts">
  import { createEventDispatcher, untrack } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X, Camera } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi, SvelteHttpError } from '../../stores/http'
  import { authStore } from '../../stores/auth'
  import type { AuthUser } from '../../../core/entities/Auth'

  interface Props {
    open?: boolean
  }

  let { open = false }: Props = $props()

  const dispatch = createEventDispatcher<{ close: void }>()

  let username = $state('')
  let firstName = $state('')
  let lastName = $state('')
  let displayName = $state('')
  let nickname = $state('')
  let position = $state('')
  let avatarUrl = $state('')
  let loading = $state(false)
  let error = $state('')
  let success = $state('')

  const currentUser = $derived($authStore.user)

  // Full name preview
  const fullNamePreview = $derived((() => {
    const first = firstName.trim()
    const last = lastName.trim()
    if (first || last) return `${first} ${last}`.trim()
    return displayName.trim() || nickname.trim() || username
  })())

  // Populate form when modal opens
  $effect(() => {
    if (open) {
      const user = untrack(() => $authStore.user)
      if (user) {
        username = user.username || ''
        firstName = user.firstName || user.first_name || ''
        lastName = user.lastName || user.last_name || ''
        displayName = user.displayName || user.display_name || ''
        nickname = user.nickname || ''
        position = user.position || ''
        avatarUrl = user.avatarUrl || user.avatar_url || ''
        error = ''
        success = ''
      }
    }
  })

  function getInitials(name: string) {
    return name
      .split(' ')
      .map((n) => n[0])
      .filter(Boolean)
      .join('')
      .toUpperCase()
      .slice(0, 2)
  }

  async function handleSubmit(event: Event) {
    event.preventDefault()

    if (!currentUser) return

    loading = true
    error = ''
    success = ''

    try {
      await svelteApi.put<AuthUser>('/users/me', {
        username: username.trim() || undefined,
        first_name: firstName.trim() || undefined,
        last_name: lastName.trim() || undefined,
        display_name: displayName.trim() || undefined,
        nickname: nickname.trim() || undefined,
        position: position.trim() || undefined,
        avatar_url: avatarUrl.trim() || undefined,
      })

      // Update local user state
      await authStore.fetchMe()
      success = 'Profile updated successfully!'

      setTimeout(() => {
        dispatch('close')
      }, 1000)
    } catch (e: unknown) {
      if (e instanceof SvelteHttpError && e.data && typeof e.data === 'object' && 'message' in e.data) {
        error = String(e.data.message)
      } else if (e instanceof Error) {
        error = e.message
      } else {
        error = 'Failed to update profile'
      }
    } finally {
      loading = false
    }
  }

  function handleClose() {
    error = ''
    success = ''
    dispatch('close')
  }
</script>

<svelte:window on:keydown={(e) => open && e.key === 'Escape' && handleClose()} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4"
    data-testid="edit-profile-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      on:click={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close edit profile modal"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal -->
    <div
      class="relative bg-bg-surface-1 rounded-r-3 shadow-2xl ring-1 ring-border-1 w-full max-w-md overflow-hidden max-h-[90vh] overflow-y-auto"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border-1">
        <h2 class="text-lg font-semibold text-text-1">Edit Profile</h2>
        <button
          type="button"
          on:click={handleClose}
          class="flex h-10 w-10 items-center justify-center rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
          aria-label="Close"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      <!-- Form -->
      <form on:submit={handleSubmit} class="p-6 space-y-5">
        <!-- Avatar Preview -->
        <div class="flex justify-center">
          <div class="relative">
            <div class="flex h-24 w-24 items-center justify-center overflow-hidden rounded-full bg-brand text-3xl font-bold text-brand-foreground">
              {#if avatarUrl}
                <img src={avatarUrl} alt="Avatar" class="w-full h-full object-cover" />
              {:else}
                {getInitials(fullNamePreview || username || 'U')}
              {/if}
            </div>
            <button
              type="button"
              class="absolute bottom-0 right-0 w-8 h-8 bg-gray-800 rounded-full flex items-center justify-center border-2 border-white"
            >
              <Camera class="w-4 h-4 text-white" />
            </button>
          </div>
        </div>

        <!-- Full Name Preview -->
        <div class="text-center">
          <p class="text-lg font-semibold text-text-1">{fullNamePreview}</p>
          <p class="text-sm text-text-3">@{username}</p>
        </div>

        <!-- Error/Success Messages -->
        {#if error}
          <div class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
            {error}
          </div>
        {/if}
        {#if success}
          <div class="p-3 bg-green-50 border border-green-200 rounded-lg text-green-600 text-sm">
            {success}
          </div>
        {/if}

        <!-- Name Fields -->
        <div class="space-y-1">
          <label class="block text-xs font-semibold text-text-3 uppercase tracking-wider">Full Name</label>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">First Name</label>
              <input
                type="text"
                bind:value={firstName}
                placeholder="John"
                disabled={loading}
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Last Name</label>
              <input
                type="text"
                bind:value={lastName}
                placeholder="Doe"
                disabled={loading}
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
              />
            </div>
          </div>
        </div>

        <!-- Display Name -->
        <div>
          <label class="block text-sm font-medium text-text-2 mb-1">Display Name</label>
          <input
            type="text"
            bind:value={displayName}
            placeholder="How you want to be called"
            disabled={loading}
            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
          />
        </div>

        <!-- Nickname -->
        <div>
          <label class="block text-sm font-medium text-text-2 mb-1">Nickname</label>
          <input
            type="text"
            bind:value={nickname}
            placeholder="Your preferred name"
            disabled={loading}
            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
          />
        </div>

        <!-- Position -->
        <div>
          <label class="block text-sm font-medium text-text-2 mb-1">Position / Job Title</label>
          <input
            type="text"
            bind:value={position}
            placeholder="e.g. Software Engineer"
            disabled={loading}
            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
          />
        </div>

        <!-- Username -->
        <div>
          <label class="block text-sm font-medium text-text-2 mb-1">Username</label>
          <input
            type="text"
            bind:value={username}
            placeholder="your_username"
            disabled={loading}
            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
          />
        </div>

        <!-- Avatar URL -->
        <div>
          <label class="block text-sm font-medium text-text-2 mb-1">Avatar URL</label>
          <input
            type="text"
            bind:value={avatarUrl}
            placeholder="https://example.com/avatar.jpg"
            disabled={loading}
            class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm disabled:opacity-50"
          />
        </div>

        <!-- Email (read-only) -->
        <div class="space-y-1">
          <label class="block text-sm font-medium text-text-2">Email</label>
          <div class="px-3 py-2 bg-bg-surface-2 rounded-lg text-text-3 text-sm">
            {currentUser?.email ?? ''}
          </div>
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
            disabled={loading}
            class="px-4 py-2 text-sm font-medium text-brand-foreground bg-brand rounded-lg hover:bg-brand-hover transition-colors disabled:opacity-50"
          >
            {loading ? 'Saving...' : 'Save Changes'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
