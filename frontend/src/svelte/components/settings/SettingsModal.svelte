<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X, LogOut, Bell, Monitor, Layout, Settings, Phone, User } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { authStore } from '../../stores/auth'
  import { uiStore, type SettingsTab } from '../../stores/ui'
  import NotificationsTab from './tabs/NotificationsTab.svelte'
  import DisplayTab from './tabs/DisplayTab.svelte'
  import SidebarTab from './tabs/SidebarTab.svelte'
  import AdvancedTab from './tabs/AdvancedTab.svelte'
  import CallsTab from './tabs/CallsTab.svelte'
  import ProfileTab from './tabs/ProfileTab.svelte'

  export let open = false

  const dispatch = createEventDispatcher<{ close: void }>()

  let activeTab: SettingsTab = 'notifications'
  let error = ''
  let success = ''

  const tabs: Array<{ id: SettingsTab; label: string; icon: typeof Bell }> = [
    { id: 'notifications', label: 'Notifications', icon: Bell },
    { id: 'display', label: 'Display', icon: Monitor },
    { id: 'sidebar', label: 'Sidebar', icon: Layout },
    { id: 'advanced', label: 'Advanced', icon: Settings },
  ]

  const pluginTabs: Array<{ id: SettingsTab; label: string; icon: typeof Phone }> = [
    { id: 'calls', label: 'Calls', icon: Phone },
  ]

  const allTabs: Array<{ id: SettingsTab; label: string; icon: typeof Bell }> = [
    ...tabs,
    ...pluginTabs,
    { id: 'profile', label: 'Profile', icon: User },
  ]

  $: if (open) {
    error = ''
    success = ''
    activeTab = allTabs.some((t) => t.id === $uiStore.settingsTab) ? $uiStore.settingsTab : 'notifications'
  }

  function setTab(tab: SettingsTab) {
    activeTab = tab
    uiStore.openSettings(tab)
  }

  function handleClose() {
    dispatch('close')
  }

  function handleLogout() {
    handleClose()
    authStore.logout()
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose()
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4" role="dialog" aria-modal="true">
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      on:click={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close settings"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal Panel -->
    <div
      class="relative bg-bg-surface-1 rounded-r-3 shadow-2xl ring-1 ring-border-1 w-full max-w-5xl max-h-[calc(100svh-1rem)] sm:max-h-[90vh] flex flex-col overflow-hidden"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-4 sm:px-6 py-4 border-b border-border-1 shrink-0">
        <h2 class="text-xl sm:text-2xl font-semibold text-text-1">Settings</h2>
        <button
          type="button"
          on:click={handleClose}
          class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
          aria-label="Close settings"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      <div class="flex-1 min-h-0 flex flex-col sm:flex-row overflow-hidden">
        <!-- Sidebar -->
        <div
          class="w-full sm:w-64 bg-bg-surface-2 border-b sm:border-b-0 sm:border-r border-border-1 flex flex-col shrink-0 overflow-y-auto"
        >
          <!-- User Info Card -->
          <div class="border-b border-border-1 px-4 py-4">
            <p class="text-[10px] font-semibold uppercase tracking-[0.2em] text-text-3">Personal settings</p>
            <p class="mt-2 truncate text-sm font-semibold text-text-1">
              {$authStore.user?.display_name || $authStore.user?.username || 'Account'}
            </p>
            <p class="truncate text-xs text-text-3">@{$authStore.user?.username || 'user'}</p>
          </div>

          <!-- Main Tabs -->
          <nav class="grid grid-cols-2 gap-2 p-3 sm:flex sm:flex-col sm:gap-0.5 sm:p-2">
            {#each tabs as tab (tab.id)}
              <button
                type="button"
                on:click={() => setTab(tab.id)}
                class="flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-r-2 whitespace-nowrap transition-standard"
                class:bg-bg-surface-1={activeTab === tab.id}
                class:text-brand={activeTab === tab.id}
                class:shadow-sm={activeTab === tab.id}
                class:ring-1={activeTab === tab.id}
                class:ring-border-1={activeTab === tab.id}
                class:text-text-2={activeTab !== tab.id}
                class:hover:bg-bg-surface-1={activeTab !== tab.id}
                class:hover:text-text-1={activeTab !== tab.id}
              >
                <svelte:component this={tab.icon} class="w-4 h-4 shrink-0" />
                {tab.label}
              </button>
            {/each}
          </nav>

          <!-- Plugin Section -->
          <div class="px-3 pt-0 pb-2 sm:px-4 sm:py-2 text-[10px] font-semibold uppercase tracking-wider text-text-3">
            Plugin Preferences
          </div>
          <nav class="grid grid-cols-2 gap-2 px-3 pb-3 sm:flex sm:flex-col sm:gap-0.5 sm:px-2 sm:pb-2">
            {#each pluginTabs as tab (tab.id)}
              <button
                type="button"
                on:click={() => setTab(tab.id)}
                class="flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-r-2 whitespace-nowrap transition-standard"
                class:bg-bg-surface-1={activeTab === tab.id}
                class:text-brand={activeTab === tab.id}
                class:shadow-sm={activeTab === tab.id}
                class:ring-1={activeTab === tab.id}
                class:ring-border-1={activeTab === tab.id}
                class:text-text-2={activeTab !== tab.id}
                class:hover:bg-bg-surface-1={activeTab !== tab.id}
                class:hover:text-text-1={activeTab !== tab.id}
              >
                <svelte:component this={tab.icon} class="w-4 h-4 shrink-0" />
                {tab.label}
              </button>
            {/each}
          </nav>

          <!-- Profile & Logout -->
          <div class="grid grid-cols-2 gap-2 p-3 border-t border-border-1 sm:mt-auto sm:block sm:p-2">
            <button
              type="button"
              on:click={() => setTab('profile')}
              class="w-full flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-r-2 transition-standard sm:mb-1"
              class:bg-bg-surface-1={activeTab === 'profile'}
              class:text-brand={activeTab === 'profile'}
              class:shadow-sm={activeTab === 'profile'}
              class:ring-1={activeTab === 'profile'}
              class:ring-border-1={activeTab === 'profile'}
              class:text-text-2={activeTab !== 'profile'}
              class:hover:bg-bg-surface-1={activeTab !== 'profile'}
              class:hover:text-text-1={activeTab !== 'profile'}
            >
              <User class="w-4 h-4 shrink-0" />
              Profile
            </button>
            <button
              type="button"
              on:click={handleLogout}
              class="w-full flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium text-danger hover:bg-danger/5 rounded-r-2 transition-standard"
            >
              <LogOut class="w-4 h-4 shrink-0" />
              Log out
            </button>
          </div>
        </div>

        <!-- Content -->
        <div class="flex-1 min-w-0 overflow-y-auto p-4 sm:p-6 bg-bg-surface-1">
          <!-- Messages -->
          {#if error}
            <div class="mb-4 p-3 bg-danger/10 border border-danger/20 rounded-r-2 text-danger text-sm">
              {error}
            </div>
          {/if}
          {#if success}
            <div class="mb-4 p-3 bg-success/10 border border-success/20 rounded-r-2 text-success text-sm">
              {success}
            </div>
          {/if}

          <!-- Tab Content -->
          <div class="max-w-2xl">
            {#if activeTab === 'profile'}
              <h3 class="text-lg font-semibold text-text-1 mb-4">Profile</h3>
              <ProfileTab />
            {:else if activeTab === 'notifications'}
              <h3 class="text-lg font-semibold text-text-1 mb-1">Notifications</h3>
              <p class="text-sm text-text-3 mb-6">Manage how you receive notifications.</p>
              <NotificationsTab />
            {:else if activeTab === 'display'}
              <h3 class="text-lg font-semibold text-text-1 mb-1">Display</h3>
              <p class="text-sm text-text-3 mb-6">Customize your display preferences.</p>
              <DisplayTab />
            {:else if activeTab === 'sidebar'}
              <h3 class="text-lg font-semibold text-text-1 mb-1">Sidebar</h3>
              <p class="text-sm text-text-3 mb-6">Configure your sidebar preferences.</p>
              <SidebarTab />
            {:else if activeTab === 'advanced'}
              <h3 class="text-lg font-semibold text-text-1 mb-1">Advanced</h3>
              <p class="text-sm text-text-3 mb-6">Advanced settings and options.</p>
              <AdvancedTab />
            {:else if activeTab === 'calls'}
              <h3 class="text-lg font-semibold text-text-1 mb-1">Calls</h3>
              <p class="text-sm text-text-3 mb-6">Configure your call preferences.</p>
              <CallsTab />
            {/if}
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
