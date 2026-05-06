<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { createEventDispatcher } from 'svelte'
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { format } from 'date-fns'
  import { X, MessageSquare, Send, Loader2 } from 'lucide-svelte'
  import { chatStore } from '../../stores/chat'
  import ThreadMessageItem from './ThreadMessageItem.svelte'

  export let threadId: string | null = null
  export let channelId: string | null = null
  export let open = false

  const dispatch = createEventDispatcher<{ close: void }>()

  let composerRef: HTMLTextAreaElement | null = null
  let draft = ''
  let isSending = false
  let listRef: HTMLDivElement | null = null
  let isScrolledToBottom = true

  $: state = $chatStore
  $: parentMessage =
    channelId && threadId
      ? (state.messagesByChannel[channelId]?.find((m) => m.id === threadId) ?? null)
      : null
  $: replies = threadId ? (state.threadsByParent[threadId] ?? []) : []
  $: replyCount = replies.length

  // Fetch replies when threadId changes and panel is open
  $: if (open && threadId) {
    chatStore.fetchThreadReplies(threadId).catch((err: unknown) => {
      console.error('Failed to load thread:', err)
    })
  }

  // Focus composer when thread opens
  $: if (open && composerRef) {
    setTimeout(() => composerRef?.focus(), 100)
  }

  // Scroll to bottom when replies change (if already at bottom)
  $: if (replies.length > 0 && isScrolledToBottom && listRef) {
    tick().then(() => {
      if (listRef) {
        listRef.scrollTo({ top: listRef.scrollHeight, behavior: 'smooth' })
      }
    })
  }

  function handleClose() {
    draft = ''
    dispatch('close')
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault()
      handleClose()
      return
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      sendReply()
    }
  }

  async function sendReply() {
    const trimmed = draft.trim()
    if (!trimmed || !threadId || !channelId || isSending) return
    isSending = true
    try {
      await chatStore.sendThreadReply(threadId, channelId, trimmed)
      draft = ''
      tick().then(() => {
        if (listRef) {
          listRef.scrollTo({ top: listRef.scrollHeight, behavior: 'smooth' })
        }
      })
    } catch (error) {
      console.error('Failed to send reply:', error)
    } finally {
      isSending = false
    }
  }

  function handleScroll() {
    if (!listRef) return
    const { scrollTop, scrollHeight, clientHeight } = listRef
    isScrolledToBottom = scrollHeight - scrollTop - clientHeight < 50
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      const activeElement = document.activeElement
      const isInComposer = activeElement === composerRef
      const isInOtherInput =
        activeElement instanceof HTMLInputElement ||
        (activeElement instanceof HTMLTextAreaElement && !isInComposer)

      if (!isInOtherInput || isInComposer) {
        handleClose()
      }
    }
  }

  onMount(() => {
    document.addEventListener('keydown', handleGlobalKeydown)
    return () => {
      document.removeEventListener('keydown', handleGlobalKeydown)
    }
  })

  function getInitials(name: string) {
    return name
      .split(' ')
      .map((n) => n[0])
      .filter(Boolean)
      .join('')
      .toUpperCase()
      .slice(0, 2)
  }

  function formatTime(value: string | undefined) {
    if (!value) return ''
    const date = new Date(value)
    if (Number.isNaN(date.getTime())) return ''
    return format(date, 'MMM d, h:mm a')
  }
</script>

{#if open}
  <!-- Overlay for mobile -->
  <div
    class="thread-panel-overlay fixed inset-0 bg-black/40 backdrop-blur-sm z-30 lg:hidden"
    role="button"
    tabindex="-1"
    aria-label="Close thread"
    onclick={(e) => {
      if ((e.target as HTMLElement).classList.contains('thread-panel-overlay')) {
        handleClose()
      }
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        handleClose()
      }
    }}
  ></div>

  <div
    data-testid="thread-panel"
    class="fixed lg:relative top-0 right-0 h-full w-full sm:w-[400px] lg:w-[var(--rhs-width)] bg-bg-surface-1 border-l border-border-1 z-40 flex flex-col shadow-2xl"
    role="dialog"
    aria-modal="true"
    aria-label="Thread panel"
    tabindex="-1"
    onkeydown={handleKeydown}
    transition:fly={{ x: 300, duration: 250, easing: cubicOut }}
  >
    <!-- Header -->
    <div class="border-b border-border-1 bg-bg-surface-2">
      <div class="h-12 flex items-center justify-between px-4 border-b border-border-1">
        <div class="flex items-center gap-2">
          <MessageSquare class="w-5 h-5 text-text-2" />
          <h3 class="font-bold text-[15px] text-text-1 uppercase tracking-wider">Thread</h3>
          {#if replyCount > 0}
            <span data-testid="thread-reply-count" class="text-sm text-text-3">
              ({replyCount}
              {replyCount === 1 ? 'reply' : 'replies'})
            </span>
          {/if}
        </div>
        <button
          onclick={handleClose}
          class="p-1.5 hover:bg-bg-surface-1 rounded-lg text-text-3 hover:text-text-1 transition-standard focus-ring"
          aria-label="Close thread"
          title="Close thread"
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Parent Message -->
      {#if parentMessage}
        <div class="p-5">
          <div class="flex items-start space-x-3">
            <div class="shrink-0 mt-0.5">
              {#if parentMessage.avatar_url}
                <img
                  src={parentMessage.avatar_url}
                  alt={parentMessage.username ?? 'Unknown'}
                  class="w-10 h-10 rounded-lg object-cover"
                />
              {:else}
                <div
                  class="w-10 h-10 rounded-lg bg-brand/10 text-brand flex items-center justify-center text-xs font-bold"
                >
                  {getInitials(parentMessage.username ?? 'Unknown')}
                </div>
              {/if}
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-baseline space-x-2 mb-1">
                <span class="font-bold text-[15px] text-text-1 leading-tight">
                  {parentMessage.username ?? 'Unknown User'}
                </span>
                <span class="text-[11px] text-text-3 font-medium">
                  {formatTime(parentMessage.created_at)}
                </span>
              </div>
              <div class="text-[15px] text-text-1 leading-relaxed break-words whitespace-pre-wrap">
                {parentMessage.message ?? ''}
              </div>
            </div>
          </div>
        </div>
      {:else}
        <div class="p-5">
          <div class="flex items-center gap-3 text-text-3">
            <div class="w-10 h-10 rounded-lg bg-bg-surface-1 flex items-center justify-center">
              <MessageSquare class="w-5 h-5" />
            </div>
            <div>
              <p class="text-sm font-medium text-text-2">Message not found</p>
              <p class="text-xs">This message is no longer available</p>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Replies List -->
    <div
      bind:this={listRef}
      class="flex-1 overflow-y-auto p-5 space-y-5 custom-scrollbar"
      onscroll={handleScroll}
    >
      {#if replies.length === 0}
        <div class="flex flex-col items-center justify-center py-12 text-center">
          <div class="w-16 h-16 bg-bg-surface-2 rounded-full flex items-center justify-center mb-4">
            <MessageSquare class="w-8 h-8 text-text-3" />
          </div>
          <p class="text-[15px] font-semibold text-text-1 mb-1">No replies yet</p>
          <p class="text-sm text-text-3">Be the first to share your thoughts!</p>
        </div>
      {:else}
        {#each replies as reply (reply.id)}
          <ThreadMessageItem message={reply} />
        {/each}
      {/if}
    </div>

    <!-- Reply Composer -->
    <div class="p-4 border-t border-border-1 bg-bg-surface-2">
      <div
        class="flex items-end space-x-2 bg-bg-surface-1 border border-border-1 rounded-xl focus-within:ring-2 focus-within:ring-brand/40 focus-within:border-brand/50 transition-all p-1.5 shadow-sm"
      >
        <textarea
          bind:this={composerRef}
          bind:value={draft}
          onkeydown={handleKeydown}
          rows={2}
          data-testid="thread-composer"
          class="flex-1 px-3 py-2 bg-transparent text-text-1 resize-none border-none focus:ring-0 text-[14px] scrollbar-none"
          placeholder="Reply to thread..."
          disabled={isSending}
        ></textarea>

        <button
          onclick={sendReply}
          disabled={!draft.trim() || isSending}
          class="mb-1 mr-1 flex items-center justify-center rounded-lg bg-brand p-2.5 text-brand-foreground shadow-lg shadow-brand/20 transition-all hover:bg-brand-hover active:scale-95 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {#if isSending}
            <Loader2 class="w-4 h-4 animate-spin" />
          {:else}
            <Send class="w-4 h-4" />
          {/if}
        </button>
      </div>

      <div class="mt-2 text-[11px] text-text-3 text-right">
        <span>Press </span>
        <kbd class="px-1.5 py-0.5 bg-bg-surface-1 border border-border-1 rounded text-[10px] font-mono">Enter</kbd>
        <span> to send, </span>
        <kbd class="px-1.5 py-0.5 bg-bg-surface-1 border border-border-1 rounded text-[10px] font-mono">Shift+Enter</kbd>
        <span> for new line</span>
      </div>
    </div>
  </div>
{/if}
