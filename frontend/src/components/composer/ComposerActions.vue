<script setup lang="ts">
import { ref } from 'vue'
import { Smile, Paperclip, Send, Phone, ChevronDown, ChevronUp } from 'lucide-vue-next'
import EmojiPicker from '../atomic/EmojiPicker.vue'

const props = defineProps<{
  canSend: boolean
  sendDisabledReason: string
  sendShortcutLabel: string
  showToolbar: boolean
  formattingAllowed: boolean
  isInCall: boolean
}>()

const emit = defineEmits<{
  toggleFormatting: []
  attach: []
  insertEmoji: [emoji: string]
  startAudioCall: []
  toggleCall: []
  send: []
}>()

const showEmojiPicker = ref(false)
const emojiButtonRef = ref<HTMLElement | null>(null)

function onInsertEmoji(emoji: string) {
  showEmojiPicker.value = false
  emit('insertEmoji', emoji)
}
</script>

<template>
  <div class="flex flex-wrap items-center gap-x-2 gap-y-2 border-t border-border-1/60 px-2 py-2">
    <!-- Left Actions -->
    <div class="flex items-center gap-1">
      <button
        class="flex h-9 w-9 items-center justify-center rounded-r-1 transition-standard hover:bg-brand/10 hover:text-brand focus-ring"
        title="Attach file"
        aria-label="Attach file"
        @click="emit('attach')"
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
          @select="onInsertEmoji"
          @close="showEmojiPicker = false"
        />
      </div>

      <button
        class="hidden sm:inline-flex min-h-11 items-center gap-1 rounded-r-1 px-3 transition-standard focus-ring"
        :class="showToolbar ? 'bg-brand/10 text-brand' : 'hover:bg-brand/10 hover:text-brand'"
        :disabled="!formattingAllowed"
        :title="showToolbar ? 'Hide formatting (Ctrl+Alt+T)' : 'Show formatting (Ctrl+Alt+T)'"
        aria-label="Toggle formatting toolbar"
        @click="emit('toggleFormatting')"
      >
        <span class="text-sm font-medium">Aa</span>
        <ChevronUp v-if="showToolbar" class="h-3 w-3" />
        <ChevronDown v-else class="h-3 w-3" />
      </button>

      <button
        v-if="!isInCall"
        class="flex h-9 w-9 items-center justify-center rounded-r-1 transition-standard hover:bg-success/10 hover:text-success focus-ring"
        title="Start audio call"
        aria-label="Start audio call"
        @click="emit('startAudioCall')"
      >
        <Phone class="h-3.5 w-3.5" />
      </button>
      <button
        v-else
        class="flex h-9 w-9 items-center justify-center rounded-r-1 bg-success/10 text-success transition-standard focus-ring"
        title="Show active call"
        aria-label="Show active call"
        @click="emit('toggleCall')"
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
        @click="emit('send')"
      >
        <Send class="h-4 w-4" />
        <span class="hidden sm:inline text-xs font-medium">Send</span>
      </button>
    </div>
  </div>
</template>
