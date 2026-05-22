<script setup lang="ts">
import { log } from '@/utils/log'
import { ref, computed, nextTick } from 'vue'
import { format, formatDistanceToNow } from 'date-fns'
import {
  Smile,
  MessageSquare,
  MoreHorizontal,
  Pencil,
  Trash2,
  Pin,
  X,
  Check,
  Bookmark,
  ArrowRight,
  Video,
  Phone,
  PhoneOff,
} from 'lucide-vue-next'
import type { Message } from '@/features/messages/stores/messageStore'
import { useMessageStore } from '@/features/messages/stores/messageStore'
import { useAuthStore } from '../../features/auth/stores/authStore'
import { useUnreadStore } from '@/features/unreads/stores/unreadStore'
import { useUIStore } from '../../features/ui/stores/uiStore'
import { useConfigStore } from '../../features/config/stores/configStore'
import { postsApi } from '../../api/posts'
import EmojiPicker from '../atomic/EmojiPicker.vue'
import FilePreview from '../atomic/FilePreview.vue'
import RcAvatar from '../ui/RcAvatar.vue'
import ImageGallery from '../atomic/ImageGallery.vue'
import { getEmojiChar, getPreferredEmojiName, getReactionEmojiKey } from '../../utils/emoji'
import type { FileUploadResponse } from '../../api/files'
import { useMarkdownRenderer } from '../../composables/useMarkdownRenderer'
import { normalizeSafeHttpUrl } from '../../utils/safeUrl'

const props = defineProps<{
  message: Message
}>()

const emit = defineEmits<{
  (e: 'edit', id: string): void
  (e: 'delete', id: string): void
  (e: 'reply', id: string): void
  (e: 'update', id: string, content: string): void
  (e: 'openProfile', userId: string): void
}>()

function openUserProfile() {
  emit('openProfile', props.message.userId)
}

function toggleEmojiPicker() {
  showEmojiPicker.value = !showEmojiPicker.value
  showActions.value = true
}

const authStore = useAuthStore()
const messageStore = useMessageStore()
const unreadStore = useUnreadStore()
const uiStore = useUIStore()
const configStore = useConfigStore()

const showActions = ref(false)
const showMenu = ref(false)
const showEmojiPicker = ref(false)
const deleting = ref(false)
const isEditing = ref(false)
const editContent = ref('')
const editInputRef = ref<HTMLTextAreaElement | null>(null)
const emojiButtonRef = ref<HTMLElement | null>(null)
const saving = ref(false)
const { renderMarkdown } = useMarkdownRenderer()

const isOwnMessage = computed(() => authStore.user?.id === props.message.userId)
const isEdited = computed(() => Boolean(props.message.editedAt))
const canEditMessage = computed(() => {
  if (!isOwnMessage.value) return false

  const limit = Number(configStore.siteConfig.post_edit_time_limit_seconds ?? -1)
  if (Number.isNaN(limit) || limit < 0) return true
  if (limit === 0) return false

  const createdAt = new Date(props.message.timestamp).getTime()
  if (!Number.isFinite(createdAt)) return false
  return Date.now() - createdAt < limit * 1000
})

const isSystemMessage = computed(() => {
  const type = props.message.props?.type
  return type === 'system_join_leave' || type === 'system_purpose' || type === 'system_header'
})

const isVideoCall = computed(() => props.message.props?.type === 'video_call')
const isCallsProtocol = computed(() => props.message.props?.type === 'custom_com.mattermost.calls')

function joinCall() {
  if (!props.message.props) return
  const { meeting_url, mode } = props.message.props
  const safeMeetingUrl = normalizeSafeHttpUrl(meeting_url)
  if (!safeMeetingUrl) return

  if (mode === 'embed_iframe') {
    uiStore.openVideoCall(safeMeetingUrl)
  } else {
    window.open(safeMeetingUrl, '_blank', 'noopener,noreferrer')
  }
}

async function handleSave() {
  try {
    if (props.message.isSaved) {
      await messageStore.unsaveMessage(props.message.id, props.message.channelId)
    } else {
      await messageStore.saveMessage(props.message.id, props.message.channelId)
    }
    showMenu.value = false
  } catch (e) {
    log.error('Failed to toggle save', e)
  }
}

async function handlePin() {
  try {
    if (props.message.isPinned) {
      await messageStore.unpinMessage(props.message.id, props.message.channelId)
    } else {
      await messageStore.pinMessage(props.message.id, props.message.channelId)
    }
    showMenu.value = false
  } catch (e) {
    log.error('Failed to toggle pin', e)
  }
}

async function handleMarkAsUnread() {
  try {
    await unreadStore.markAsUnreadFromPost(props.message.id)
    showMenu.value = false
  } catch (e) {
    log.error('Failed to mark as unread', e)
  }
}

async function handleDelete() {
  if (!confirm('Delete this message?')) return

  deleting.value = true
  try {
    await postsApi.delete(props.message.id)
    emit('delete', props.message.id)
  } catch (e) {
    log.error('Failed to delete message', e)
  } finally {
    deleting.value = false
    showMenu.value = false
  }
}

function startEditing() {
  editContent.value = props.message.content
  isEditing.value = true
  showMenu.value = false
  nextTick(() => {
    editInputRef.value?.focus()
    editInputRef.value?.select()
  })
}

function cancelEditing() {
  isEditing.value = false
  editContent.value = ''
}

async function saveEdit() {
  if (!editContent.value.trim() || editContent.value === props.message.content) {
    cancelEditing()
    return
  }

  saving.value = true
  try {
    const { data: updatedPost } = await postsApi.update(props.message.id, editContent.value)
    messageStore.handleMessageUpdate(updatedPost as unknown as Record<string, unknown>)
    if (!updatedPost?.edited_at && !updatedPost?.edit_at) {
      messageStore.handleMessageUpdate({
        id: props.message.id,
        message: editContent.value,
        edited_at: new Date().toISOString(),
      })
    }
    emit('update', props.message.id, editContent.value)
    isEditing.value = false
  } catch (e) {
    log.error('Failed to update message', e)
  } finally {
    saving.value = false
  }
}

function handleEditKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    cancelEditing()
  } else if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    saveEdit()
  }
}

function handleReply() {
  emit('reply', props.message.id)
}

const formattedContent = computed(() => {
  return renderMarkdown(props.message.content, authStore.user?.username || undefined)
})

const isMentioned = computed(() => {
  const username = authStore.user?.username
  return username && props.message.content.includes(`@${username}`)
})

// Quick reactions
const quickEmojis = ['👍', '❤️', '😄']

// Gallery state
const showGallery = ref(false)
const galleryInitialIndex = ref(0)
const galleryImages = computed(() =>
  (props.message.files || []).filter(f => f.mime_type?.startsWith('image/'))
)

function openGallery(file: FileUploadResponse) {
  const index = galleryImages.value.findIndex(f => f.id === file.id)
  if (index !== -1) {
    galleryInitialIndex.value = index
    showGallery.value = true
  }
}

async function handleEmojiSelect(emoji: string) {
  showEmojiPicker.value = false
  showActions.value = false
  await toggleReaction(emoji)
}

function handleMessageMouseLeave() {
  if (showEmojiPicker.value) {
    showMenu.value = false
    return
  }

  showActions.value = false
  showMenu.value = false
}

function handleEmojiPickerClose() {
  showEmojiPicker.value = false
  showActions.value = false
}

async function toggleReaction(emoji: string) {
  const reactionKey = getReactionEmojiKey(emoji)
  const reaction = props.message.reactions?.find(r => getReactionEmojiKey(r.emoji) === reactionKey)
  const me = authStore.user?.id
  if (!me) return

  const hasReacted = reaction?.users.includes(me)
  const emojiKey = reaction?.apiKey || getPreferredEmojiName(emoji)

  try {
    if (hasReacted) {
      await postsApi.removeReaction(props.message.id, emojiKey)
      messageStore.handleReactionRemoved({
        post_id: props.message.id,
        user_id: me,
        emoji_name: emojiKey,
      })
    } else {
      await postsApi.addReaction(props.message.id, emojiKey)
      messageStore.handleReactionAdded({
        post_id: props.message.id,
        user_id: me,
        emoji_name: emojiKey,
      })
    }
  } catch (e) {
    log.error('Failed to toggle reaction', e)
  }
}
</script>

<template>
  <!-- System Message -->
  <div
    v-if="isSystemMessage"
    class="flex items-center px-3 py-1 hover:bg-bg-app/50 transition-standard"
  >
    <div class="flex items-center text-xs text-text-3 italic w-full">
      <ArrowRight class="w-3.5 h-3.5 mr-2 text-text-3" />
      <span v-html="formattedContent"></span>
      <span class="ml-2 text-[10px] text-text-4">
        {{ format(new Date(message.timestamp), 'h:mm a') }}
      </span>
    </div>
  </div>

  <!-- Video Call Message -->
  <div
    v-else-if="isVideoCall"
    class="flex items-start group px-3 py-2 hover:bg-bg-app/50 transition-standard relative"
  >
    <div class="shrink-0 select-none mr-3 mt-1">
      <RcAvatar
        :user-id="message.userId"
        :src="message.avatarUrl"
        :username="message.username"
        size="md"
        class="w-8 h-8 rounded-r-1"
      />
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-baseline gap-2 mb-1">
        <span class="font-semibold text-sm text-text-1">{{ message.username }}</span>
        <span class="text-[11px] text-text-3">{{
          format(new Date(message.timestamp), 'h:mm a')
        }}</span>
      </div>
      <div
        class="bg-bg-surface-2 border border-border-1 rounded-r-2 p-3 inline-flex flex-col max-w-sm shadow-1"
      >
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-success/10 rounded-full flex items-center justify-center">
            <Video class="w-5 h-5 text-success" />
          </div>
          <div>
            <div class="font-semibold text-sm text-text-1">Video Call</div>
            <div class="text-xs text-text-3">
              {{ message.props?.status === 'ended' ? 'Call ended' : 'Ongoing call' }}
            </div>
          </div>
        </div>
        <button
          v-if="message.props?.status !== 'ended'"
          class="w-full bg-success text-white text-sm font-medium py-2 rounded-r-2 hover:opacity-90 transition-standard flex items-center justify-center gap-2"
          @click="joinCall"
        >
          <Video class="w-4 h-4" />
          Join Call
        </button>
        <div v-else class="text-xs text-text-3 flex items-center gap-1.5 px-1">
          <span>Call ended</span>
          <span>•</span>
          <span>{{ message.props?.duration_text || 'Just now' }}</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Calls Plugin Message -->
  <div
    v-else-if="isCallsProtocol"
    class="flex items-center px-3 py-1 hover:bg-bg-app/50 transition-standard group"
  >
    <div class="flex items-center text-[13px] text-text-3 w-full">
      <div class="w-8 h-8 rounded-full bg-brand/10 flex items-center justify-center mr-3 shrink-0">
        <Phone v-if="!message.props?.end_at" class="w-4 h-4 text-brand" />
        <PhoneOff v-else class="w-4 h-4 text-text-3" />
      </div>
      <div class="flex-1">
        <span class="font-medium text-text-2">{{ message.username }}</span>
        <span class="mx-1">{{ message.props?.end_at ? 'ended the call' : 'started a call' }}</span>
        <span v-if="message.props?.duration" class="text-text-4"
          >({{ Math.floor(message.props.duration / 1000 / 60) }}m
          {{ Math.floor((message.props.duration / 1000) % 60) }}s)</span
        >
      </div>
      <span
        class="ml-2 text-[10px] text-text-4 opacity-0 group-hover:opacity-100 transition-opacity"
      >
        {{ format(new Date(message.timestamp), 'h:mm a') }}
      </span>
    </div>
  </div>

  <!-- Regular Message -->
  <div
    v-else
    class="flex items-start group transition-standard relative px-2 sm:px-3 py-1 hover:bg-bg-app/30"
    :class="[
      uiStore.density === 'compact' ? 'py-0.5' : 'py-1',
      isMentioned ? 'bg-brand/5' : '',
      {
        'opacity-70': message.status === 'sending',
        'bg-danger/5': message.status === 'failed',
      },
    ]"
    @mouseenter="showActions = true"
    @mouseleave="handleMessageMouseLeave"
  >
    <!-- Avatar -->
    <div
      data-testid="message-avatar"
      class="shrink-0 select-none mr-2 sm:mr-3 mt-0.5 cursor-pointer"
      @click="openUserProfile"
    >
      <RcAvatar
        :user-id="message.userId"
        :src="message.avatarUrl"
        :username="message.username"
        size="md"
        class="w-[var(--avatar-size)] h-[var(--avatar-size)] rounded-r-1 hover:shadow-2 transition-standard"
      />
    </div>

    <div class="flex-1 min-w-0">
      <!-- Header -->
      <div class="flex items-baseline gap-1.5 flex-wrap">
        <span
          class="font-semibold text-sm text-text-1 hover:underline cursor-pointer transition-colors hover:text-brand"
          @click="openUserProfile"
        >
          {{ message.username }}
        </span>
        <span class="text-xs text-text-3 hover:underline cursor-pointer">
          {{ format(new Date(message.timestamp), 'h:mm a') }}
        </span>
        <span v-if="isEdited" class="text-[10px] text-text-3">(edited)</span>

        <!-- Status Indicators -->
        <span
          v-if="message.status === 'sending'"
          class="text-[10px] text-text-3 italic animate-pulse"
          >Sending...</span
        >
        <span v-if="message.status === 'failed'" class="text-[10px] text-danger font-medium"
          >Failed</span
        >

        <!-- Pinned/Saved badges -->
        <div v-if="message.isPinned || message.isSaved" class="flex items-center gap-1">
          <span
            v-if="message.isPinned"
            class="bg-bg-surface-2 text-[10px] px-1.5 py-0.5 rounded text-text-3 font-medium flex items-center"
          >
            <Pin class="w-2.5 h-2.5 mr-0.5" />
            Pinned
          </span>
          <span
            v-if="message.isSaved"
            class="bg-warning/10 text-[10px] px-1.5 py-0.5 rounded text-warning font-medium flex items-center"
          >
            <Bookmark class="w-2.5 h-2.5 mr-0.5 fill-current" />
            Saved
          </span>
        </div>
      </div>

      <!-- Body - Normal or Editing -->
      <div v-if="isEditing" class="mt-1 max-w-[95%]">
        <textarea
          ref="editInputRef"
          v-model="editContent"
          rows="2"
          class="w-full px-3 py-2 border border-brand rounded-r-2 bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-brand/20 focus:outline-none text-sm"
          @keydown="handleEditKeydown"
        ></textarea>
        <div class="flex items-center gap-2 mt-1.5">
          <button
            :disabled="saving"
            class="flex items-center gap-1 rounded-r-1 bg-brand px-3 py-1.5 text-xs font-medium text-brand-foreground transition-standard hover:bg-brand-hover disabled:opacity-50"
            @click="saveEdit"
          >
            <Check class="w-3 h-3" />
            <span>{{ saving ? 'Saving...' : 'Save' }}</span>
          </button>
          <button
            :disabled="saving"
            class="px-3 py-1.5 bg-bg-surface-2 text-text-2 text-xs font-medium rounded-r-1 hover:bg-bg-surface-1 transition-standard flex items-center gap-1"
            @click="cancelEditing"
          >
            <X class="w-3 h-3" />
            <span>Cancel</span>
          </button>
          <span class="text-xs text-text-3">Esc to cancel • Enter to save</span>
        </div>
      </div>

      <!-- Message Content -->
      <div v-else class="relative">
        <div
          class="message-content text-text-1 text-sm mt-0.5 whitespace-pre-wrap leading-relaxed max-w-full break-words"
          :class="{ 'bg-brand/5 -mx-2 px-2 py-1 rounded': isMentioned }"
          v-html="formattedContent"
        ></div>
      </div>

      <!-- Files -->
      <div v-if="message.files && message.files.length > 0" class="mt-2 flex flex-wrap gap-2">
        <FilePreview
          v-for="file in message.files"
          :key="file.id"
          :file="file"
          @preview="openGallery"
        />
      </div>

      <!-- Thread Reply Count -->
      <div v-if="message.threadCount && message.threadCount > 0" class="mt-1.5">
        <button
          class="inline-flex items-center gap-2 px-2 py-1 rounded-r-1 hover:bg-brand/5 transition-standard border border-transparent hover:border-brand/20"
          @click.stop="handleReply"
        >
          <MessageSquare class="w-3.5 h-3.5 text-brand" />
          <span class="text-[13px] font-medium text-brand">
            {{ message.threadCount }} {{ message.threadCount === 1 ? 'reply' : 'replies' }}
          </span>
          <span v-if="message.lastReplyAt" class="text-[11px] text-text-3">
            Last reply {{ formatDistanceToNow(new Date(message.lastReplyAt)) }} ago
          </span>
        </button>
      </div>

      <!-- Reactions -->
      <div
        v-if="message.reactions && message.reactions.length > 0 && !isEditing"
        class="flex items-center mt-1.5 gap-1.5 flex-wrap"
      >
        <button
          v-for="reaction in message.reactions"
          :key="reaction.emoji"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border transition-all hover:scale-105"
          :class="
            reaction.users.includes(authStore.user?.id || '')
              ? 'bg-brand/10 border-brand/30 text-brand'
              : 'bg-bg-surface-2 border-border-1 text-text-2 hover:border-border-2'
          "
          @click="toggleReaction(reaction.emoji)"
        >
          <span>{{ getEmojiChar(reaction.emoji) }}</span>
          <span class="font-medium">{{ reaction.count }}</span>
        </button>
      </div>
    </div>

    <!-- Hover Actions -->
    <div
      v-show="showActions && !isEditing"
      class="absolute right-2 sm:right-4 top-0 -translate-y-1/2 flex items-center bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 px-1 py-0.5 z-10"
    >
      <!-- Quick Reactions -->
      <div class="flex items-center border-r border-border-1 pr-1 mr-1">
        <button
          v-for="emoji in quickEmojis"
          :key="emoji"
          class="p-1.5 hover:bg-bg-surface-2 rounded transition-colors text-base leading-none"
          :title="`React with ${emoji}`"
          @click="toggleReaction(emoji)"
        >
          {{ emoji }}
        </button>
      </div>

      <button
        class="p-1.5 hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-colors rounded"
        title="Reply in thread"
        @click="handleReply"
      >
        <MessageSquare class="w-4 h-4" />
      </button>

      <div class="relative">
        <button
          ref="emojiButtonRef"
          class="p-1.5 hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-colors rounded"
          :class="{ 'bg-bg-surface-2 text-brand': showEmojiPicker }"
          title="Add reaction"
          @click.stop="toggleEmojiPicker"
        >
          <Smile class="w-4 h-4" />
        </button>
        <EmojiPicker
          :show="showEmojiPicker"
          :anchor-el="emojiButtonRef"
          @select="handleEmojiSelect"
          @close="handleEmojiPickerClose"
        />
      </div>

      <div class="relative">
        <button
          class="p-1.5 hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-colors rounded"
          title="More actions"
          @click.stop="showMenu = !showMenu"
        >
          <MoreHorizontal class="w-4 h-4" />
        </button>

        <!-- Dropdown Menu -->
        <Transition
          enter-active-class="transition-all duration-200 ease-out"
          enter-from-class="opacity-0 scale-95 -translate-y-1"
          enter-to-class="opacity-100 scale-100 translate-y-0"
          leave-active-class="transition-all duration-150 ease-in"
          leave-from-class="opacity-100 scale-100 translate-y-0"
          leave-to-class="opacity-0 scale-95 -translate-y-1"
        >
          <div
            v-if="showMenu"
            class="absolute right-0 top-full mt-1 w-44 bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2xl py-1 z-20 origin-top-right"
          >
            <button
              v-if="isOwnMessage && canEditMessage"
              class="w-full px-3 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center gap-2 transition-standard"
              @click="startEditing"
            >
              <Pencil class="w-4 h-4" />
              Edit message
            </button>
            <button
              class="w-full px-3 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center gap-2 transition-standard"
              @click="handleSave"
            >
              <Bookmark class="w-4 h-4" :class="{ 'fill-current': message.isSaved }" />
              {{ message.isSaved ? 'Unsave message' : 'Save message' }}
            </button>
            <button
              class="w-full px-3 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center gap-2 transition-standard"
              @click="handlePin"
            >
              <Pin class="w-4 h-4" :class="{ 'fill-current': message.isPinned }" />
              {{ message.isPinned ? 'Unpin from channel' : 'Pin to channel' }}
            </button>
            <button
              class="w-full px-3 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center gap-2 transition-standard"
              @click="handleMarkAsUnread"
            >
              <Check class="w-4 h-4" />
              Mark as unread
            </button>
            <button
              v-if="isOwnMessage"
              :disabled="deleting"
              class="w-full px-3 py-2 text-left text-sm text-danger hover:bg-danger/5 flex items-center gap-2 transition-standard"
              @click="handleDelete"
            >
              <Trash2 class="w-4 h-4" />
              {{ deleting ? 'Deleting...' : 'Delete message' }}
            </button>
          </div>
        </Transition>
      </div>
    </div>

    <!-- Image Gallery -->
    <Teleport to="body">
      <ImageGallery
        v-if="showGallery"
        :images="galleryImages"
        :initial-index="galleryInitialIndex"
        @close="showGallery = false"
      />
    </Teleport>
  </div>
</template>
