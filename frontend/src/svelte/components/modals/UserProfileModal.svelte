<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X, Mail, MessageCircle, Briefcase, Check, Circle, Clock3, Minus } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi } from '../../stores/http'

  export let userId: string
  export let open = false

  const dispatch = createEventDispatcher<{
    close: void
    message: string
  }>()

  interface UserData {
    id: string
    username: string
    email?: string
    display_name?: string
    nickname?: string
    first_name?: string
    last_name?: string
    position?: string
    avatar_url?: string
    presence?: 'online' | 'away' | 'dnd' | 'offline'
    status_text?: string | null
    status_emoji?: string | null
  }

  let user: UserData | null = null
  let loading = false
  let error = ''

  $: if (open && userId) {
    loadUser()
  }

  async function loadUser() {
    loading = true
    error = ''
    try {
      const { data } = await svelteApi.get<UserData>(`/users/${userId}`)
      user = data
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : 'Failed to load user profile'
    } finally {
      loading = false
    }
  }

  $: fullName = (() => {
    if (!user) return ''
    const first = user.first_name || ''
    const last = user.last_name || ''
    if (first || last) return `${first} ${last}`.trim()
    return user.display_name || user.nickname || user.username
  })()

  function presenceLabel(presence?: string | null): string {
    switch (presence) {
      case 'online':
        return 'Online'
      case 'away':
        return 'Away'
      case 'dnd':
        return 'Do not disturb'
      default:
        return 'Offline'
    }
  }

  function presenceBadgeClass(presence?: string | null): string {
    switch (presence) {
      case 'online':
        return 'bg-success text-white border-bg-surface-1'
      case 'away':
        return 'bg-warning/15 text-warning border-bg-surface-1'
      case 'dnd':
        return 'bg-danger text-white border-bg-surface-1'
      default:
        return 'bg-bg-surface-1 text-text-4 border-border-2'
    }
  }

  function handleClose() {
    dispatch('close')
  }

  function handleMessage() {
    if (user) {
      dispatch('message', user.id)
    }
  }

  function getInitials(name: string) {
    return name
      .split(' ')
      .map((n) => n[0])
      .filter(Boolean)
      .join('')
      .toUpperCase()
      .slice(0, 2)
  }
</script>

<svelte:window on:keydown={(e) => open && e.key === 'Escape' && handleClose()} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center" role="dialog" aria-modal="true">
    <!-- Backdrop -->
    <div class="absolute inset-0 bg-bg-app/70 backdrop-blur-sm" on:click={handleClose} on:keydown={(e) => e.key === 'Escape' && handleClose()} role="button" tabindex="-1" aria-label="Close modal" transition:fade={{ duration: 150, easing: cubicOut }}></div>

    <!-- Modal -->
    <div
      class="relative mx-4 w-full max-w-sm overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-end border-b border-border-1 px-4 py-3">
        <button on:click={handleClose} class="rounded-r-2 p-1 transition-standard hover:bg-bg-surface-2" aria-label="Close">
          <X class="h-5 w-5 text-text-3" />
        </button>
      </div>

      <!-- Loading State -->
      {#if loading}
        <div class="p-8 flex items-center justify-center">
          <div class="h-8 w-8 animate-spin rounded-full border-2 border-brand border-t-transparent"></div>
        </div>
      {:else if error}
        <!-- Error State -->
        <div class="p-8 text-center text-danger">
          {error}
        </div>
      {:else if user}
        <!-- Profile Content -->
        <div class="pb-6">
          <!-- Avatar & Name Section -->
          <div class="flex flex-col items-center px-6">
            {#if user.avatar_url}
              <img
                src={user.avatar_url}
                alt={user.username}
                class="h-24 w-24 rounded-full object-cover shadow-lg ring-4 ring-bg-surface-1"
              />
            {:else}
              <div class="h-24 w-24 rounded-full bg-brand/10 text-brand flex items-center justify-center text-2xl font-bold shadow-lg ring-4 ring-bg-surface-1">
                {getInitials(fullName || user.username)}
              </div>
            {/if}

            <h2 class="mt-4 text-center text-xl font-semibold text-brand">
              {fullName}
            </h2>

            {#if user.nickname && user.nickname !== fullName}
              <p class="text-sm text-text-3">{user.nickname}</p>
            {/if}

            <!-- Presence Badge -->
            <div data-testid="profile-modal-status" class="mt-3 flex flex-wrap items-center justify-center gap-2">
              <span
                data-testid="profile-modal-presence"
                class="inline-flex items-center gap-1 rounded-full border px-3 py-1 text-sm font-medium {presenceBadgeClass(user.presence)}"
              >
                {#if user.presence === 'online'}
                  <Check class="h-3.5 w-3.5" />
                {:else if user.presence === 'away'}
                  <Clock3 class="h-3.5 w-3.5" />
                {:else if user.presence === 'dnd'}
                  <Minus class="h-3.5 w-3.5" />
                {:else}
                  <Circle class="h-3.5 w-3.5" />
                {/if}
                {presenceLabel(user.presence)}
              </span>
              {#if user.status_text}
                <span
                  data-testid="profile-modal-custom-status"
                  class="inline-flex max-w-full items-center gap-1 rounded-full border border-border-1 bg-bg-surface-2 px-3 py-1 text-sm text-text-2"
                >
                  {#if user.status_emoji}
                    <span>{user.status_emoji}</span>
                  {/if}
                  <span class="truncate">{user.status_text}</span>
                </span>
              {/if}
            </div>
          </div>

          <!-- Details Section -->
          <div class="mt-6 space-y-4 px-6">
            <!-- Username -->
            <div class="flex items-center space-x-3 text-sm text-text-2">
              <span class="text-text-4">@</span>
              <span>{user.username}</span>
            </div>

            <!-- Email -->
            <div class="flex items-center space-x-3 text-sm text-text-2">
              <Mail class="h-4 w-4 text-text-4" />
              <span>{user.email}</span>
            </div>

            <!-- Nickname -->
            {#if user.nickname}
              <div class="flex items-center space-x-3 text-sm text-text-2">
                <span class="w-16 text-xs uppercase tracking-wider text-text-4">Nickname</span>
                <span>{user.nickname}</span>
              </div>
            {/if}

            <!-- First & Last Name -->
            {#if user.first_name || user.last_name}
              <div class="flex items-center space-x-3 text-sm text-text-2">
                <span class="w-16 text-xs uppercase tracking-wider text-text-4">Full Name</span>
                <span>{user.first_name} {user.last_name}</span>
              </div>
            {/if}

            <!-- Position -->
            {#if user.position}
              <div class="flex items-center space-x-3 text-sm text-text-2">
                <Briefcase class="h-4 w-4 text-text-4" />
                <span>{user.position}</span>
              </div>
            {/if}
          </div>

          <!-- Message Button -->
          <div class="mt-6 px-6">
            <button
              class="w-full inline-flex items-center justify-center rounded-r-2 bg-brand px-4 py-2.5 text-sm font-medium text-brand-foreground transition-standard hover:bg-brand-hover"
              on:click={handleMessage}
            >
              <MessageCircle class="w-4 h-4 mr-2" />
              Message
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}
