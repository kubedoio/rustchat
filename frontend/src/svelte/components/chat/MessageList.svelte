<script lang="ts">
  import { onMount, afterUpdate } from 'svelte'
  import { isSameDay, format } from 'date-fns'
  import { chatStore } from '../../stores/chat'
  import type { ChatMessage } from './types'
  import MessageItem from './MessageItem.svelte'

  export let messages: ChatMessage[] = []
  export let channelId: string | null = null
  export let currentUserId: string | undefined = undefined

  let container: HTMLElement
  let isNearBottom = true
  let showNewMessagesButton = false
  let highlightedMessageId: string | null = null
  let highlightTimeout: ReturnType<typeof setTimeout>

  $: activeChannelId = channelId ?? $chatStore.currentChannelId
  $: renderedMessages = activeChannelId ? ($chatStore.messagesByChannel[activeChannelId] ?? []) : messages

  type TimelineItem =
    | { type: 'date'; date: Date }
    | { type: 'message'; message: ChatMessage }

  function getTimeline(msgs: ChatMessage[]): TimelineItem[] {
    const result: TimelineItem[] = []
    let lastDate: Date | null = null
    for (const message of msgs) {
      const date = new Date(message.createdAt ?? message.created_at ?? 0)
      if (!lastDate || !isSameDay(date, lastDate)) {
        result.push({ type: 'date', date })
        lastDate = date
      }
      result.push({ type: 'message', message })
    }
    return result
  }

  $: timeline = getTimeline(renderedMessages)

  function scrollToBottom(behavior: ScrollBehavior = 'auto') {
    if (container) {
      container.scrollTo({ top: container.scrollHeight, behavior })
    }
  }

  export function scrollToMessage(messageId: string) {
    if (!container) return
    const el = container.querySelector(`[data-message-id="${messageId}"]`)
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' })
      highlightedMessageId = messageId
      clearTimeout(highlightTimeout)
      highlightTimeout = setTimeout(() => {
        highlightedMessageId = null
      }, 2000)
    }
  }

  function handleScroll() {
    if (!container) return
    const threshold = 50
    isNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < threshold
    if (isNearBottom) showNewMessagesButton = false
  }

  $: if (renderedMessages.length > 0 && !isNearBottom) {
    showNewMessagesButton = true
  }

  onMount(() => {
    scrollToBottom()
  })

  afterUpdate(() => {
    if (isNearBottom) {
      scrollToBottom('smooth')
    }
  })
</script>

<section
  bind:this={container}
  on:scroll={handleScroll}
  class="flex-1 overflow-y-auto bg-bg-app p-4"
  aria-label="Message list"
>
  {#if renderedMessages.length === 0}
    <div class="rounded-r-2 border border-dashed border-border-1 bg-bg-surface-1 p-6 text-center text-sm text-text-2">
      No messages yet. Start the conversation below.
    </div>
  {:else}
    <div class="space-y-1">
      {#each timeline as item (item.type === 'date' ? `date-${item.date.toISOString()}` : item.message.id)}
        {#if item.type === 'date'}
          <div class="sticky top-2 z-[5] flex justify-center my-3">
            <span class="px-3 py-1 text-xs font-medium text-text-3 bg-bg-surface-1 border border-border-1 rounded-full shadow-1">
              {format(item.date, 'MMMM d, yyyy')}
            </span>
          </div>
        {:else}
          <MessageItem
            message={item.message}
            {currentUserId}
            isHighlighted={highlightedMessageId === item.message.id}
            on:reply
            on:edit
            on:delete
            on:openProfile
            on:thread
          />
        {/if}
      {/each}
    </div>
  {/if}
</section>

{#if showNewMessagesButton}
  <button
    class="fixed bottom-20 left-1/2 -translate-x-1/2 px-4 py-2 bg-brand text-brand-foreground text-sm font-medium rounded-full shadow-2 hover:bg-brand-hover transition-standard z-20"
    on:click={() => { scrollToBottom('smooth'); showNewMessagesButton = false }}
  >
    New messages ↓
  </button>
{/if}
