<script lang="ts">
  import { createEventDispatcher, onMount, tick } from 'svelte'
  import { Hash, Phone, PhoneOff, Eye } from 'lucide-svelte'
  import { chatStore } from '../../stores/chat'
  import { callsStore } from '../../stores/calls.svelte'
  import { emojiMap } from '../../utils/emoji'
  import { renderMarkdown } from '@/utils/markdown'
  import EmojiPicker from '../../components/composer/EmojiPicker.svelte'
  import type { ChatAttachment, ChatMember, ComposerSubmit } from './types'
  import type { SvelteChatChannel } from '../../stores/chat'

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
  let emojiButtonEl: HTMLButtonElement | null = null
  let hydrated = false
  let previousChannelId = channelId
  let isDragging = false
  let showPreview = false

  const commands = [
    { id: 'call-start', label: 'Start audio call', icon: Phone },
    { id: 'call-join', label: 'Join active call', icon: Phone },
    { id: 'call-leave', label: 'Leave call', icon: PhoneOff },
  ]

  $: hasUploadedFiles = attachments.some((a) => a.fileId && !a.uploading)
  $: hasUploadInProgress = attachments.some((a) => a.uploading)
  $: canSend = (draft.trim().length > 0 || hasUploadedFiles) && !hasUploadInProgress
  $: isSendDisabled = !canSend || disabled
  $: emojiMatch = findToken(':')
  $: matchingEmojis = emojiMatch
    ? Object.entries(emojiMap)
        .filter(([name]) => name.toLowerCase().startsWith(emojiMatch.toLowerCase()))
        .slice(0, 8)
    : []
  $: mentionMatch = findToken('@')
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
  $: channelMatch = findToken('~')
  $: matchingChannels = channelMatch
    ? ($chatStore.channels ?? []).filter((ch) =>
        ch.name.toLowerCase().startsWith(channelMatch.toLowerCase()) ||
        (ch.display_name ?? '').toLowerCase().startsWith(channelMatch.toLowerCase())
      )
    : []
  $: commandMatch = draft.includes('^k') ? (draft.split('^k').pop()?.trim() ?? '') : ''
  $: matchingCommands = commandMatch
    ? commands.filter((cmd) => cmd.label.toLowerCase().includes(commandMatch.toLowerCase()))
    : []
  $: previewHtml = renderMarkdown(draft)

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

  function findToken(prefix: ':' | '@' | '~') {
    const beforeCursor = draft.slice(0, textarea?.selectionStart ?? draft.length)
    const match = beforeCursor.match(
      prefix === ':'
        ? /:([a-z0-9_+-]{2,})$/i
        : prefix === '~'
          ? /~([a-z0-9_-]{1,})$/i
          : /@([a-z0-9_.-]{2,})$/i
    )
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

  function insertAtCursor(text: string) {
    const start = textarea?.selectionStart ?? draft.length
    const end = textarea?.selectionEnd ?? draft.length
    draft = `${draft.slice(0, start)}${text} ${draft.slice(end)}`
    void tick().then(() => {
      const nextCursor = start + text.length + 1
      textarea?.focus()
      textarea?.setSelectionRange(nextCursor, nextCursor)
    })
  }

  function replaceCurrentToken(prefix: ':' | '@' | '~', value: string) {
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
    const nextAttachments: ChatAttachment[] = Array.from(fileList).map((file) => ({
      id: `${file.name}-${file.size}-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      name: file.name,
      size: file.size,
      file,
      mimeType: file.type,
      uploading: true,
      progress: 0,
    }))

    attachments = [...attachments, ...nextAttachments]

    nextAttachments.forEach((attachment) => {
      if (!attachment.file) return
      chatStore
        .uploadFile(attachment.file)
        .then((uploadedFile) => {
          attachments = attachments.map((a) =>
            a.id === attachment.id
              ? {
                  ...a,
                  uploading: false,
                  uploadError: false,
                  fileId: uploadedFile.id,
                  url: uploadedFile.url,
                  mimeType: uploadedFile.mimeType ?? uploadedFile.mime_type,
                  mime_type: uploadedFile.mime_type ?? uploadedFile.mimeType,
                }
              : a,
          )
        })
        .catch(() => {
          attachments = attachments.map((a) =>
            a.id === attachment.id ? { ...a, uploading: false, uploadError: true } : a,
          )
        })
    })
  }

  function handleFileInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement
    if (input.files) {
      attachFiles(input.files)
      input.value = ''
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault()
    isDragging = true
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault()
    isDragging = false
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault()
    isDragging = false
    const files = e.dataTransfer?.files
    if (files?.length) {
      attachFiles(files)
    }
  }

  function removeAttachment(id: string) {
    attachments = attachments.filter((attachment) => attachment.id !== id)
  }

  function sendMessage() {
    if (!canSend) return

    const file_ids = attachments
      .filter((attachment) => attachment.fileId && !attachment.uploading)
      .map((attachment) => attachment.fileId!)

    const message: ComposerSubmit = {
      channelId,
      content: draft.trim(),
      body: draft.trim(),
      attachments,
      file_ids,
    }

    onSend?.(message)
    dispatch('send', message)
    draft = ''
    attachments = []
  }

  function insertChannel(channel: SvelteChatChannel) {
    replaceCurrentToken('~', `~${channel.name}`)
  }

  function insertEmoji(name: string) {
    replaceCurrentToken(':', `:${name}:`)
    emojiOpen = false
  }

  function executeCommand(cmdId: string) {
    if (cmdId === 'call-start') {
      callsStore.startCall(channelId)
    } else if (cmdId === 'call-join') {
      callsStore.joinCall(channelId)
    } else if (cmdId === 'call-leave') {
      callsStore.leaveCall()
    }
    draft = ''
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

    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'i') {
      event.preventDefault()
      insertFormatting('*', '*')
      return
    }

    if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === '7') {
      event.preventDefault()
      insertAtCursor('1. ')
      return
    }

    if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === '8') {
      event.preventDefault()
      insertAtCursor('- ')
      return
    }

    if (event.key === 'Enter' && !event.shiftKey) {
      if (commandMatch && matchingCommands.length > 0) {
        event.preventDefault()
        executeCommand(matchingCommands[0].id)
        return
      }

      if (mentionMatch && matchingMembers.length > 0) {
        event.preventDefault()
        replaceCurrentToken('@', `@${matchingMembers[0].username}`)
        return
      }

      if (channelMatch && matchingChannels.length > 0) {
        event.preventDefault()
        replaceCurrentToken('~', `~${matchingChannels[0].name}`)
        return
      }

      if (emojiMatch && matchingEmojis.length > 0) {
        event.preventDefault()
        replaceCurrentToken(':', `:${matchingEmojis[0][0]}:`)
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

<div
  class="relative"
  role="region"
  aria-label="File drop zone"
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if isDragging}
    <div class="absolute inset-0 z-10 bg-brand/10 border-2 border-dashed border-brand rounded-r-2 flex items-center justify-center">
      <span class="text-brand font-medium">Drop files here</span>
    </div>
  {/if}

  <section
    class="border-t border-border-1 bg-bg-surface-1 p-4"
    aria-label="Message composer region"
  >
    {#if attachments.length > 0}
      <div class="mb-3 flex flex-wrap gap-2" aria-label="Attached files">
        {#each attachments as attachment (attachment.id)}
          <span
            data-testid="file-attachment"
            class="inline-flex items-center gap-2 rounded-full border px-3 py-1 text-sm {attachment.uploadError ? 'border-red-200 bg-red-50 text-red-700' : 'border-border-1 bg-bg-app text-gray-700'}"
          >
            {#if attachment.uploading}
              <span data-testid="upload-progress" class="inline-block h-3 w-3 animate-spin rounded-full border-2 border-border-2 border-t-indigo-600"></span>
            {:else if attachment.uploadError}
              <span class="text-red-500" aria-label="Upload failed">!</span>
            {/if}
            <span class="truncate max-w-[12rem]">{attachment.name}</span>
            {#if attachment.size && !attachment.uploading}
              <span class="text-xs text-gray-400">({Math.round(attachment.size / 1024)}KB)</span>
            {/if}
            <button
              type="button"
              class="text-text-3 hover:text-gray-900 disabled:opacity-40"
              aria-label={`Remove ${attachment.name}`}
              disabled={attachment.uploading}
              on:click={() => removeAttachment(attachment.id)}
            >
              x
            </button>
          </span>
        {/each}
      </div>
    {/if}

    <EmojiPicker
      show={emojiOpen}
      anchorEl={emojiButtonEl}
      onSelect={(emoji) => { insertAtCursor(emoji); emojiOpen = false }}
      onClose={() => { emojiOpen = false }}
    />

    {#if mentionMatch && matchingMembers.length > 0}
      <div class="mb-3 rounded-lg border border-border-1 bg-bg-app p-3 text-sm text-gray-700" role="listbox" aria-label="Channel Members">
        <p class="font-medium text-gray-900">Channel Members</p>
        {#each matchingMembers as member (member.id ?? member.user_id ?? member.username)}
          <button type="button" class="mt-2 block rounded-md px-2 py-1 text-left hover:bg-bg-surface-1" on:click={() => replaceCurrentToken('@', `@${member.username}`)}>
            {member.displayName ?? member.display_name ?? member.username}
          </button>
        {/each}
      </div>
    {/if}

    <div class="relative rounded-xl border border-border-2 bg-bg-surface-1 shadow-sm focus-within:border-brand focus-within:ring-2 focus-within:ring-brand/10">
      {#if emojiMatch && matchingEmojis.length > 0}
        <div class="absolute bottom-full left-0 mb-1 w-64 max-h-48 overflow-y-auto bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 z-20">
          {#each matchingEmojis as [name, char]}
            <button class="flex items-center gap-3 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard" on:click={() => insertEmoji(name)}>
              <span class="text-lg">{char}</span>
              <span class="text-text-3">:{name}:</span>
            </button>
          {/each}
        </div>
      {/if}

      {#if channelMatch && matchingChannels.length > 0}
        <div class="absolute bottom-full left-0 mb-1 w-64 max-h-48 overflow-y-auto bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 z-20">
          {#each matchingChannels as channel}
            <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard" on:click={() => insertChannel(channel)}>
              <Hash class="w-4 h-4 text-text-3" />
              <span class="truncate">{channel.display_name || channel.name}</span>
            </button>
          {/each}
        </div>
      {/if}

      {#if commandMatch && matchingCommands.length > 0}
        <div class="absolute bottom-full left-0 mb-1 w-64 max-h-48 overflow-y-auto bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 z-20">
          {#each matchingCommands as cmd}
            <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard" on:click={() => executeCommand(cmd.id)}>
              <svelte:component this={cmd.icon} class="w-4 h-4 text-text-3" />
              <span class="truncate">{cmd.label}</span>
            </button>
          {/each}
        </div>
      {/if}

      {#if showPreview}
        <div class="w-full min-h-[80px] px-3 py-2 text-sm text-text-1 bg-bg-surface-1 rounded-t-xl border-0 overflow-y-auto max-h-[320px]">
          {@html previewHtml}
        </div>
      {:else}
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
      {/if}

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

          <button type="button" bind:this={emojiButtonEl} class="rounded-md px-2 py-1 hover:bg-gray-100" aria-label="Insert emoji" on:click={() => (emojiOpen = !emojiOpen)}>
            :)
          </button>

          <button
            class="p-1.5 rounded {showPreview ? 'bg-brand/10 text-brand' : 'text-text-3 hover:text-text-1 hover:bg-bg-surface-2'} transition-standard"
            on:click={() => showPreview = !showPreview}
            title="Preview markdown"
          >
            <Eye class="w-4 h-4" />
          </button>

          {#if callsStore.currentCall?.channelId === channelId}
            <button
              type="button"
              class="flex items-center gap-1 rounded-md px-2 py-1 text-success hover:bg-bg-surface-2 transition-standard"
              on:click={() => callsStore.toggleExpanded()}
            >
              <Phone class="w-4 h-4" />
              <span class="text-xs">Show call</span>
            </button>
          {:else if callsStore.currentCall}
            <button
              type="button"
              class="flex items-center gap-1 rounded-md px-2 py-1 text-success hover:bg-bg-surface-2 transition-standard animate-pulse"
              on:click={() => callsStore.joinCall(callsStore.currentCall!.channelId)}
            >
              <Phone class="w-4 h-4" />
              <span class="text-xs">Join call</span>
            </button>
          {:else}
            <button
              type="button"
              class="flex items-center gap-1 rounded-md px-2 py-1 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard"
              on:click={() => callsStore.startCall(channelId)}
            >
              <Phone class="w-4 h-4" />
              <span class="text-xs">Start call</span>
            </button>
          {/if}
        </div>

        <div class="flex items-center gap-3">
          <button
            data-testid="send-button"
            type="button"
            class="rounded-lg bg-brand px-4 py-2 font-medium text-white shadow-sm hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50"
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

    <div class="flex items-center justify-between px-1 mt-1">
      <span class="text-[11px] text-text-4">
        <kbd class="px-1 py-0.5 bg-bg-surface-2 border border-border-1 rounded text-[10px]">Enter</kbd> to send,
        <kbd class="px-1 py-0.5 bg-bg-surface-2 border border-border-1 rounded text-[10px]">Shift</kbd> +
        <kbd class="px-1 py-0.5 bg-bg-surface-2 border border-border-1 rounded text-[10px]">Enter</kbd> for newline
      </span>
    </div>
  </section>
</div>
