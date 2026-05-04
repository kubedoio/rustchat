<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { format, formatDistanceToNow } from 'date-fns'
  import { MessageSquare, Pencil, Trash2, Pin, Check, Bookmark, MoreHorizontal, X } from 'lucide-svelte'
  import type { ChatMessage } from './types'

  export let message: ChatMessage
  export let currentUserId: string | undefined = undefined

  const dispatch = createEventDispatcher<{
    reply: string
    thread: { messageId: string; channelId: string }
    edit: { id: string; content: string }
    delete: string
    openProfile: string
  }>()

  let showActions = false
  let showMenu = false
  let isEditing = false
  let editContent = ''
  let saving = false

  const quickEmojis = ['👍', '❤️', '😄']

  $: isOwnMessage = currentUserId !== undefined && message.user_id === currentUserId
  $: isEdited = Boolean(message.editedAt)
  $: isSystemMessage =
    message.props?.type === 'system_join_leave' ||
    message.props?.type === 'system_purpose' ||
    message.props?.type === 'system_header'

  $: timestamp = message.createdAt ?? message.created_at
  $: authorName = message.username ?? message.authorName ?? 'Someone'
  $: body = message.body ?? message.message ?? ''
  $: files = message.attachments ?? message.files ?? []
  $: reactions = message.reactions ?? []

  $: statusClasses = [
    message.status === 'sending' ? 'opacity-70' : '',
    message.status === 'failed' ? 'bg-danger/5' : '',
  ]
    .filter(Boolean)
    .join(' ')

  function formatTime(value: string | Date | undefined) {
    if (!value) return ''
    const date = value instanceof Date ? value : new Date(value)
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

  function handleReply() {
    dispatch('reply', message.id)
  }

  function handleThreadClick() {
    dispatch('thread', {
      messageId: message.id,
      channelId: message.channelId ?? message.channel_id ?? '',
    })
  }

  function startEditing() {
    editContent = body
    isEditing = true
    showMenu = false
    requestAnimationFrame(() => {
      const el = document.getElementById(`edit-textarea-${message.id}`) as HTMLTextAreaElement | null
      el?.focus()
      el?.select()
    })
  }

  function cancelEditing() {
    isEditing = false
    editContent = ''
  }

  function saveEdit() {
    const trimmed = editContent.trim()
    if (!trimmed || trimmed === body) {
      cancelEditing()
      return
    }
    saving = true
    dispatch('edit', { id: message.id, content: trimmed })
    saving = false
    isEditing = false
  }

  function handleEditKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      cancelEditing()
    } else if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      saveEdit()
    }
  }

  function handleDelete() {
    if (!confirm('Delete this message?')) return
    dispatch('delete', message.id)
  }

  function hasUserReacted(reaction: { users: string[] }) {
    return currentUserId !== undefined && reaction.users.includes(currentUserId)
  }

  function toggleReaction(emoji: string) {
    if (!currentUserId) return
    reactions = reactions
      .map((r) => {
        if (r.emoji !== emoji) return r
        const reacted = r.users.includes(currentUserId)
        return {
          ...r,
          count: reacted ? r.count - 1 : r.count + 1,
          users: reacted ? r.users.filter((id) => id !== currentUserId) : [...r.users, currentUserId],
        }
      })
      .filter((r) => r.count > 0)
  }
</script>

{#if isSystemMessage}
  <div class="flex items-center px-3 py-1 hover:bg-bg-app/50 transition-standard">
    <div class="flex items-center text-xs text-text-3 italic w-full justify-center">
      <span class="message-content">{body}</span>
      {#if formatTime(timestamp)}
        <span class="ml-2 text-[10px] text-text-4">{formatTime(timestamp)}</span>
      {/if}
    </div>
  </div>
{:else}
  <div
    class="flex items-start group transition-standard relative px-2 sm:px-3 py-1 hover:bg-bg-app/30 {statusClasses}"
    on:mouseenter={() => (showActions = true)}
    on:mouseleave={() => {
      showActions = false
      showMenu = false
    }}
    role="article"
    aria-label={`Message from ${authorName}`}
  >
    <!-- Avatar -->
    <div
      data-testid="message-avatar"
      class="shrink-0 select-none mr-2 sm:mr-3 mt-0.5 cursor-pointer"
      on:click={() => message.user_id && dispatch('openProfile', message.user_id)}
      role="button"
      tabindex="0"
      aria-label={`Open profile for ${authorName}`}
      on:keydown={(e) => e.key === 'Enter' && message.user_id && dispatch('openProfile', message.user_id)}
    >
      {#if message.avatarUrl}
        <img src={message.avatarUrl} alt={authorName} class="w-8 h-8 rounded-r-1 object-cover" />
      {:else}
        <div class="w-8 h-8 rounded-r-1 bg-brand/10 text-brand flex items-center justify-center text-xs font-bold">
          {getInitials(authorName)}
        </div>
      {/if}
    </div>

    <div class="flex-1 min-w-0">
      <!-- Header -->
      <div class="flex items-baseline gap-1.5 flex-wrap">
        <span
          class="font-semibold text-sm text-text-1 hover:underline cursor-pointer transition-colors hover:text-brand"
          on:click={() => message.user_id && dispatch('openProfile', message.user_id)}
          role="button"
          tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && message.user_id && dispatch('openProfile', message.user_id)}
        >
          {authorName}
        </span>
        {#if formatTime(timestamp)}
          <span class="text-xs text-text-3 hover:underline cursor-pointer">{formatTime(timestamp)}</span>
        {/if}
        {#if isEdited}
          <span class="text-[10px] text-text-3">(edited)</span>
        {/if}
        {#if message.status === 'sending'}
          <span class="text-[10px] text-text-3 italic animate-pulse">Sending...</span>
        {/if}
        {#if message.status === 'failed'}
          <span class="text-[10px] text-danger font-medium">Failed</span>
        {/if}
        {#if message.isPinned || message.isSaved}
          <div class="flex items-center gap-1">
            {#if message.isPinned}
              <span class="bg-bg-surface-2 text-[10px] px-1.5 py-0.5 rounded text-text-3 font-medium flex items-center">
                <Pin class="w-2.5 h-2.5 mr-0.5" />
                Pinned
              </span>
            {/if}
            {#if message.isSaved}
              <span class="bg-warning/10 text-[10px] px-1.5 py-0.5 rounded text-warning font-medium flex items-center">
                <Bookmark class="w-2.5 h-2.5 mr-0.5 fill-current" />
                Saved
              </span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Edit mode -->
      {#if isEditing}
        <div class="mt-1 max-w-[95%]">
          <textarea
            id={`edit-textarea-${message.id}`}
            bind:value={editContent}
            on:keydown={handleEditKeydown}
            rows={2}
            class="w-full px-3 py-2 border border-brand rounded-r-2 bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-brand/20 focus:outline-none text-sm"
          ></textarea>
          <div class="flex items-center gap-2 mt-1.5">
            <button
              on:click={saveEdit}
              disabled={saving}
              class="flex items-center gap-1 rounded-r-1 bg-brand px-3 py-1.5 text-xs font-medium text-brand-foreground transition-standard hover:bg-brand-hover disabled:opacity-50"
            >
              <Check class="w-3 h-3" />
              <span>{saving ? 'Saving...' : 'Save'}</span>
            </button>
            <button
              on:click={cancelEditing}
              disabled={saving}
              class="px-3 py-1.5 bg-bg-surface-2 text-text-2 text-xs font-medium rounded-r-1 hover:bg-bg-surface-1 transition-standard flex items-center gap-1"
            >
              <X class="w-3 h-3" />
              <span>Cancel</span>
            </button>
            <span class="text-xs text-text-3">Esc to cancel • Enter to save</span>
          </div>
        </div>
      {:else}
        <!-- Message Content -->
        <div class="relative">
          <div class="message-content text-text-1 text-sm mt-0.5 whitespace-pre-wrap leading-relaxed max-w-full break-words">
            {body}
          </div>
        </div>

        <!-- Files -->
        {#if files.length > 0}
          <div class="mt-2 flex flex-wrap gap-2">
            {#each files as file (file.id)}
              {#if file.mimeType?.startsWith('image/') || file.mime_type?.startsWith('image/')}
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

        <!-- Thread Reply Count -->
        {#if message.threadCount && message.threadCount > 0}
          <div class="mt-1.5">
            <button
              data-testid="thread-count-button"
              on:click={handleThreadClick}
              class="inline-flex items-center gap-2 px-2 py-1 rounded-r-1 hover:bg-brand/5 transition-standard border border-transparent hover:border-brand/20"
            >
              <MessageSquare class="w-3.5 h-3.5 text-brand" />
              <span class="text-[13px] font-medium text-brand">
                {message.threadCount} {message.threadCount === 1 ? 'reply' : 'replies'}
              </span>
              {#if message.lastReplyAt}
                <span class="text-[11px] text-text-3">
                  Last reply {formatDistanceToNow(new Date(message.lastReplyAt))} ago
                </span>
              {/if}
            </button>
          </div>
        {/if}

        <!-- Reactions -->
        {#if reactions.length > 0}
          <div class="flex items-center mt-1.5 gap-1.5 flex-wrap">
            {#each reactions as reaction (reaction.emoji)}
              {#if hasUserReacted(reaction)}
                <button
                  on:click={() => toggleReaction(reaction.emoji)}
                  class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border transition-all hover:scale-105 bg-brand/10 border-brand/30 text-brand"
                >
                  <span>{reaction.emoji}</span>
                  <span class="font-medium">{reaction.count}</span>
                </button>
              {:else}
                <button
                  on:click={() => toggleReaction(reaction.emoji)}
                  class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border transition-all hover:scale-105 bg-bg-surface-2 border-border-1 text-text-2 hover:border-border-2"
                >
                  <span>{reaction.emoji}</span>
                  <span class="font-medium">{reaction.count}</span>
                </button>
              {/if}
            {/each}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Hover Actions -->
    {#if showActions && !isEditing}
      <div
        class="absolute right-2 sm:right-4 top-0 -translate-y-1/2 flex items-center bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 px-1 py-0.5 z-10"
      >
        <!-- Quick Reactions -->
        <div class="flex items-center border-r border-border-1 pr-1 mr-1">
          {#each quickEmojis as emoji}
            <button
              on:click={() => toggleReaction(emoji)}
              class="p-1.5 hover:bg-bg-surface-2 rounded transition-colors text-base leading-none"
              title={`React with ${emoji}`}
            >
              {emoji}
            </button>
          {/each}
        </div>

        <button
          on:click={handleReply}
          class="p-1.5 hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-colors rounded"
          title="Reply in thread"
        >
          <MessageSquare class="w-4 h-4" />
        </button>

        <div class="relative">
          <button
            on:click|stopPropagation={() => (showMenu = !showMenu)}
            class="p-1.5 hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-colors rounded"
            title="More actions"
          >
            <MoreHorizontal class="w-4 h-4" />
          </button>

          <!-- Dropdown Menu -->
          {#if showMenu}
            <div
              class="absolute right-0 top-full mt-1 w-44 bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2xl py-1 z-20 origin-top-right"
            >
              {#if isOwnMessage}
                <button
                  on:click={startEditing}
                  class="w-full px-3 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center gap-2 transition-standard"
                >
                  <Pencil class="w-4 h-4" />
                  Edit message
                </button>
              {/if}
              <button
                on:click={handleDelete}
                class="w-full px-3 py-2 text-left text-sm text-danger hover:bg-danger/5 flex items-center gap-2 transition-standard"
              >
                <Trash2 class="w-4 h-4" />
                Delete message
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}
