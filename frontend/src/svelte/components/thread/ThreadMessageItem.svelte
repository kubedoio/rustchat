<script lang="ts">
  import { format } from 'date-fns'
  import type { SvelteChatPost } from '../../stores/chat'

  export let message: SvelteChatPost

  $: authorName = message.username ?? 'Unknown User'
  $: body = message.message ?? ''
  $: timestamp = message.created_at

  function formatTime(value: string | undefined) {
    if (!value) return ''
    const date = new Date(value)
    if (Number.isNaN(date.getTime())) return ''
    return format(date, 'h:mm a')
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

<div class="flex items-start space-x-3">
  <!-- User Avatar -->
  <div class="shrink-0 mt-0.5">
    {#if message.avatar_url}
      <img
        src={message.avatar_url}
        alt={authorName}
        class="w-8 h-8 rounded-lg object-cover"
      />
    {:else}
      <div class="w-8 h-8 rounded-lg bg-brand/10 text-brand flex items-center justify-center text-xs font-bold">
        {getInitials(authorName)}
      </div>
    {/if}
  </div>

  <!-- Content -->
  <div class="flex-1 min-w-0">
    <!-- Header -->
    <div class="flex items-baseline space-x-2 mb-0.5">
      <span class="font-bold text-sm text-text-1 leading-tight">
        {authorName}
      </span>
      <span class="text-[11px] text-text-3 font-medium">
        {formatTime(timestamp)}
      </span>
    </div>

    <!-- Message Content -->
    <div class="text-[14px] text-text-2 leading-normal break-words whitespace-pre-wrap">
      {body}
    </div>

    <!-- Files -->
    {#if message.files && message.files.length > 0}
      <div class="mt-3 flex flex-wrap gap-2">
        {#each message.files as file (file.id)}
          {#if file.mime_type?.startsWith('image/') || file.mimeType?.startsWith('image/')}
            <div class="rounded-r-1 overflow-hidden border border-border-1 w-20 h-20 bg-bg-surface-2">
              <img
                src={file.url ?? ''}
                alt={file.name ?? 'Image'}
                class="w-full h-full object-cover"
                loading="lazy"
              />
            </div>
          {:else}
            <div class="rounded-full border border-border-1 bg-bg-surface-2 px-3 py-1 text-xs text-text-2 flex items-center gap-1.5">
              <span class="truncate max-w-[12rem]">{file.name ?? 'Attached file'}</span>
              {#if file.size}
                <span class="text-text-4">({Math.round(file.size / 1024)}KB)</span>
              {/if}
            </div>
          {/if}
        {/each}
      </div>
    {/if}

    <!-- Reactions -->
    {#if message.reactions && message.reactions.length > 0}
      <div class="flex items-center mt-2 gap-1.5 flex-wrap">
        {#each message.reactions as reaction (reaction.emoji)}
          <div
            class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border bg-bg-surface-2 border-border-1 text-text-2"
          >
            <span>{reaction.emoji}</span>
            <span class="font-medium">{reaction.count}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
