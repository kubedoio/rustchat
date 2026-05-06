<script lang="ts">
  import { Search, Bell, Activity, ChevronDown, Settings, LogOut, Shield } from 'lucide-svelte'
  import { authStore } from '../../stores/auth'
  import { uiStore } from '../../stores/ui'
  import { activityStore } from '../../stores/activity'
  import NotificationsDropdown from '../ui/NotificationsDropdown.svelte'
  import SearchModal from '../search/SearchModal.svelte'

  let showUserMenu = false
  let showNotifications = false
  let searchOpen = false

  function openSearch() {
    searchOpen = true
  }

  function logout() {
    authStore.logout()
  }
</script>

<header class="shrink-0 h-[var(--header-height)] bg-bg-surface-1/95 backdrop-blur-sm border-b border-border-1 flex items-center px-3 gap-2 z-30">
  <!-- Logo / Site name -->
  <div class="flex items-center gap-2 min-w-0">
    <div class="w-8 h-8 rounded-r-2 bg-brand text-brand-foreground flex items-center justify-center font-bold text-sm">
      R
    </div>
    <span class="font-semibold text-text-1 hidden sm:block">RustChat</span>
  </div>

  <!-- Search bar (desktop) -->
  <button
    class="hidden md:flex items-center gap-2 flex-1 max-w-md mx-4 px-3 py-1.5 bg-bg-app border border-border-1 rounded-r-2 text-sm text-text-3 hover:border-border-2 transition-standard"
    on:click={openSearch}
  >
    <Search class="w-4 h-4" />
    <span class="flex-1 text-left">Search</span>
    <kbd class="hidden lg:inline-flex px-1.5 py-0.5 bg-bg-surface-2 border border-border-1 rounded text-[10px] text-text-3">⌘K</kbd>
  </button>

  <div class="flex items-center gap-1 ml-auto">
    <!-- Notifications -->
    <div class="relative">
      <button
        class="w-9 h-9 flex items-center justify-center rounded-r-2 hover:bg-bg-surface-2 text-text-2 transition-standard relative"
        on:click={() => showNotifications = !showNotifications}
        aria-label="Notifications"
      >
        <Bell class="w-5 h-5" />
        {#if $uiStore.unreadNotificationCount > 0}
          <span class="absolute top-1 right-1 w-2 h-2 bg-danger rounded-full" />
        {/if}
      </button>
      {#if showNotifications}
        <NotificationsDropdown open={true} on:close={() => showNotifications = false} />
      {/if}
    </div>

    <!-- Activity feed -->
    <button
      class="w-9 h-9 flex items-center justify-center rounded-r-2 hover:bg-bg-surface-2 text-text-2 transition-standard relative"
      on:click={() => activityStore.openFeed()}
      aria-label="Activity feed"
    >
      <Activity class="w-5 h-5" />
      {#if $activityStore.unreadCount > 0}
        <span class="absolute -top-0.5 -right-0.5 min-w-[16px] h-4 px-1 bg-danger text-white text-[10px] font-bold rounded-full flex items-center justify-center">
          {$activityStore.unreadCount}
        </span>
      {/if}
    </button>

    <!-- User menu -->
    <div class="relative">
      <button
        class="flex items-center gap-2 pl-2 pr-1 py-1 rounded-r-2 hover:bg-bg-surface-2 transition-standard"
        on:click={() => showUserMenu = !showUserMenu}
      >
        <div class="w-7 h-7 rounded-full bg-brand/10 text-brand flex items-center justify-center text-xs font-bold">
          {($authStore.user?.username ?? '?').slice(0, 2).toUpperCase()}
        </div>
        <span class="hidden sm:block text-sm font-medium text-text-1 max-w-[100px] truncate">
          {$authStore.user?.username ?? 'User'}
        </span>
        <ChevronDown class="w-4 h-4 text-text-3" />
      </button>

      {#if showUserMenu}
        <div class="fixed inset-0 z-30" on:click={() => showUserMenu = false} />
        <div class="absolute right-0 top-full mt-1 w-56 bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 py-1 z-40">
          <div class="px-3 py-2 border-b border-border-1">
            <p class="text-sm font-semibold text-text-1">{$authStore.user?.display_name || $authStore.user?.username}</p>
            <p class="text-xs text-text-3">@{$authStore.user?.username}</p>
          </div>
          <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard" on:click={() => { uiStore.openSettings(); showUserMenu = false }}>
            <Settings class="w-4 h-4" /> Settings
          </button>
          {#if $authStore.user?.role === 'system_admin'}
            <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard" on:click={() => { window.location.href = '/admin'; showUserMenu = false }}>
              <Shield class="w-4 h-4" /> Admin Console
            </button>
          {/if}
          <div class="my-1 border-t border-border-1" />
          <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-danger hover:bg-danger/5 transition-standard" on:click={logout}>
            <LogOut class="w-4 h-4" /> Log out
          </button>
        </div>
      {/if}
    </div>
  </div>
</header>

{#if searchOpen}
  <SearchModal open={true} on:close={() => searchOpen = false} />
{/if}
