<script lang="ts">
  import type { ChatMessage } from './types'

  export let messages: ChatMessage[] = []

  function messageAuthor(message: ChatMessage) {
    return message.authorName ?? message.user_id ?? 'Someone'
  }

  function messageBody(message: ChatMessage) {
    return message.body ?? message.message ?? ''
  }

  function messageAttachments(message: ChatMessage) {
    return message.attachments ?? message.files ?? []
  }

  function formatTime(value: string | Date | undefined) {
    if (!value) return ''
    const date = value instanceof Date ? value : new Date(value)
    if (Number.isNaN(date.getTime())) return ''
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  }
</script>

<section class="flex-1 overflow-y-auto bg-gray-50 p-4" aria-label="Message list">
  {#if messages.length === 0}
    <div class="rounded-xl border border-dashed border-gray-300 bg-white p-6 text-center text-sm text-gray-500">
      No messages yet. Start the conversation below.
    </div>
  {:else}
    <div class="space-y-4">
      {#each messages as message (message.id)}
        <article class="rounded-xl bg-white p-4 shadow-sm" aria-label={`Message from ${messageAuthor(message)}`}>
          <header class="flex flex-wrap items-baseline gap-2">
            <h3 class="font-semibold text-gray-900">{messageAuthor(message)}</h3>
            {#if formatTime(message.createdAt ?? message.created_at)}
              <time class="text-xs text-gray-500" datetime={(message.createdAt ?? message.created_at)?.toString()}>{formatTime(message.createdAt ?? message.created_at)}</time>
            {/if}
          </header>

          {#if messageBody(message)}
            <p class="mt-2 whitespace-pre-wrap text-sm leading-6 text-gray-800">{messageBody(message)}</p>
          {/if}

          {#if messageAttachments(message).length}
            <ul class="mt-3 flex flex-wrap gap-2" aria-label="Message attachments">
              {#each messageAttachments(message) as attachment (attachment.id)}
                <li class="rounded-full border border-gray-200 bg-gray-50 px-3 py-1 text-sm text-gray-700">
                  {attachment.name ?? 'attached file'}
                </li>
              {/each}
            </ul>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>
