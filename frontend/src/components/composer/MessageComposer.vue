<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useStorage } from '@vueuse/core'
import { useToast } from '../../composables/useToast'
import { useCallsStore } from '../../stores/calls'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { usePreferencesStore } from '../../features/preferences/stores/preferencesStore'
import { useWebSocket } from '../../composables/useWebSocket'
import { useComposerDraft } from '../../composables/useComposerDraft'
import { useComposerKeyboard } from '../../composables/useComposerKeyboard'
import { useComposerSend } from '../../composables/useComposerSend'
import { useCodeFormatting } from '../../composables/useCodeFormatting'
import ComposerAutocomplete from './ComposerAutocomplete.vue'
import ComposerToolbar from './ComposerToolbar.vue'
import ComposerFileUpload from './ComposerFileUpload.vue'
import ComposerActions from './ComposerActions.vue'
import MarkdownPreview from './MarkdownPreview.vue'

const emit = defineEmits(['send', 'typing', 'stopTyping', 'startAudioCall'])

const toast = useToast()
const callsStore = useCallsStore()
const channelStore = useChannelStore()
const preferencesStore = usePreferencesStore()

const { connectionStatus } = useWebSocket()
const isConnected = computed(() => connectionStatus.value === 'connected')

const content = ref('')
const showPreview = useStorage('composer-show-preview', false)
const showFormatting = ref(true)
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const isMac = ref(false)

const attachedFiles = ref<{
  file: File
  uploading: boolean
  progress: number
  uploaded?: import('../../api/files').FileUploadResponse
}[]>([])

const { loadDraft, saveDraft, clearDraft } = useComposerDraft()
const { formatInlineCode, formatCodeBlock } = useCodeFormatting(textareaRef)

const composerAutocompleteRef = ref<InstanceType<typeof ComposerAutocomplete> | null>(null)
const composerFileUploadRef = ref<InstanceType<typeof ComposerFileUpload> | null>(null)

let lastTypingEmit = 0
const TYPING_ACTIVITY_INTERVAL_MS = 2000
const MAX_TEXTAREA_HEIGHT = 320

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
  if (attachedFiles.value.some((f) => f.uploading)) return 'Uploading...'
  if (!content.value.trim() && !attachedFiles.value.some((f) => f.uploaded)) return 'Type a message'
  return ''
})

const sendShortcutLabel = computed(() => {
  if (!sendOnCtrlEnter.value) return 'Enter'
  return isMac.value ? 'Cmd+Enter' : 'Ctrl+Enter'
})

const autocompleteShowMentionMenu = computed(() => composerAutocompleteRef.value?.showMentionMenu ?? false)
const autocompleteHasMentionSuggestions = computed(() => composerAutocompleteRef.value?.hasMentionSuggestions ?? false)
const autocompleteShowEmojiAutocomplete = computed(() => composerAutocompleteRef.value?.showEmojiAutocomplete ?? false)
const autocompleteHasEmojiSuggestions = computed(() => composerAutocompleteRef.value?.hasEmojiSuggestions ?? false)
const autocompleteShowChannelAutocomplete = computed(() => composerAutocompleteRef.value?.showChannelAutocomplete ?? false)
const autocompleteHasChannelSuggestions = computed(() => composerAutocompleteRef.value?.hasChannelSuggestions ?? false)
const autocompleteShowCommandAutocomplete = computed(() => composerAutocompleteRef.value?.showCommandAutocomplete ?? false)
const autocompleteHasCommandSuggestions = computed(() => composerAutocompleteRef.value?.hasCommandSuggestions ?? false)

const { handleSend } = useComposerSend({
  content,
  attachedFiles,
  canSend,
  onClearDraft: clearDraft,
  onResetComposer: resetComposer,
  emitSend: (payload) => emit('send', payload),
})

const { handleKeydown, handleGlobalKeydown } = useComposerKeyboard({
  textareaRef,
  content,
  sendOnCtrlEnter,
  showMentionMenu: autocompleteShowMentionMenu,
  hasMentionSuggestions: autocompleteHasMentionSuggestions,
  showEmojiAutocomplete: autocompleteShowEmojiAutocomplete,
  hasEmojiSuggestions: autocompleteHasEmojiSuggestions,
  showChannelAutocomplete: autocompleteShowChannelAutocomplete,
  hasChannelSuggestions: autocompleteHasChannelSuggestions,
  showCommandAutocomplete: autocompleteShowCommandAutocomplete,
  hasCommandSuggestions: autocompleteHasCommandSuggestions,
  autocompleteRef: computed(() => composerAutocompleteRef.value ?? null),
  onSend: handleSend,
  onOpenCommandMenu: openCommandMenu,
  onCloseAllMenus: () => {
    composerAutocompleteRef.value?.reset()
  },
  onToggleFormatting: toggleFormatting,
  onSaveDraft: () => saveDraft(content.value),
  onAutoResize: autoResize,
})

function autoResize() {
  const textarea = textareaRef.value
  if (!textarea) return
  textarea.style.height = 'auto'
  const nextHeight = Math.min(textarea.scrollHeight, MAX_TEXTAREA_HEIGHT)
  textarea.style.height = `${nextHeight}px`
  textarea.style.overflowY = textarea.scrollHeight > MAX_TEXTAREA_HEIGHT ? 'auto' : 'hidden'
}

function toggleFormatting() {
  if (!formattingAllowed.value) return
  showFormatting.value = !showFormatting.value
}

function handleFormat(type: string) {
  switch (type) {
    case 'code':
      formatInlineCode(content)
      break
    case 'codeblock':
      formatCodeBlock(content)
      break
  }
}

function openCommandMenu() {
  const newContent = composerAutocompleteRef.value?.openCommandMenu(content.value)
  if (newContent !== undefined) {
    content.value = newContent
  }
  saveDraft(content.value)
  nextTick(() => {
    if (!textareaRef.value) return
    const pos = content.value.length
    textareaRef.value.focus()
    textareaRef.value.setSelectionRange(pos, pos)
    autoResize()
  })
}

function handleInput() {
  autoResize()
  saveDraft(content.value)
  const now = Date.now()
  if (content.value.trim().length > 0 && now - lastTypingEmit > TYPING_ACTIVITY_INTERVAL_MS) {
    lastTypingEmit = now
    emit('typing')
  } else if (content.value.trim().length === 0) {
    emit('stopTyping')
  }
  const textarea = textareaRef.value
  if (!textarea) return
  composerAutocompleteRef.value?.onInput(content.value, textarea.selectionStart)
}

function handleTextareaBlur() {
  emit('stopTyping')
  setTimeout(() => {
    composerAutocompleteRef.value?.reset()
  }, 120)
}

function onAutocompleteFocusCursor(pos: number) {
  nextTick(() => {
    textareaRef.value?.focus()
    textareaRef.value?.setSelectionRange(pos, pos)
    autoResize()
  })
}

function insertEmoji(emoji: string) {
  const textarea = textareaRef.value
  if (!textarea) {
    content.value += emoji
    saveDraft(content.value)
    return
  }
  const start = textarea.selectionStart
  const end = textarea.selectionEnd
  content.value = `${content.value.substring(0, start)}${emoji}${content.value.substring(end)}`
  saveDraft(content.value)
  nextTick(() => {
    textarea.focus()
    const newCursorPos = start + emoji.length
    textarea.setSelectionRange(newCursorPos, newCursorPos)
    autoResize()
  })
}

function resetComposer() {
  content.value = ''
  attachedFiles.value = []
  showPreview.value = false
  composerAutocompleteRef.value?.reset()
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
  composerAutocompleteRef.value?.reset()
  nextTick(() => {
    autoResize()
  })
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
  <ComposerFileUpload
    ref="composerFileUploadRef"
    v-model="attachedFiles"
    :channel-id="channelStore.currentChannelId || undefined"
    @upload-success="toast.success('File uploaded', $event)"
    @upload-error="toast.error('Upload failed', $event)"
  >
    <template #toolbar>
      <ComposerToolbar
        v-if="showToolbar"
        :show-preview="showPreview"
        @format="handleFormat"
        @toggle-preview="showPreview = !showPreview"
      />
    </template>

    <!-- Editor Container with Side-by-Side Layout -->
    <div class="flex gap-3">
      <!-- Textarea Container -->
      <div :class="showPreview ? 'flex-1' : 'w-full'">
        <div class="relative">
          <ComposerAutocomplete
            ref="composerAutocompleteRef"
            :content="content"
            @update:content="content = $event"
            @focus-cursor="onAutocompleteFocusCursor"
            @save-draft="saveDraft(content)"
            @auto-resize="autoResize"
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

    <template #bottom>
      <ComposerActions
        :can-send="canSend"
        :send-disabled-reason="sendDisabledReason"
        :send-shortcut-label="sendShortcutLabel"
        :show-toolbar="showToolbar"
        :formatting-allowed="formattingAllowed"
        :is-in-call="callsStore.isInCall"
        @toggle-formatting="toggleFormatting"
        @attach="composerFileUploadRef?.openFilePicker()"
        @insert-emoji="insertEmoji"
        @start-audio-call="$emit('startAudioCall')"
        @toggle-call="callsStore.toggleExpanded()"
        @send="handleSend"
      />
    </template>
  </ComposerFileUpload>
</template>
