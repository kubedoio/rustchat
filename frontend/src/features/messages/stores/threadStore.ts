import { log } from '@/utils/log'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Post } from '../../../api/posts'
import { threadService } from '../services/threadService'

export interface ThreadState {
  isOpen: boolean
  parentPostId: string | null
  parentPost: Post | null
  replies: Post[]
  hasMore: boolean
  cursor: string | null
  isLoading: boolean
  isSending: boolean
  draft: string
}

export const useThreadStore = defineStore('thread', () => {
  // State
  const isOpen = ref(false)
  const parentPostId = ref<string | null>(null)
  const parentPost = ref<Post | null>(null)
  const replies = ref<Post[]>([])
  const hasMore = ref(false)
  const cursor = ref<string | null>(null)
  const isLoading = ref(false)
  const isSending = ref(false)
  const draft = ref('')

  // Getters
  const replyCount = computed(() => replies.value.length)

  // Actions
  async function openThread(postId: string): Promise<void> {
    if (parentPostId.value === postId && isOpen.value) return

    isOpen.value = true
    parentPostId.value = postId
    isLoading.value = true
    replies.value = []

    try {
      const response = await threadService.getThread(postId, { limit: 50 })
      parentPost.value = response.posts[postId] || null
      replies.value = response.order
        .filter(id => id !== postId)
        .map(id => response.posts[id])
        .filter((post): post is Post => post !== undefined)
      cursor.value = response.next_cursor || null
      hasMore.value = !!response.next_cursor

      const savedDraft = localStorage.getItem(`thread_draft_${postId}`)
      if (savedDraft) draft.value = savedDraft
    } catch (error) {
      log.error('Failed to load thread:', error)
      closeThread()
      throw error
    } finally {
      isLoading.value = false
    }
  }

  function closeThread(): void {
    if (parentPostId.value && draft.value.trim()) {
      localStorage.setItem(`thread_draft_${parentPostId.value}`, draft.value)
    }
    isOpen.value = false
    parentPostId.value = null
    parentPost.value = null
    replies.value = []
    hasMore.value = false
    cursor.value = null
    draft.value = ''
  }

  async function loadMoreReplies(): Promise<void> {
    if (!parentPostId.value || !cursor.value || isLoading.value) return
    isLoading.value = true
    try {
      const response = await threadService.getThread(parentPostId.value, {
        cursor: cursor.value,
        limit: 50,
      })
      const newReplies = response.order
        .filter(id => id !== parentPostId.value)
        .map(id => response.posts[id])
        .filter((post): post is Post => post !== undefined)
      replies.value.push(...newReplies)
      cursor.value = response.next_cursor || null
      hasMore.value = !!response.next_cursor
    } catch (error) {
      log.error('Failed to load more replies:', error)
    } finally {
      isLoading.value = false
    }
  }

  async function sendReply(message: string, fileIds: string[] = []): Promise<void> {
    if (!parentPostId.value || !parentPost.value || isSending.value) return
    isSending.value = true
    try {
      const reply = await threadService.sendReply(
        parentPost.value.channel_id,
        parentPostId.value,
        message,
        fileIds
      )
      replies.value.push(reply)
      draft.value = ''
      localStorage.removeItem(`thread_draft_${parentPostId.value}`)
      if (parentPost.value) {
        parentPost.value.reply_count = (parentPost.value.reply_count || 0) + 1
      }
    } catch (error) {
      log.error('Failed to send reply:', error)
      throw error
    } finally {
      isSending.value = false
    }
  }

  function setDraft(value: string): void {
    draft.value = value
    if (parentPostId.value) {
      localStorage.setItem(`thread_draft_${parentPostId.value}`, value)
    }
  }

  // WebSocket event handlers
  function onNewReply(post: Post): void {
    // Only add if not already present (avoid duplicates)
    const exists = replies.value.some(r => r.id === post.id)
    if (!exists) {
      replies.value.push(post)
      // Update reply count on parent post
      if (parentPost.value) {
        parentPost.value.reply_count = (parentPost.value.reply_count || 0) + 1
      }
    }
  }

  function onPostDeleted(postId: string): void {
    const index = replies.value.findIndex(r => r.id === postId)
    if (index !== -1) {
      replies.value.splice(index, 1)
      // Update reply count on parent post
      if (parentPost.value) {
        parentPost.value.reply_count = Math.max(0, (parentPost.value.reply_count || 0) - 1)
      }
    }
    // If the parent post itself was deleted, close the thread
    if (parentPostId.value === postId) {
      closeThread()
    }
  }

  return {
    isOpen,
    parentPostId,
    parentPost,
    replies,
    hasMore,
    cursor,
    isLoading,
    isSending,
    draft,
    replyCount,
    openThread,
    closeThread,
    loadMoreReplies,
    sendReply,
    setDraft,
    onNewReply,
    onPostDeleted,
  }
})
