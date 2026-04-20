import { computed, ref, watchEffect } from 'vue'
import { type User, usersApi } from '../api/users'
import { usePresenceStore } from '../features/presence'
import { normalizePresenceStatus } from '../features/presence/presencePresentation'
import { clearStatusExpiryTimer, parseStatusExpiryMs, scheduleStatusExpiry } from '../features/presence/statusExpiry'
import { useTeamStore } from '../stores/teams'

export interface UserSummary {
  id: string
  username: string
  displayName?: string
  email?: string
  nickname?: string
  firstName?: string
  lastName?: string
  position?: string
  avatarUrl?: string
  presence: 'online' | 'away' | 'dnd' | 'offline'
  statusText?: string
  statusEmoji?: string
  statusExpiresAt?: number
}

interface HydratedUserRecord {
  id: string
  username: string
  display_name?: string | null
  email?: string | null
  nickname?: string | null
  first_name?: string | null
  last_name?: string | null
  position?: string | null
  avatar_url?: string | null
  presence?: string | null
  status_text?: string | null
  status_emoji?: string | null
  status_expires_at?: string | number | null
  fully_hydrated?: boolean
}

interface LiveUserStatusUpdate {
  userId: string
  presence?: string | null
  statusText?: string | null
  statusEmoji?: string | null
  statusExpiresAt?: string | number | null
}

const hydratedUsers = ref<Record<string, HydratedUserRecord>>({})
const pendingUsers = ref<Record<string, boolean>>({})
const userErrors = ref<Record<string, string | undefined>>({})
const expiryTimers = new Map<string, ReturnType<typeof setTimeout>>()

function clearExpiredCustomStatus(userId: string) {
  const existing = hydratedUsers.value[userId]
  if (!existing) {
    return
  }

  hydratedUsers.value = {
    ...hydratedUsers.value,
    [userId]: {
      ...existing,
      status_text: null,
      status_emoji: null,
      status_expires_at: null,
    },
  }
}

function scheduleExpiry(userId: string, expiresAt: unknown) {
  scheduleStatusExpiry(expiryTimers, userId, expiresAt, () => {
    clearExpiredCustomStatus(userId)
  })
}

function upsertHydratedUser(userId: string, partial: Partial<HydratedUserRecord>) {
  const existing = hydratedUsers.value[userId]
  const next: HydratedUserRecord = {
    id: userId,
    username: '',
    ...existing,
    ...partial,
  }

  hydratedUsers.value = {
    ...hydratedUsers.value,
    [userId]: next,
  }

  scheduleExpiry(userId, next.status_expires_at)
}

function mergeUserSummary(userId: string, teamStore = useTeamStore(), presenceStore = usePresenceStore()): UserSummary | null {
  const teamMember = teamStore.members.find((member) => member.user_id === userId)
  const hydrated = hydratedUsers.value[userId]
  const livePresence = presenceStore.getUserPresence(userId).value?.presence
  const presence = normalizePresenceStatus(livePresence || hydrated?.presence || teamMember?.presence)
  const statusExpiresAt = parseStatusExpiryMs(hydrated?.status_expires_at)
  const statusExpired = Boolean(statusExpiresAt && statusExpiresAt <= Date.now())

  if (!teamMember && !hydrated) {
    return null
  }

  return {
    id: userId,
    username: hydrated?.username || teamMember?.username || '',
    displayName: hydrated?.display_name || teamMember?.display_name || undefined,
    email: hydrated?.email || undefined,
    nickname: hydrated?.nickname || undefined,
    firstName: hydrated?.first_name || undefined,
    lastName: hydrated?.last_name || undefined,
    position: hydrated?.position || undefined,
    avatarUrl: hydrated?.avatar_url || teamMember?.avatar_url || undefined,
    presence,
    statusText: statusExpired ? undefined : hydrated?.status_text || undefined,
    statusEmoji: statusExpired ? undefined : hydrated?.status_emoji || undefined,
    statusExpiresAt: statusExpired || statusExpiresAt == null ? undefined : statusExpiresAt,
  }
}

async function ensureUserSummary(userId: string) {
  const existing = hydratedUsers.value[userId]
  if (!userId || pendingUsers.value[userId] || existing?.fully_hydrated) {
    return
  }

  pendingUsers.value = { ...pendingUsers.value, [userId]: true }

  try {
    const { data } = await usersApi.get(userId)
    upsertHydratedUser(userId, {
      id: data.id,
      username: data.username,
      display_name: data.display_name,
      email: data.email,
      nickname: data.nickname ?? null,
      first_name: data.first_name ?? null,
      last_name: data.last_name ?? null,
      position: data.position ?? null,
      avatar_url: data.avatar_url,
      presence: data.presence,
      status_text: data.status_text ?? null,
      status_emoji: data.status_emoji ?? null,
      status_expires_at: data.status_expires_at ?? null,
      fully_hydrated: true,
    })
    userErrors.value = { ...userErrors.value, [userId]: undefined }
  } catch (err: any) {
    userErrors.value = {
      ...userErrors.value,
      [userId]: err?.response?.data?.message || 'Failed to load user profile',
    }
  } finally {
    pendingUsers.value = { ...pendingUsers.value, [userId]: false }
  }
}

function markPendingUsers(userIds: string[], pending: boolean) {
  const next = { ...pendingUsers.value }
  userIds.forEach((userId) => {
    next[userId] = pending
  })
  pendingUsers.value = next
}

function mapHydratedUser(user: User): HydratedUserRecord {
  return {
    id: user.id,
    username: user.username,
    display_name: user.display_name,
    email: user.email,
    nickname: user.nickname ?? null,
    first_name: user.first_name ?? null,
    last_name: user.last_name ?? null,
    position: user.position ?? null,
    avatar_url: user.avatar_url ?? null,
    presence: user.presence ?? null,
    status_text: user.status_text ?? null,
    status_emoji: user.status_emoji ?? null,
    status_expires_at: user.status_expires_at ?? null,
    fully_hydrated: true,
  }
}

function upsertUsers(users: HydratedUserRecord[]) {
  const nextErrors = { ...userErrors.value }
  users.forEach((user) => {
    upsertHydratedUser(user.id, user)
    nextErrors[user.id] = undefined
  })
  userErrors.value = nextErrors
}

export function getUserSummarySnapshot(userId: string) {
  return mergeUserSummary(userId)
}

export function prefetchUserSummaries(userIds: string[]) {
  const uniqueIds = [...new Set(userIds.filter(Boolean))]
  const missingIds = uniqueIds.filter((userId) => {
    const existing = hydratedUsers.value[userId]
    return !pendingUsers.value[userId] && !existing?.fully_hydrated
  })

  if (missingIds.length === 0) {
    return
  }

  markPendingUsers(missingIds, true)

  void usersApi.getByIds(missingIds)
    .then(({ data }) => {
      const users = data.map(mapHydratedUser)
      upsertUsers(users)

      const returnedIds = new Set(users.map((user) => user.id))
      missingIds.forEach((userId) => {
        if (!returnedIds.has(userId)) {
          userErrors.value = {
            ...userErrors.value,
            [userId]: 'Failed to load user profile',
          }
        }
      })
    })
    .catch((err: any) => {
      const message = err?.response?.data?.message || 'Failed to load user profile'
      const nextErrors = { ...userErrors.value }
      missingIds.forEach((userId) => {
        nextErrors[userId] = message
      })
      userErrors.value = nextErrors
    })
    .finally(() => {
      markPendingUsers(missingIds, false)
    })
}

export function applyUserStatusSnapshot(update: LiveUserStatusUpdate) {
  const partial: Partial<HydratedUserRecord> = {}

  if (update.presence !== undefined) {
    partial.presence = update.presence
  }
  if ('statusText' in update) {
    partial.status_text = update.statusText ?? null
  }
  if ('statusEmoji' in update) {
    partial.status_emoji = update.statusEmoji ?? null
  }
  if ('statusExpiresAt' in update) {
    partial.status_expires_at = update.statusExpiresAt ?? null
  }

  upsertHydratedUser(update.userId, partial)
}

export function clearUserSummaryCache() {
  hydratedUsers.value = {}
  pendingUsers.value = {}
  userErrors.value = {}
  expiryTimers.forEach((_, userId) => clearStatusExpiryTimer(expiryTimers, userId))
  expiryTimers.clear()
}

export function useUserSummary(userIdSource: () => string | null | undefined) {
  const teamStore = useTeamStore()
  const presenceStore = usePresenceStore()

  const userSummary = computed(() => {
    const userId = userIdSource()
    if (!userId) {
      return null
    }
    return mergeUserSummary(userId, teamStore, presenceStore)
  })

  const isLoading = computed(() => {
    const userId = userIdSource()
    return !!(userId && pendingUsers.value[userId])
  })

  const error = computed(() => {
    const userId = userIdSource()
    return (userId && userErrors.value[userId]) || ''
  })

  watchEffect(() => {
    const userId = userIdSource()
    if (!userId) {
      return
    }
    void ensureUserSummary(userId)
  })

  return {
    userSummary,
    isLoading,
    error,
  }
}
