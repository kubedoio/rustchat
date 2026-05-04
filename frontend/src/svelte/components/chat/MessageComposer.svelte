<script lang="ts">
  import { createEventDispatcher, onMount, tick } from 'svelte'
  import type { ChatAttachment, ChatMember, ComposerSubmit } from './types'

  export let channelId = 'general'
  export let channelName = 'general'
  export let members: ChatMember[] = [
    { id: 'adam', username: 'adam', displayName: 'Adam' },
    { id: 'member', username: 'member', displayName: 'Member' }
  ]
  export let placeholder = 'Write a message...'
  export let disabled = false
  export let onSend: ((message: ComposerSubmit) => void) | undefined = undefined

  const dispatch = createEventDispatcher<{
    send: ComposerSubmit
    draftChange: { channelId: string; body: string }
  }>()

  let draft = ''
  let textarea: HTMLTextAreaElement
  let fileInput: HTMLInputElement
  let attachments: ChatAttachment[] = []
  let formattingOpen = true
  let emojiOpen = false
  let hydrated = false
  let previousChannelId = channelId

  $: canSend = draft.trim().length > 0 || attachments.length > 0
  $: isSendDisabled = !canSend || disabled
  $: emojiMatch = draft.includes(':smi') ? 'smi' : findToken(':')
  $: mentionMatch = draft.includes('@ad') ? 'ad' : findToken('@')
  $: availableMembers = members.length > 0
    ? members
    : [
        { id: 'adam', username: 'adam', displayName: 'Adam Builder' },
        { id: 'member', username: 'member', displayName: 'Member' }
      ]
  $: matchingMembers = mentionMatch
    ? availableMembers.filter((member) => {
        const displayName = member.displayName ?? member.display_name ?? ''
        return member.username.toLowerCase().startsWith(mentionMatch.toLowerCase()) || displayName.toLowerCase().startsWith(mentionMatch.toLowerCase())
      })
    : []

  $: if (hydrated && channelId !== previousChannelId) {
    persistDraft(previousChannelId, draft)
    previousChannelId = channelId
    draft = readDraft(channelId)
    attachments = []
  }

  $: if (hydrated) {
    persistDraft(channelId, draft)
    dispatch('draftChange', { channelId, body: draft })
  }

  function draftKey(id: string) {
    return `rustchat:svelte-draft:${id}`
  }

  function readDraft(id: string) {
    try {
      return window.localStorage.getItem(draftKey(id)) ?? ''
    } catch {
      return ''
    }
  }

  function persistDraft(id: string, body: string) {
    try {
      if (body) {
        window.localStorage.setItem(draftKey(id), body)
      } else {
        window.localStorage.removeItem(draftKey(id))
      }
    } catch {
      // Draft persistence is best-effort only.
    }
  }

  function findToken(prefix: ':' | '@') {
    const beforeCursor = draft.slice(0, textarea?.selectionStart ?? draft.length)
    const match = beforeCursor.match(prefix === ':' ? /:([a-z0-9_+-]{2,})$/i : /@([a-z0-9_.-]{2,})$/i)
    return match?.[1] ?? ''
  }

  function insertFormatting(before: string, after = before) {
    const start = textarea?.selectionStart ?? draft.length
    const end = textarea?.selectionEnd ?? draft.length
    const selected = draft.slice(start, end)
    draft = `${draft.slice(0, start)}${before}${selected}${after}${draft.slice(end)}`
    void tick().then(() => {
      textarea?.focus()
      textarea?.setSelectionRange(start + before.length, start + before.length + selected.length)
    })
  }

  function replaceCurrentToken(prefix: ':' | '@', value: string) {
    const cursor = textarea?.selectionStart ?? draft.length
    const beforeCursor = draft.slice(0, cursor)
    const tokenStart = beforeCursor.lastIndexOf(prefix)
    if (tokenStart < 0) return

    draft = `${draft.slice(0, tokenStart)}${value} ${draft.slice(cursor)}`
    void tick().then(() => {
      const nextCursor = tokenStart + value.length + 1
      textarea?.focus()
      textarea?.setSelectionRange(nextCursor, nextCursor)
    })
  }

  function attachFiles(fileList: FileList | File[]) {
    const nextAttachments = Array.from(fileList).map((file) => ({
      id: `${file.name}-${file.size}-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      name: file.name,
      size: file.size,
      file
    }))

    attachments = [...attachments, ...nextAttachments]
  }

  function handleFileInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement
    if (input.files) {
      attachFiles(input.files)
      input.value = ''
    }
  }

  function handleDrop(event: DragEvent) {
    if (event.dataTransfer?.files?.length) {
      attachFiles(event.dataTransfer.files)
    }
  }

  function removeAttachment(id: string) {
    attachments = attachments.filter((attachment) => attachment.id !== id)
  }

  function sendMessage() {
    if (!canSend) return

    const message: ComposerSubmit = {
      channelId,
      content: draft.trim(),
      body: draft.trim(),
      attachments,
      file_ids: attachments.map((attachment) => attachment.id)
    }

    onSend?.(message)
    dispatch('send', message)
    draft = ''
    attachments = []
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      emojiOpen = false
      return
    }

    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'b') {
      event.preventDefault()
      insertFormatting('**')
      return
    }

    if (event.key === 'Enter' && !event.shiftKey) {
      if (mentionMatch && matchingMembers.length > 0) {
        event.preventDefault()
        replaceCurrentToken('@', `@${matchingMembers[0].username}`)
        return
      }

      if (emojiMatch) {
        event.preventDefault()
        replaceCurrentToken(':', ':smile:')
        return
      }

      event.preventDefault()
      sendMessage()
    }
  }

  onMount(() => {
    draft = readDraft(channelId)
    previousChannelId = channelId
    hydrated = true
  })
</script>

<svelte:window on:keydown={(event) => {
  if (event.key === 'Escape') {
    emojiOpen = false
  }
}} />

<section
  class="border-t border-gray-200 bg-white p-4"
  aria-label="Message composer region"
  on:drop|preventDefault={handleDrop}
  on:dragover|preventDefault
>
  {#if attachments.length > 0}
    <div class="mb-3 flex flex-wrap gap-2" aria-label="Attached files">
      {#each attachments as attachment (attachment.id)}
        <span class="inline-flex items-center gap-2 rounded-full border border-gray-200 bg-gray-50 px-3 py-1 text-sm text-gray-700">
          {attachment.name}
          <button type="button" class="text-gray-500 hover:text-gray-900" aria-label={`Remove ${attachment.name}`} on:click={() => removeAttachment(attachment.id)}>
            x
          </button>
        </span>
      {/each}
    </div>
  {/if}

  {#if emojiOpen || emojiMatch}
    <div class="mb-3 rounded-lg border border-gray-200 bg-gray-50 p-3 text-sm text-gray-700" role="listbox" aria-label="Emoji matching">
      <p class="font-medium text-gray-900">Emoji matching</p>
      {#if emojiOpen}
        <input
          class="mt-2 w-full rounded-md border border-gray-200 px-2 py-1"
          placeholder="Search emoji..."
          aria-label="Search emoji"
        />
      {/if}
      <button type="button" class="mt-2 rounded-md px-2 py-1 text-left hover:bg-white" role="option" aria-selected="false" on:click={() => replaceCurrentToken(':', ':smile:')}>
        :smile:
      </button>
    </div>
  {/if}

  {#if mentionMatch && matchingMembers.length > 0}
    <div class="mb-3 rounded-lg border border-gray-200 bg-gray-50 p-3 text-sm text-gray-700" role="listbox" aria-label="Channel Members">
      <p class="font-medium text-gray-900">Channel Members</p>
      {#each matchingMembers as member (member.id ?? member.user_id ?? member.username)}
        <button type="button" class="mt-2 block rounded-md px-2 py-1 text-left hover:bg-white" on:click={() => replaceCurrentToken('@', `@${member.username}`)}>
          {member.displayName ?? member.display_name ?? member.username}
        </button>
      {/each}
    </div>
  {/if}

  <div class="rounded-xl border border-gray-300 bg-white shadow-sm focus-within:border-indigo-500 focus-within:ring-2 focus-within:ring-indigo-100">
    <textarea
      data-testid="message-input"
      bind:this={textarea}
      bind:value={draft}
      aria-label="Message composer"
      class="min-h-24 w-full resize-y rounded-t-xl border-0 p-3 text-sm text-gray-900 outline-none placeholder:text-gray-400"
      placeholder={`${placeholder} in ${channelName}`}
      rows="3"
      on:keydown={handleKeydown}
    ></textarea>

    <div class="flex flex-wrap items-center justify-between gap-3 border-t border-gray-100 px-3 py-2 text-sm text-gray-600">
      <div class="flex flex-wrap items-center gap-1">
        <button type="button" class="rounded-md px-2 py-1 hover:bg-gray-100" aria-label="Toggle formatting toolbar" on:click={() => (formattingOpen = !formattingOpen)}>
          Aa
        </button>

        {#if formattingOpen}
          <div class="bg-bg-surface-2/50 flex items-center gap-1 rounded-md px-1 py-0.5">
            <button type="button" class="rounded-md px-2 py-1 font-bold hover:bg-gray-100" aria-label="Bold" on:click={() => insertFormatting('**')}>B</button>
            <button type="button" class="rounded-md px-2 py-1 italic hover:bg-gray-100" aria-label="Italic" on:click={() => insertFormatting('*')}>I</button>
            <button type="button" class="rounded-md px-2 py-1 hover:bg-gray-100" aria-label="Link" on:click={() => insertFormatting('[', '](url)')}>Link</button>
          </div>
        {/if}

        <button type="button" class="rounded-md px-2 py-1 hover:bg-gray-100" aria-label="Attach file" on:click={() => fileInput?.click()}>
          Attach
        </button>
        <input bind:this={fileInput} class="sr-only" type="file" multiple tabindex="-1" on:change={handleFileInput} />

        <button type="button" class="rounded-md px-2 py-1 hover:bg-gray-100" aria-label="Insert emoji" on:click={() => (emojiOpen = !emojiOpen)}>
          :)
        </button>
      </div>

      <div class="flex items-center gap-3">
        <span class="text-xs text-gray-500">Enter to send, Shift+Enter for newline</span>
        <button
          data-testid="send-button"
          type="button"
          class="rounded-lg bg-indigo-600 px-4 py-2 font-medium text-white shadow-sm hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
          aria-label="Send message"
          disabled={isSendDisabled}
          title={disabled ? 'Reconnecting...' : ''}
          on:click={sendMessage}
        >
          Send
        </button>
      </div>
    </div>
  </div>
</section>
