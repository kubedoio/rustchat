import { useChannelStore } from '@/features/channels/stores/channelStore'

const DRAFT_STORAGE_PREFIX = 'rustchat_draft:'

export function useComposerDraft() {
  const channelStore = useChannelStore()

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

  function saveDraft(content: string) {
    const key = getDraftKey(channelStore.currentChannelId ?? undefined)
    if (!key) return

    if (!content.trim()) {
      localStorage.removeItem(key)
      return
    }

    localStorage.setItem(
      key,
      JSON.stringify({
        content,
        timestamp: Date.now(),
      })
    )
  }

  function clearDraft() {
    const key = getDraftKey(channelStore.currentChannelId ?? undefined)
    if (!key) return
    localStorage.removeItem(key)
  }

  return { loadDraft, saveDraft, clearDraft }
}
