<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useStorage } from '@vueuse/core'
import { Smile, Paperclip, Send, X, File as FileIcon, Phone, ChevronDown, ChevronUp } from 'lucide-vue-next'
import { useToast } from '../../composables/useToast'
import { filesApi, type FileUploadResponse } from '../../api/files'
import FileUploader from '../atomic/FileUploader.vue'
import EmojiPicker from '../atomic/EmojiPicker.vue'
import FormattingToolbar from './FormattingToolbar.vue'
import MarkdownPreview from './MarkdownPreview.vue'
import MentionAutocomplete from './MentionAutocomplete.vue'
import EmojiAutocomplete from './autocomplete/EmojiAutocomplete.vue'
import ChannelAutocomplete from './autocomplete/ChannelAutocomplete.vue'
import CommandAutocomplete from './autocomplete/CommandAutocomplete.vue'
import { useTeamStore } from '../../stores/teams'
import { useCallsStore } from '../../stores/calls'
import { useChannelStore } from '../../stores/channels'
import { usePreferencesStore } from '../../stores/preferences'
import { searchEmojis } from '../../utils/emoji'
import { useCodeFormatting } from '../../composables/useCodeFormatting'
import { useWebSocket } from '../../composables/useWebSocket'

const emit = defineEmits(['send', 'typing', 'stopTyping', 'startAudioCall'])

const toast = useToast()
const teamStore = useTeamStore()
const callsStore = useCallsStore()
const channelStore = useChannelStore()
const preferencesStore = usePreferencesStore()

const { connectionStatus } = useWebSocket()
const isConnected = computed(() => connectionStatus.value === 'connected')

const content = ref('')
const showEmojiPicker = ref(false)
const showPreview = useStorage('composer-show-preview', false)
const showMentionMenu = ref(false)
const showEmojiAutocomplete = ref(false)
const showChannelAutocomplete = ref(false)
const showCommandAutocomplete = ref(false)
const showFormatting = ref(true)
const mentionQuery = ref('')
const emojiQuery = ref('')
const channelQuery = ref('')
const commandQuery = ref('')
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const emojiButtonRef = ref<HTMLElement | null>(null)
const fileUploaderRef = ref<InstanceType<typeof FileUploader> | null>(null)
const emojiAutocompleteRef = ref<InstanceType<typeof EmojiAutocomplete> | null>(null)
const channelAutocompleteRef = ref<InstanceType<typeof ChannelAutocomplete> | null>(null)
const commandAutocompleteRef = ref<InstanceType<typeof CommandAutocomplete> | null>(null)
const isMac = ref(false)
const autocompleteStartPos = ref(0)

const attachedFiles = ref<{
  file: File
  uploading: boolean
  progress: number
  uploaded?: FileUploadResponse
}[]>([])

// Initialize code formatting composable
const { formatInlineCode, formatCodeBlock } = useCodeFormatting(textareaRef)

// Handle format events from toolbar
function handleFormat(type: string) {
  switch (type) {
    case 'code':
      formatInlineCode(content)
      break
    case 'codeblock':
      formatCodeBlock(content)
      break
    // Other format cases can be added later
  }
}

let lastTypingEmit = 0
const TYPING_ACTIVITY_INTERVAL_MS = 2000
const DRAFT_STORAGE_PREFIX = 'rustchat_draft:'
const MAX_TEXTAREA_HEIGHT = 320
const COMMAND_PREFIX = '^k'

const placeholderText = computed(() => {
  const channelName = channelStore.currentChannel?.display_name || channelStore.currentChannel?.name
  return channelName ? `Message #${channelName}` : 'Write a message'
})

const sendOnCtrlEnter = computed(() => preferencesStore.preferences?.send_on_ctrl_enter ?? false)
const formattingAllowed = computed(() => preferencesStore.preferences?.enable_post_formatting !== false)
const showToolbar = computed(() => formattingAllowed.value && showFormatting.value)

const canSend = computed(() => {
  if (!isConnected.value) return false
  const hasContent = content.value.trim().length > 0
  const hasUploadedFiles = attachedFiles.value.some((file) => file.uploaded)
  const hasUploadInProgress = attachedFiles.value.some((file) => file.uploading)
  return (hasContent || hasUploadedFiles) && !hasUploadInProgress
})

const sendDisabledReason = computed(() => {
  if (!isConnected.value) return 'Reconnecting...'
  if (attachedFiles.value.some(f => f.uploading)) return 'Uploading...'
  if (!content.value.trim() && !attachedFiles.value.some(f => f.uploaded)) return 'Type a message'
  return ''
})

const uploadInProgressCount = computed(
  () => attachedFiles.value.filter((attachment) => attachment.uploading).length
)

const sendShortcutLabel = computed(() => {
  if (!sendOnCtrlEnter.value) return 'Enter'
  return isMac.value ? 'Cmd+Enter' : 'Ctrl+Enter'
})

const hasMentionSuggestions = computed(() => {
  if (!showMentionMenu.value) return false
  const query = mentionQuery.value.toLowerCase()
  return teamStore.members.some((member) => member.username.toLowerCase().includes(query))
})

const hasEmojiSuggestions = computed(() => {
  if (!showEmojiAutocomplete.value || !emojiQuery.value) return false
  return searchEmojis(emojiQuery.value, 1).length > 0
})

const hasChannelSuggestions = computed(() => {
  if (!showChannelAutocomplete.value || !channelQuery.value) return false
  const query = channelQuery.value.toLowerCase()
  return channelStore.channels.some((channel) => {
    const channelName = channel.name?.toLowerCase() ?? ''
    const displayName = channel.display_name?.toLowerCase() ?? ''
    return channelName.includes(query) || displayName.includes(query)
  })
})

const commandEntries = ['call start', 'call join', 'call leave', 'call end']

const hasCommandSuggestions = computed(() => {
  if (!showCommandAutocomplete.value) return false
  const query = commandQuery.value.trim().toLowerCase().replace(/^\^k\s*/, '')
  if (!query) return commandEntries.length > 0
  return commandEntries.some((command) => command.toLowerCase().startsWith(query))
})

function getDraftKey(channelId?: string): string | null {
  if (!channelId) return null
  return `${DRAFT_STORAGE_PREFIX}${channelId}`
}

function loadDraft(channelId?: string): string {
  const key = getDraftKey(channelId)
  if (!key) return ''

  try {
    const raw = localStorage.getItem(key)
    if (!raw) return ''
    const parsed = JSON.parse(raw) as { content?: string; timestamp?: number }

    if (!parsed.timestamp || Date.now() - parsed.timestamp > 7 * 24 * 60 * 60 * 1000) {
      localStorage.removeItem(key)
      return ''
    }

    return parsed.content || ''
  } catch {
    return ''
  }
}

function saveDraft() {
  const key = getDraftKey(channelStore.currentChannelId ?? undefined)
  if (!key) return

  if (!content.value.trim()) {
    localStorage.removeItem(key)
    return
  }

  localStorage.setItem(
    key,
    JSON.stringify({
      content: content.value,
      timestamp: Date.now(),
    })
  )
}

function clearDraft() {
  const key = getDraftKey(channelStore.currentChannelId ?? undefined)
  if (!key) return
  localStorage.removeItem(key)
}

function autoResize() {
  const textarea = textareaRef.value
  if (!textarea) return

  textarea.style.height = 'auto'
  const nextHeight = Math.min(textarea.scrollHeight, MAX_TEXTAREA_HEIGHT)
  textarea.style.height = `${nextHeight}px`
  textarea.style.overflowY = textarea.scrollHeight > MAX_TEXTAREA_HEIGHT ? 'auto' : 'hidden'
}

function resetComposer() {
  content.value = ''
  attachedFiles.value = []
  showPreview.value = false
  showMentionMenu.value = false
  showEmojiAutocomplete.value = false
  showChannelAutocomplete.value = false
  showCommandAutocomplete.value = false
  showEmojiPicker.value = false

  nextTick(() => {
    if (textareaRef.value) {
      textareaRef.value.style.height = 'auto'
      textareaRef.value.focus()
    }
  })
}

function resetForChannelChange(channelId?: string) {
  content.value = loadDraft(channelId)
  attachedFiles.value = []
  showPreview.value = false
  showMentionMenu.value = false
  showEmojiAutocomplete.value = false
  showChannelAutocomplete.value = false
  showCommandAutocomplete.value = false
  showEmojiPicker.value = false
  mentionQuery.value = ''
  emojiQuery.value = ''
  channelQuery.value = ''
  commandQuery.value = ''

  nextTick(() => {
    autoResize()
  })
}

function toggleFormatting() {
  if (!formattingAllowed.value) return
  showFormatting.value = !showFormatting.value
}

function handleGlobalKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.altKey && event.key.toLowerCase() === 't') {
    event.preventDefault()
    toggleFormatting()
  }
}

async function handleCommandAction(command: string, args: string[]): Promise<boolean> {
  const channelId = channelStore.currentChannelId
  if (!channelId) return false

  switch (command) {
    case 'call': {
      const subCommand = args[0]

      if (subCommand === 'start' || !subCommand) {
        const existingCall = callsStore.currentChannelCall(channelId)
        if (existingCall) {
          toast.error('A call is already in progress', 'Join the existing call instead')
          return true
        }

        try {
          await callsStore.startCall(channelId)
          toast.success('Call started', 'You are now in a call')
        } catch (error: any) {
          toast.error('Failed to start call', error.message || 'Unknown error')
        }
        return true
      }

      if (subCommand === 'join') {
        try {
          await callsStore.joinCall(channelId)
        } catch (error: any) {
          toast.error('Failed to join call', error.message || 'Unknown error')
        }
        return true
      }

      if (subCommand === 'leave') {
        if (callsStore.isInCall) {
          await callsStore.leaveCall()
        } else {
          toast.error('Not in a call', 'You are not currently in a call')
        }
        return true
      }

      if (subCommand === 'end') {
        if (callsStore.isInCall) {
          await callsStore.endCall()
        } else {
          toast.error('Not in a call', 'You are not currently in a call')
        }
        return true
      }

      return false
    }
    default:
      return false
  }
}

async function handleSend() {
  if (!canSend.value) return

  const trimmedText = content.value.trim()
  const commandMatch = trimmedText.match(/^\^k\s*(.*)$/i)
  if (commandMatch) {
    const commandPayload = (commandMatch[1] ?? '').trim()
    if (!commandPayload) {
      toast.error('No command selected', 'Use Ctrl/Cmd+K or type ^k and choose a command')
      return
    }

    const parts = commandPayload.split(/\s+/)
    const command = (parts[0] ?? '').toLowerCase()
    const args = parts.slice(1)
    const handled = await handleCommandAction(command, args)
    if (handled) {
      clearDraft()
      resetComposer()
      return
    }

    toast.error('Unknown command', `The command "${commandPayload}" is not available`)
    return
  }

  const fileIds = attachedFiles.value
    .filter((attachment) => !attachment.uploading && attachment.uploaded)
    .map((attachment) => attachment.uploaded!.id)

  emit('send', {
    content: content.value,
    file_ids: fileIds,
  })

  clearDraft()
  resetComposer()
}

function handleKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === 'k') {
    event.preventDefault()
    openCommandMenu()
    return
  }

  if (hasCommandSuggestions.value) {
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      commandAutocompleteRef.value?.selectPrevious()
      return
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      commandAutocompleteRef.value?.selectNext()
      return
    }
    if (event.key === 'Enter' || event.key === 'Tab') {
      event.preventDefault()
      commandAutocompleteRef.value?.selectCurrent()
      return
    }
  }

  if (hasEmojiSuggestions.value) {
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      emojiAutocompleteRef.value?.selectPrevious()
      return
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      emojiAutocompleteRef.value?.selectNext()
      return
    }
    if (event.key === 'Enter' || event.key === 'Tab') {
      event.preventDefault()
      emojiAutocompleteRef.value?.selectCurrent()
      return
    }
  }

  if (hasChannelSuggestions.value) {
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      channelAutocompleteRef.value?.selectPrevious()
      return
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      channelAutocompleteRef.value?.selectNext()
      return
    }
    if (event.key === 'Enter' || event.key === 'Tab') {
      event.preventDefault()
      channelAutocompleteRef.value?.selectCurrent()
      return
    }
  }

  if (showMentionMenu.value && hasMentionSuggestions.value) {
    if (event.key === 'ArrowUp' || event.key === 'ArrowDown' || event.key === 'Enter' || event.key === 'Tab') {
      event.preventDefault()
      return
    }
  }

  if (event.key === 'Escape') {
    showMentionMenu.value = false
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
    showCommandAutocomplete.value = false
    showEmojiPicker.value = false
    event.preventDefault()
    return
  }

  if ((event.ctrlKey || event.metaKey) && !event.altKey) {
    if (event.key.toLowerCase() === 'b') {
      event.preventDefault()
      applyFormat('bold')
      return
    }

    if (event.key.toLowerCase() === 'i') {
      event.preventDefault()
      applyFormat('italic')
      return
    }

    if (event.shiftKey && event.key.toLowerCase() === 'x') {
      event.preventDefault()
      applyFormat('strike')
      return
    }
  }

  if ((event.ctrlKey || event.metaKey) && event.shiftKey) {
    if (event.key === '7') {
      event.preventDefault()
      applyFormat('numbered')
      return
    }
    if (event.key === '8') {
      event.preventDefault()
      applyFormat('bullet')
      return
    }
  }

  if (event.key !== 'Enter' || event.shiftKey) return

  if (sendOnCtrlEnter.value) {
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault()
      void handleSend()
    }
    return
  }

  if (!event.ctrlKey && !event.metaKey) {
    event.preventDefault()
    void handleSend()
  }
}

function handleInput() {
  autoResize()
  saveDraft()

  const now = Date.now()
  if (content.value.trim().length > 0 && now - lastTypingEmit > TYPING_ACTIVITY_INTERVAL_MS) {
    lastTypingEmit = now
    emit('typing')
  } else if (content.value.trim().length === 0) {
    // Emit stop typing when user clears the input
    emit('stopTyping')
  }

  const textarea = textareaRef.value
  if (!textarea) return

  const cursorPos = textarea.selectionStart
  const textBefore = content.value.substring(0, cursorPos)

  const commandPrefixMatch = !textBefore.includes('\n') ? textBefore.match(/^\^k\s*(.*)$/i) : null
  if (commandPrefixMatch) {
    commandQuery.value = commandPrefixMatch[1] ?? ''
    autocompleteStartPos.value = 0
    showCommandAutocomplete.value = true
    showMentionMenu.value = false
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
    return
  }

  const mentionMatch = textBefore.match(/@([^\s@]*)$/)
  if (mentionMatch) {
    mentionQuery.value = mentionMatch[1] ?? ''
    autocompleteStartPos.value = cursorPos - mentionMatch[0].length
    showMentionMenu.value = true
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
    showCommandAutocomplete.value = false
    if (teamStore.members.length === 0 && teamStore.currentTeamId) {
      teamStore.fetchMembers(teamStore.currentTeamId as string)
    }
    return
  }

  const emojiMatch = textBefore.match(/:([^\s:]*)$/)
  const emojiToken = emojiMatch?.[1] ?? ''
  if (emojiMatch && emojiToken.length > 0) {
    emojiQuery.value = emojiToken
    autocompleteStartPos.value = cursorPos - emojiMatch[0].length
    showEmojiAutocomplete.value = true
    showMentionMenu.value = false
    showChannelAutocomplete.value = false
    showCommandAutocomplete.value = false
    return
  }

  const channelMatch = textBefore.match(/~([^\s~]*)$/)
  if (channelMatch) {
    channelQuery.value = channelMatch[1] ?? ''
    autocompleteStartPos.value = cursorPos - channelMatch[0].length
    showChannelAutocomplete.value = true
    showMentionMenu.value = false
    showEmojiAutocomplete.value = false
    showCommandAutocomplete.value = false
    return
  }

  showMentionMenu.value = false
  showEmojiAutocomplete.value = false
  showChannelAutocomplete.value = false
  showCommandAutocomplete.value = false
}

function handleMentionSelect(username: string) {
  const textarea = textareaRef.value
  if (!textarea) return

  const cursorPos = textarea.selectionStart
  content.value = `${content.value.substring(0, autocompleteStartPos.value)}@${username} ${content.value.substring(cursorPos)}`
  showMentionMenu.value = false
  saveDraft()

  textarea.focus()
  const newPos = autocompleteStartPos.value + username.length + 2
  setTimeout(() => {
    textarea.setSelectionRange(newPos, newPos)
    autoResize()
  }, 0)
}

function handleEmojiAutocompleteSelect(emojiName: string) {
  const textarea = textareaRef.value
  if (!textarea) return
  const cursorPos = textarea.selectionStart
  content.value = `${content.value.substring(0, autocompleteStartPos.value)}:${emojiName}: ${content.value.substring(cursorPos)}`
  showEmojiAutocomplete.value = false
  saveDraft()
  const newPos = autocompleteStartPos.value + emojiName.length + 3
  nextTick(() => {
    textarea.focus()
    textarea.setSelectionRange(newPos, newPos)
    autoResize()
  })
}

function handleChannelAutocompleteSelect(channelName: string) {
  const textarea = textareaRef.value
  if (!textarea) return
  const cursorPos = textarea.selectionStart
  content.value = `${content.value.substring(0, autocompleteStartPos.value)}~${channelName} ${content.value.substring(cursorPos)}`
  showChannelAutocomplete.value = false
  saveDraft()
  const newPos = autocompleteStartPos.value + channelName.length + 2
  nextTick(() => {
    textarea.focus()
    textarea.setSelectionRange(newPos, newPos)
    autoResize()
  })
}

function handleCommandAutocompleteSelect(command: string) {
  const textarea = textareaRef.value
  if (!textarea) return

  const cursorPos = textarea.selectionStart
  const suffix = content.value.substring(cursorPos).replace(/^\s+/, '')
  const prefix = `${COMMAND_PREFIX} ${command} `
  content.value = suffix.length > 0 ? `${prefix}${suffix}` : prefix

  showCommandAutocomplete.value = false
  saveDraft()

  nextTick(() => {
    textarea.focus()
    textarea.setSelectionRange(prefix.length, prefix.length)
    autoResize()
  })
}

function handleTextareaBlur() {
  // Emit stop typing when user leaves the textarea
  emit('stopTyping')
  setTimeout(() => {
    showMentionMenu.value = false
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
    showCommandAutocomplete.value = false
  }, 120)
}

function openCommandMenu() {
  const existing = content.value.trim()
  if (!existing.toLowerCase().startsWith(COMMAND_PREFIX)) {
    content.value = existing ? `${COMMAND_PREFIX} ${existing}` : `${COMMAND_PREFIX} `
  }

  commandQuery.value = content.value.replace(/^\^k\s*/i, '')
  showCommandAutocomplete.value = true
  showMentionMenu.value = false
  showEmojiAutocomplete.value = false
  showChannelAutocomplete.value = false

  saveDraft()
  nextTick(() => {
    if (!textareaRef.value) return
    const pos = content.value.length
    textareaRef.value.focus()
    textareaRef.value.setSelectionRange(pos, pos)
    autoResize()
  })
}

function applyFormat(type: string) {
  const textarea = textareaRef.value
  if (!textarea) return

  const start = textarea.selectionStart
  const end = textarea.selectionEnd
  const selectedText = content.value.substring(start, end)

  let before = ''
  let after = ''
  let prefix = ''

  switch (type) {
    case 'bold':
      before = '**'
      after = '**'
      break
    case 'italic':
      before = '*'
      after = '*'
      break
    case 'strike':
      before = '~~'
      after = '~~'
      break
    case 'heading':
      prefix = '### '
      break
    case 'code':
      before = '`'
      after = '`'
      break
    case 'codeblock':
      before = '```\n'
      after = '\n```'
      break
    case 'link':
      before = '['
      after = '](url)'
      break
    case 'quote':
      prefix = '> '
      break
    case 'bullet':
      prefix = '- '
      break
    case 'numbered':
      prefix = '1. '
      break
  }

  if (prefix) {
    const lineStart = content.value.lastIndexOf('\n', start - 1) + 1
    content.value = content.value.substring(0, lineStart) + prefix + content.value.substring(lineStart)
  } else {
    content.value =
      content.value.substring(0, start) + before + selectedText + after + content.value.substring(end)
    textarea.focus()
    setTimeout(() => {
      textarea.setSelectionRange(start + before.length, end + before.length)
    }, 0)
  }

  saveDraft()
  autoResize()
}

function openFilePicker() {
  fileUploaderRef.value?.openFilePicker()
}

async function handleFiles(files: File[]) {
  for (const file of files) {
    const attachment = {
      file,
      uploading: true,
      progress: 0,
      uploaded: undefined as FileUploadResponse | undefined,
    }
    attachedFiles.value.push(attachment)
    const attachmentIndex = attachedFiles.value.length - 1

    try {
      const response = await filesApi.upload(file, channelStore.currentChannelId || undefined, (progressEvent) => {
        if (progressEvent.total) {
          const currentAttachment = attachedFiles.value[attachmentIndex]
          if (currentAttachment) {
            attachedFiles.value[attachmentIndex] = {
              ...currentAttachment,
              progress: Math.round((progressEvent.loaded * 100) / progressEvent.total)
            }
          }
        }
      })

      const currentAttachment = attachedFiles.value[attachmentIndex]
      if (currentAttachment) {
        attachedFiles.value[attachmentIndex] = {
          ...currentAttachment,
          uploaded: response.data,
          uploading: false
        }
      }
      toast.success('File uploaded', file.name)
    } catch (error: any) {
      toast.error('Upload failed', error.message || 'Unknown error')
      attachedFiles.value = attachedFiles.value.filter((fileItem) => fileItem !== attachment)
    }
  }
}

function removeAttachment(index: number) {
  if (attachedFiles.value[index]?.uploading) return
  attachedFiles.value.splice(index, 1)
}

function insertEmoji(emoji: string) {
  const textarea = textareaRef.value
  if (!textarea) {
    content.value += emoji
    saveDraft()
    showEmojiPicker.value = false
    return
  }

  const start = textarea.selectionStart
  const end = textarea.selectionEnd
  content.value = `${content.value.substring(0, start)}${emoji}${content.value.substring(end)}`
  saveDraft()
  showEmojiPicker.value = false

  nextTick(() => {
    textarea.focus()
    const newCursorPos = start + emoji.length
    textarea.setSelectionRange(newCursorPos, newCursorPos)
    autoResize()
  })
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

watch(
  () => channelStore.currentChannelId,
  (channelId) => {
    resetForChannelChange(channelId || undefined)
  },
  { immediate: true }
)

watch(
  () => formattingAllowed.value,
  (allowed) => {
    if (!allowed) {
      showFormatting.value = false
      showPreview.value = false
    }
  },
  { immediate: true }
)

onMounted(() => {
  isMac.value = navigator.platform.toUpperCase().includes('MAC')
  window.addEventListener('keydown', handleGlobalKeydown)

  if (!preferencesStore.preferences) {
    void preferencesStore.fetchPreferences()
  }

  if (textareaRef.value) {
    autoResize()
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <FileUploader
    ref="fileUploaderRef"
    class="shrink-0 z-[80] border-t border-border-1 bg-bg-surface-1 p-3"
    @files-selected="handleFiles"
  >
    <div class="relative overflow-visible rounded-r-2 border border-border-1 bg-bg-surface-1 transition-standard focus-within:border-brand/60 focus-within:ring-2 focus-within:ring-brand/10">
      <!-- Formatting Toolbar -->
      <FormattingToolbar
        v-if="showToolbar"
        :showPreview="showPreview"
        @format="handleFormat"
        @togglePreview="showPreview = !showPreview"
      />

      <!-- Attached Files -->
      <div v-if="attachedFiles.length > 0" class="flex flex-wrap gap-2 border-b border-border-1 bg-bg-surface-2/30 px-3 py-2">
        <div
          v-for="(attachment, index) in attachedFiles"
          :key="index"
          class="relative flex min-w-[200px] max-w-[300px] items-center gap-2 rounded-r-1 border border-border-1 bg-bg-surface-1 px-3 py-2"
        >
          <FileIcon class="h-4 w-4 shrink-0 text-text-3" />
          <div class="min-w-0 flex-1">
            <p class="truncate text-xs font-medium text-text-1">{{ attachment.file.name }}</p>
            <p class="text-[10px] text-text-3">{{ formatFileSize(attachment.file.size) }}</p>
          </div>
          <button
            class="rounded p-1 text-text-3 transition-standard hover:bg-danger/10 hover:text-danger focus-ring disabled:cursor-not-allowed disabled:opacity-40"
            aria-label="Remove attachment"
            :disabled="attachment.uploading"
            @click="removeAttachment(index)"
          >
            <X class="h-3.5 w-3.5" />
          </button>

          <div v-if="attachment.uploading" class="absolute inset-x-0 bottom-0 h-0.5 bg-border-1">
            <div class="h-full bg-brand transition-standard" :style="{ width: `${attachment.progress}%` }"></div>
          </div>
        </div>
      </div>

      <!-- Editor Container with Side-by-Side Layout -->
      <div class="flex gap-3">
        <!-- Textarea Container -->
        <div :class="showPreview ? 'flex-1' : 'w-full'">
          <!-- Autocomplete Menus -->
          <div class="relative">
            <MentionAutocomplete
              :show="showMentionMenu"
              :query="mentionQuery"
              @select="handleMentionSelect"
              @close="showMentionMenu = false"
            />
            <CommandAutocomplete
              ref="commandAutocompleteRef"
              :show="showCommandAutocomplete"
              :query="commandQuery"
              @select="handleCommandAutocompleteSelect"
              @close="showCommandAutocomplete = false"
            />
            <EmojiAutocomplete
              ref="emojiAutocompleteRef"
              :show="showEmojiAutocomplete"
              :query="emojiQuery"
              @select="handleEmojiAutocompleteSelect"
              @close="showEmojiAutocomplete = false"
            />
            <ChannelAutocomplete
              ref="channelAutocompleteRef"
              :show="showChannelAutocomplete"
              :query="channelQuery"
              @select="handleChannelAutocompleteSelect"
              @close="showChannelAutocomplete = false"
            />

            <!-- Textarea -->
            <textarea
              ref="textareaRef"
              v-model="content"
              rows="1"
              class="max-h-80 min-h-[40px] w-full resize-none border-0 bg-transparent px-3 py-2 text-xs leading-relaxed text-text-1 placeholder:text-text-3 focus:ring-0"
              :placeholder="placeholderText"
              aria-label="Message composer"
              @keydown="handleKeydown"
              @input="handleInput"
              @blur="handleTextareaBlur"
            ></textarea>
          </div>
        </div>

        <!-- Preview Panel (Side-by-Side) -->
        <div v-if="showPreview" class="flex-1 border-l border-border-1 pl-3">
          <MarkdownPreview :content="content" />
        </div>
      </div>

      <!-- Bottom Toolbar -->
      <div class="flex flex-wrap items-center gap-x-2 gap-y-2 border-t border-border-1/60 px-2 py-2">
        <!-- Left Actions -->
        <div class="flex items-center gap-1">
          <button
            class="flex h-9 w-9 items-center justify-center rounded-r-1 transition-standard hover:bg-brand/10 hover:text-brand focus-ring"
            title="Attach file"
            aria-label="Attach file"
            @click="openFilePicker"
          >
            <Paperclip class="h-3.5 w-3.5" />
          </button>

          <div class="relative">
            <button
              ref="emojiButtonRef"
              class="flex h-9 w-9 items-center justify-center rounded-r-1 transition-standard hover:bg-warning/10 hover:text-warning focus-ring"
              title="Insert emoji"
              aria-label="Insert emoji"
              @click="showEmojiPicker = !showEmojiPicker"
            >
              <Smile class="h-3.5 w-3.5" />
            </button>
            <EmojiPicker
              :show="showEmojiPicker"
              :anchor-el="emojiButtonRef"
              @select="insertEmoji"
              @close="showEmojiPicker = false"
            />
          </div>

          <button
            class="hidden sm:inline-flex min-h-11 items-center gap-1 rounded-r-1 px-3 transition-standard focus-ring"
            :class="showToolbar ? 'bg-brand/10 text-brand' : 'hover:bg-brand/10 hover:text-brand'"
            :disabled="!formattingAllowed"
            :title="showToolbar ? 'Hide formatting (Ctrl+Alt+T)' : 'Show formatting (Ctrl+Alt+T)'"
            aria-label="Toggle formatting toolbar"
            @click="toggleFormatting"
          >
            <span class="text-sm font-medium">Aa</span>
            <ChevronUp v-if="showToolbar" class="h-3 w-3" />
            <ChevronDown v-else class="h-3 w-3" />
          </button>

          <button
            v-if="!callsStore.isInCall"
            class="flex h-9 w-9 items-center justify-center rounded-r-1 transition-standard hover:bg-success/10 hover:text-success focus-ring"
            title="Start audio call"
            aria-label="Start audio call"
            @click="$emit('startAudioCall')"
          >
            <Phone class="h-3.5 w-3.5" />
          </button>
          <button
            v-else
            class="flex h-9 w-9 items-center justify-center rounded-r-1 bg-success/10 text-success transition-standard focus-ring"
            title="Show active call"
            aria-label="Show active call"
            @click="callsStore.toggleExpanded()"
          >
            <Phone class="h-3.5 w-3.5" />
          </button>
        </div>

        <!-- Right: Shortcuts + Send -->
        <div class="ml-auto flex items-center gap-2 sm:gap-3">
          <!-- Keyboard Shortcuts Hint -->
          <div class="hidden xl:flex items-center gap-2 text-[11px] text-text-3">
            <span class="flex items-center gap-1">
              <kbd class="px-1.5 py-0.5 bg-bg-surface-2 rounded text-[10px]">{{ sendShortcutLabel }}</kbd>
              <span>to send</span>
            </span>
            <span class="text-border-2">|</span>
            <span class="flex items-center gap-1">
              <kbd class="px-1.5 py-0.5 bg-bg-surface-2 rounded text-[10px]">Shift+Enter</kbd>
              <span>newline</span>
            </span>
          </div>

          <!-- Mobile Command Hint -->
          <div class="hidden md:flex xl:hidden items-center text-[11px] text-text-3">
            <kbd class="px-1 py-0.5 bg-bg-surface-2 rounded">^k</kbd>
            <span class="ml-1">command</span>
          </div>

          <!-- Send Button -->
          <button
            class="flex h-9 min-w-9 items-center justify-center gap-1.5 rounded-r-1 bg-brand px-3 text-brand-foreground shadow-1 transition-standard hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50 sm:px-4"
            :disabled="!canSend"
            :title="sendDisabledReason"
            aria-label="Send message"
            @click="handleSend"
          >
            <Send class="h-4 w-4" />
            <span class="hidden sm:inline text-xs font-medium">Send</span>
          </button>
        </div>
      </div>

      <!-- Upload Progress -->
      <div v-if="uploadInProgressCount > 0" class="border-t border-border-1/50 px-3 py-1 text-[11px] text-text-3">
        Uploading {{ uploadInProgressCount }} file{{ uploadInProgressCount > 1 ? 's' : '' }}...
      </div>
    </div>
  </FileUploader>
</template>
