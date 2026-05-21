/**
 * Draft Persistence Hook
 * Persists composer drafts per channel/DM to localStorage
 * Clear drafts on successful send
 */

import { log } from '@/utils/log'
import { ref, onMounted } from 'vue'

const STORAGE_KEY_PREFIX = 'rustchat_draft:'
const MAX_DRAFT_AGE_MS = 7 * 24 * 60 * 60 * 1000 // 7 days

export interface DraftData {
  content: string
  timestamp: number
  attachments?: string[] // file_ids
}

export function useDraft(channelId: string) {
  const draft = ref<DraftData | null>(null)
  const hasDraft = ref(false)
  const isRestored = ref(false)

  const storageKey = `${STORAGE_KEY_PREFIX}${channelId}`

  /**
   * Load draft from localStorage
   */
  function loadDraft(): DraftData | null {
    try {
      const stored = localStorage.getItem(storageKey)
      if (!stored) return null

      const data: DraftData = JSON.parse(stored)

      // Check if draft is too old
      if (Date.now() - data.timestamp > MAX_DRAFT_AGE_MS) {
        localStorage.removeItem(storageKey)
        return null
      }

      return data
    } catch (e) {
      log.error('Failed to load draft:', e)
      return null
    }
  }

  /**
   * Save draft to localStorage
   */
  function saveDraft(content: string, attachments?: string[]) {
    try {
      if (!content.trim() && (!attachments || attachments.length === 0)) {
        // Clear if empty
        clearDraft()
        return
      }

      const data: DraftData = {
        content,
        timestamp: Date.now(),
        attachments,
      }

      localStorage.setItem(storageKey, JSON.stringify(data))
      draft.value = data
      hasDraft.value = true
    } catch (e) {
      log.error('Failed to save draft:', e)
    }
  }

  /**
   * Clear draft from localStorage
   */
  function clearDraft() {
    try {
      localStorage.removeItem(storageKey)
      draft.value = null
      hasDraft.value = false
    } catch (e) {
      log.error('Failed to clear draft:', e)
    }
  }

  /**
   * Update draft content without saving to storage
   * (for reactive updates during typing)
   */
  function updateDraftContent(content: string) {
    if (draft.value) {
      draft.value.content = content
    } else {
      draft.value = {
        content,
        timestamp: Date.now(),
        attachments: [],
      }
    }
    hasDraft.value = content.trim().length > 0
  }

  /**
   * Get all draft keys for cleanup
   */
  function getAllDraftKeys(): string[] {
    const keys: string[] = []
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)
      if (key?.startsWith(STORAGE_KEY_PREFIX)) {
        keys.push(key)
      }
    }
    return keys
  }

  /**
   * Cleanup old drafts
   */
  function cleanupOldDrafts() {
    const keys = getAllDraftKeys()
    const now = Date.now()

    for (const key of keys) {
      try {
        const stored = localStorage.getItem(key)
        if (stored) {
          const data: DraftData = JSON.parse(stored)
          if (now - data.timestamp > MAX_DRAFT_AGE_MS) {
            localStorage.removeItem(key)
          }
        }
      } catch (e) {
        // Remove invalid entries
        localStorage.removeItem(key)
      }
    }
  }

  // Load draft on mount
  onMounted(() => {
    const loaded = loadDraft()
    if (loaded) {
      draft.value = loaded
      hasDraft.value = true
      isRestored.value = true
    }

    // Cleanup old drafts periodically
    cleanupOldDrafts()
  })

  return {
    draft,
    hasDraft,
    isRestored,
    loadDraft,
    saveDraft,
    clearDraft,
    updateDraftContent,
  }
}

/**
 * Hook for multiple drafts management
 * Useful for tracking drafts across channels
 */
export function useDrafts() {
  /**
   * Get count of all drafts
   */
  function getDraftCount(): number {
    let count = 0
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)
      if (key?.startsWith(STORAGE_KEY_PREFIX)) {
        count++
      }
    }
    return count
  }

  /**
   * Get all channel IDs with drafts
   */
  function getChannelsWithDrafts(): string[] {
    const channels: string[] = []
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)
      if (key?.startsWith(STORAGE_KEY_PREFIX)) {
        const channelId = key.substring(STORAGE_KEY_PREFIX.length)
        channels.push(channelId)
      }
    }
    return channels
  }

  /**
   * Clear all drafts
   */
  function clearAllDrafts() {
    const keys = getChannelsWithDrafts()
    for (const key of keys) {
      localStorage.removeItem(`${STORAGE_KEY_PREFIX}${key}`)
    }
  }

  return {
    getDraftCount,
    getChannelsWithDrafts,
    clearAllDrafts,
  }
}
