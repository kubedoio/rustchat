import { log } from '@/utils/log'
import { computed, watchEffect } from 'vue'
import { usePresenceStore } from '../features/presence'
import type { PresenceStatus } from '../core/entities/User'
import { usersApi } from '../api/users'

// Track pending fetches globally to prevent duplicate requests
const pendingFetches = new Set<string>()

/**
 * Composable for managing user presence status
 *
 * Usage:
 * - For single user: usePresence(userId)
 * - For batch fetching: usePresence().fetchMissing(userIds)
 */
export function usePresence(userId?: string) {
  const store = usePresenceStore()

  const presence = computed(() => {
    if (!userId) return 'offline'
    const user = store.getUserPresence(userId).value
    return user?.presence?.toLowerCase() || 'offline'
  })

  const isLoading = computed(() => {
    if (!userId) return false
    return pendingFetches.has(userId)
  })

  // Auto-fetch if missing (for single user usage)
  if (userId) {
    watchEffect(() => {
      if (userId && !store.presenceMap.has(userId) && !pendingFetches.has(userId)) {
        fetchMissingStatuses([userId])
      }
    })
  }

  return {
    presence,
    isLoading,
    fetchMissing: fetchMissingStatuses,
  }
}

/**
 * Batch fetch statuses for user IDs not in the store
 */
export async function fetchMissingStatuses(userIds: string[]) {
  const store = usePresenceStore()

  // Filter out IDs we already have or are fetching
  const missingIds = userIds.filter(id => {
    if (id === store.self?.userId) return false
    if (store.presenceMap.has(id)) return false
    if (pendingFetches.has(id)) return false
    return true
  })

  if (missingIds.length === 0) return

  missingIds.forEach(id => pendingFetches.add(id))

  try {
    const { data: statuses } = await usersApi.getStatusesByIds(missingIds)

    // Update store with fetched statuses
    for (const status of statuses) {
      store.setUserPresence(
        status.user_id,
        '',
        (status.status?.toLowerCase() as PresenceStatus) || 'offline'
      )
    }

    // Mark missing users as offline to prevent re-fetching
    for (const id of missingIds) {
      if (!store.presenceMap.has(id)) {
        store.setUserPresence(id, '', 'offline' as PresenceStatus)
      }
    }
  } catch (e) {
    log.error('Failed to fetch statuses:', e)
  } finally {
    missingIds.forEach(id => pendingFetches.delete(id))
  }
}

/**
 * Extract unique user IDs from messages/entities
 */
export function extractUserIds(items: Array<{ userId?: string }>): string[] {
  return [...new Set(items.map(i => i.userId).filter(Boolean))] as string[]
}
