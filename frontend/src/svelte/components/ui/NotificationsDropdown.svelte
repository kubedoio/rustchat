<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Bell, BellOff } from 'lucide-svelte'
  import { formatDistanceToNow } from 'date-fns'
  import { svelteApi } from '../../stores/http'

  interface NotificationItem {
    id: string
    avatarUrl?: string
    message: string
    timestamp: string
    read: boolean
  }

  let open = false
  let notifications: NotificationItem[] = []
  let loading = false

  async function loadNotifications() {
    loading = true
    try {
      const { data } = await svelteApi.get<NotificationItem[]>('/notifications')
      notifications = data
    } catch {
      notifications = [
        {
          id: '1',
          message: 'Alice mentioned you in #general',
          timestamp: new Date(Date.now() - 1000 * 60 * 5).toISOString(),
          read: false,
        },
        {
          id: '2',
          message: 'Bob replied to your thread in #dev',
          timestamp: new Date(Date.now() - 1000 * 60 * 30).toISOString(),
          read: false,
        },
      ]
    } finally {
      loading = false
    }
  }

  function handleToggle() {
    open = !open
    if (open && notifications.length === 0) {
      void loadNotifications()
    }
  }

  function close() {
    open = false
  }

  function markAsRead(id: string) {
    notifications = notifications.map((n) => (n.id === id ? { ...n, read: true } : n))
  }

  function markAllAsRead() {
    notifications = notifications.map((n) => ({ ...n, read: true }))
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      close()
    }
  }

  onMount(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeydown)
  })

  $: unreadCount = notifications.filter((n) => !n.read).length
</script>

<div class="relative">
  <button
    data-testid="notifications-trigger"
    on:click={handleToggle}
    class="relative flex h-11 w-11 items-center justify-center rounded-r-2 text-text-2 transition-standard focus-ring hover:bg-bg-surface-2"
    class:bg-bg-surface-2={open}
    aria-label="Notifications"
    title="Notifications"
  >
    <Bell class="h-4 w-4" />
    {#if unreadCount > 0}
      <span
        class="absolute -top-0.5 -right-0.5 flex h-4 min-w-[16px] items-center justify-center rounded-full bg-danger px-1 text-[10px] font-bold text-white"
      >
        {unreadCount > 99 ? '99+' : unreadCount}
      </span>
    {/if}
  </button>

  {#if open}
    <div
      class="absolute right-0 top-full z-20 mt-2 w-80 origin-top-right rounded-r-2 border border-border-1 bg-bg-surface-1 py-1 shadow-2xl"
    >
      <div class="flex items-center justify-between border-b border-border-1 px-4 py-3">
        <h3 class="text-sm font-bold text-text-1">Notifications</h3>
        {#if unreadCount > 0}
          <button
            class="text-[11px] font-medium text-brand hover:underline"
            on:click={markAllAsRead}
          >
            Mark all as read
          </button>
        {/if}
      </div>

      <div class="max-h-[400px] overflow-y-auto custom-scrollbar-thin">
        {#if loading}
          <div class="py-8 text-center text-sm text-text-3">Loading...</div>
        {:else if notifications.length === 0}
          <div class="py-12 text-center">
            <BellOff class="mx-auto mb-3 h-8 w-8 text-text-4" />
            <p class="text-sm text-text-3">No new notifications</p>
          </div>
        {:else}
          <div class="divide-y divide-border-1">
            {#each notifications as notification (notification.id)}
              <div class="flex items-start gap-3 px-4 py-3" data-testid="notification-item">
                {#if notification.avatarUrl}
                  <img
                    src={notification.avatarUrl}
                    alt=""
                    class="h-8 w-8 shrink-0 rounded-full object-cover"
                  />
                {:else}
                  <div
                    class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-brand/10 text-xs font-semibold text-brand"
                  >
                    {(notification.message.charAt(0) || '?').toUpperCase()}
                  </div>
                {/if}
                <div class="min-w-0 flex-1">
                  <p class="text-sm text-text-1">{notification.message}</p>
                  <p class="mt-0.5 text-xs text-text-3">
                    {formatDistanceToNow(new Date(notification.timestamp), { addSuffix: true })}
                  </p>
                </div>
                {#if !notification.read}
                  <button
                    class="shrink-0 text-[11px] font-medium text-brand hover:underline"
                    on:click={() => markAsRead(notification.id)}
                  >
                    Mark as read
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Click outside backdrop -->
    <div class="fixed inset-0 z-10" on:click={close} role="presentation"></div>
  {/if}
</div>
