<script lang="ts">
  import { onMount, afterUpdate } from 'svelte'
  import type { ChatMessage } from './types'
  import MessageItem from './MessageItem.svelte'

  export let messages: ChatMessage[] = []
  export let currentUserId: string | undefined = undefined

  let container: HTMLElement
  let isNearBottom = true

  function scrollToBottom(behavior: ScrollBehavior = 'auto') {
    if (container) {
      container.scrollTo({ top: container.scrollHeight, behavior })
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
  class="flex-1 overflow-y-auto bg-gray-50 p-4"
  aria-label="Message list"
>
  {#if messages.length === 0}
    <div class="rounded-xl border border-dashed border-gray-300 bg-white p-6 text-center text-sm text-gray-500">
      No messages yet. Start the conversation below.
    </div>
  {:else}
    <div class="space-y-1">
      {#each messages as message (message.id)}
        <MessageItem
          {message}
          {currentUserId}
          on:reply
          on:edit
          on:delete
        />
      {/each}
    </div>
  {/if}
</section>
