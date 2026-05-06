<script lang="ts">
  import { onMount, afterUpdate } from 'svelte'
  import { chatStore } from '../../stores/chat'
  import type { ChatMessage } from './types'
  import MessageItem from './MessageItem.svelte'

  export let messages: ChatMessage[] = []
  export let channelId: string | null = null
  export let currentUserId: string | undefined = undefined

  let container: HTMLElement
  let isNearBottom = true
  $: activeChannelId = channelId ?? $chatStore.currentChannelId
  $: renderedMessages = activeChannelId ? ($chatStore.messagesByChannel[activeChannelId] ?? []) : messages

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
    }
  }

  function handleScroll() {
    if (!container) return
    const threshold = 50
    isNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < threshold
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
    <div class="rounded-xl border border-dashed border-border-2 bg-bg-surface-1 p-6 text-center text-sm text-text-3">
      No messages yet. Start the conversation below.
    </div>
  {:else}
    <div class="space-y-1">
      {#each renderedMessages as message (message.id)}
        <MessageItem
          {message}
          {currentUserId}
          on:reply
          on:edit
          on:delete
          on:openProfile
          on:thread
        />
      {/each}
    </div>
  {/if}
</section>
