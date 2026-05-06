<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { LogOut, User, Smile } from 'lucide-svelte'
  import { authStore } from '../../stores/auth'

  const dispatch = createEventDispatcher<{ setStatus: void; editProfile: void }>()

  let open = false
  let menuEl: HTMLDivElement | null = null

  function getInitials(name: string) {
    return name
      .split(' ')
      .map((n) => n[0])
      .filter(Boolean)
      .join('')
      .toUpperCase()
      .slice(0, 2)
  }

  function toggleMenu() {
    open = !open
  }

  function closeMenu() {
    open = false
  }

  function handleLogout() {
    closeMenu()
    authStore.logout()
  }

  function handleClickOutside(event: MouseEvent) {
    if (menuEl && !menuEl.contains(event.target as Node)) {
      closeMenu()
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeMenu()
    }
  }
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKeydown} />

<div class="relative" bind:this={menuEl}>
  <button
    type="button"
    data-testid="user-menu-trigger"
    class="flex h-10 w-10 items-center justify-center rounded-full bg-brand text-sm font-bold text-brand-foreground transition-standard hover:opacity-90 focus-ring"
    onclick={toggleMenu}
    aria-haspopup="true"
    aria-expanded={open}
  >
    {#if $authStore.user?.avatar_url}
      <img
        src={$authStore.user.avatar_url}
        alt={$authStore.user.username}
        class="h-full w-full rounded-full object-cover"
      />
    {:else}
      {getInitials($authStore.user?.display_name || $authStore.user?.username || 'U')}
    {/if}
  </button>

  {#if open}
    <div
      class="absolute bottom-full left-0 mb-2 w-56 rounded-r-2 border border-border-1 bg-bg-surface-1 shadow-2xl ring-1 ring-border-1"
      role="menu"
    >
      <div class="px-3 py-2 border-b border-border-1">
        <p class="text-sm font-semibold text-text-1 truncate">
          {$authStore.user?.display_name || $authStore.user?.username || 'Account'}
        </p>
        <p class="text-xs text-text-3 truncate">
          @{$authStore.user?.username || 'user'}
        </p>
      </div>

      <div class="p-1.5">
        <button
          type="button"
          class="flex w-full items-center gap-2.5 rounded-r-2 px-2.5 py-2 text-sm font-medium text-text-2 transition-standard hover:bg-bg-surface-2 hover:text-text-1"
          role="menuitem"
          onclick={() => { closeMenu(); dispatch('setStatus') }}
        >
          <Smile class="h-4 w-4 shrink-0" />
          Set status
        </button>

        <button
          type="button"
          class="flex w-full items-center gap-2.5 rounded-r-2 px-2.5 py-2 text-sm font-medium text-text-2 transition-standard hover:bg-bg-surface-2 hover:text-text-1"
          role="menuitem"
          onclick={() => { closeMenu(); dispatch('editProfile') }}
        >
          <User class="h-4 w-4 shrink-0" />
          Edit Profile
        </button>

        <button
          type="button"
          class="flex w-full items-center gap-2.5 rounded-r-2 px-2.5 py-2 text-sm font-medium text-danger transition-standard hover:bg-danger/5"
          role="menuitem"
          onclick={handleLogout}
        >
          <LogOut class="h-4 w-4 shrink-0" />
          Log out
        </button>
      </div>
    </div>
  {/if}
</div>
