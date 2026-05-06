<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Bell, X, Inbox, Loader2 } from 'lucide-svelte'
  import { formatDistanceToNow } from 'date-fns'
  import { activityStore, activityList, unreadActivityCount } from '../../stores/activity'
  import type { Activity, ActivityType } from '../../stores/activity'

  const filters: { label: string; value: ActivityType | null }[] = [
    { label: 'All', value: null },
    { label: 'Mentions', value: 'mention' },
    { label: 'Threads', value: 'thread_reply' },
  ]

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && $activityStore.isOpen) {
      activityStore.closeFeed()
    }
  }

  onMount(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeydown)
  })

  function actionText(type: ActivityType): string {
    switch (type) {
      case 'mention':
        return 'mentioned you'
      case 'reply':
        return 'replied to your message'
      case 'reaction':
        return 'reacted to your message'
      case 'dm':
        return 'sent you a message'
      case 'thread_reply':
        return 'replied in a thread you follow'
      default:
        return 'interacted with you'
    }
  }

  function handleActivityClick(activity: Activity) {
    if (!activity.read) {
      void activityStore.markRead(activity.id)
    }
    activityStore.closeFeed()
  }
</script>

{#if $activityStore.isOpen}
  <div
    data-testid="activity-feed"
    class="flex w-80 shrink-0 flex-col border-l border-border-1 bg-bg-surface-1 animate-slide-in-right"
    role="dialog"
    aria-label="Activity feed"
    aria-modal="true"
  >
    <!-- Header -->
    <div class="flex h-[var(--header-height)] items-center justify-between border-b border-border-1 px-4">
      <div class="flex items-center gap-2">
        <Bell class="h-5 w-5 text-text-1" />
        <h2 class="text-sm font-semibold text-text-1">
          Activity
          {#if $unreadActivityCount > 0}
            <span class="ml-2 text-xs font-normal text-danger">({$unreadActivityCount} unread)</span>
          {/if}
        </h2>
      </div>
      <div class="flex items-center gap-2">
        {#if $unreadActivityCount > 0}
          <button
            class="text-xs text-brand hover:text-brand-hover"
            on:click={() => activityStore.markAllRead()}
          >
            Mark all read
          </button>
        {/if}
        <button
          class="flex h-8 w-8 items-center justify-center rounded-r-1 text-text-3 transition-standard hover:bg-bg-surface-2 hover:text-text-1 focus-ring"
          aria-label="Close activity feed"
          on:click={() => activityStore.closeFeed()}
        >
          <X class="h-4 w-4" />
        </button>
      </div>
    </div>

    <!-- Filters -->
    <div class="flex gap-1.5 overflow-x-auto border-b border-border-1 p-3">
      {#each filters as filter (filter.label)}
        <button
          class="whitespace-nowrap rounded-full px-3 py-1 text-xs font-medium transition-standard"
          class:bg-text-1={$activityStore.filter === filter.value}
          class:text-bg-surface-1={$activityStore.filter === filter.value}
          class:bg-bg-surface-2={$activityStore.filter !== filter.value}
          class:text-text-2={$activityStore.filter !== filter.value}
          class:hover:bg-bg-surface-3={$activityStore.filter !== filter.value}
          on:click={() => activityStore.setFilter(filter.value)}
        >
          {filter.label}
        </button>
      {/each}
    </div>

    <!-- List -->
    <div class="flex-1 overflow-y-auto custom-scrollbar-thin">
      {#if $activityStore.isLoading && $activityList.length === 0}
        <div class="flex items-center justify-center py-12 text-text-3">
          <Loader2 class="mr-2 h-6 w-6 animate-spin" />
          Loading...
        </div>
      {:else if $activityList.length === 0}
        <div class="px-4 py-12 text-center text-text-3">
          <Inbox class="mx-auto mb-3 h-12 w-12 opacity-40" />
          <p class="font-medium">No activity yet</p>
          <p class="mt-1 text-sm">Mentions, replies, and reactions will appear here</p>
        </div>
      {:else}
        <div class="divide-y divide-border-1">
          {#each $activityList as activity (activity.id)}
            <button
              class="flex w-full items-start gap-3 p-4 text-left transition-standard {!activity.read ? 'bg-brand/5' : ''} {activity.read ? 'hover:bg-bg-surface-2' : ''}"
              on:click={() => handleActivityClick(activity)}
            >
              <div class="w-1.5 self-stretch pt-1.5">
                {#if !activity.read}
                  <div class="h-1.5 w-1.5 rounded-full bg-brand"></div>
                {/if}
              </div>
              <div
                class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-bg-surface-2"
              >
                <span class="text-sm">
                  {#if activity.type === 'mention'}@
                  {:else if activity.type === 'reply'}💬
                  {:else if activity.type === 'reaction'}❤️
                  {:else if activity.type === 'dm'}✉️
                  {:else}🧵
                  {/if}
                </span>
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex items-start justify-between gap-2">
                  <p class="text-sm leading-snug text-text-1">
                    <span class="font-semibold">{activity.actorUsername}</span>
                    {actionText(activity.type)}
                  </p>
                  <span class="mt-0.5 shrink-0 whitespace-nowrap text-xs text-text-3">
                    {formatDistanceToNow(new Date(activity.createdAt), { addSuffix: true })}
                  </span>
                </div>
                {#if activity.message}
                  <p class="mt-1 line-clamp-2 text-sm text-text-2">{activity.message}</p>
                {/if}
                {#if activity.type === 'reaction' && activity.reaction}
                  <p class="mt-1 text-base">{activity.reaction}</p>
                {/if}
                <p class="mt-1 text-xs text-text-3">
                  #{activity.channelName} · {activity.teamName}
                </p>
              </div>
            </button>
          {/each}

          {#if $activityStore.hasMore}
            <div class="p-4 text-center">
              <button
                class="text-sm text-brand hover:text-brand-hover disabled:opacity-50"
                disabled={$activityStore.isLoading}
                on:click={() => activityStore.loadMore()}
              >
                {$activityStore.isLoading ? 'Loading...' : 'Load more'}
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
