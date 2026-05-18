import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick, ref } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

const getMock = vi.fn()
const getByIdsMock = vi.fn()

const teamStore = {
  members: [] as Array<{
    user_id: string
    username?: string
    display_name?: string
    avatar_url?: string
    presence?: string
  }>,
}

const presenceStore = {
  getUserPresence: vi.fn(() => ref(undefined)),
}

vi.mock('../api/users', () => ({
  usersApi: {
    get: getMock,
    getByIds: getByIdsMock,
  },
}))

vi.mock('@/features/teams/stores/teamStore', () => ({
  useTeamStore: () => teamStore,
}))

vi.mock('../features/presence', () => ({
  usePresenceStore: () => presenceStore,
}))

async function flushPromises() {
  await Promise.resolve()
  await nextTick()
  await Promise.resolve()
}

describe('useUserSummary', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-03-29T12:00:00Z'))
    vi.clearAllMocks()
    teamStore.members = []

    const module = await import('./useUserSummary')
    module.clearUserSummaryCache()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('prefetches missing users through /users/ids and hydrates the snapshot cache', async () => {
    getByIdsMock.mockResolvedValue({
      data: [
        {
          id: 'user-2',
          username: 'alex',
          email: 'alex@example.com',
          display_name: 'Alex Doe',
          avatar_url: '/avatars/alex.png',
          nickname: 'Lex',
          first_name: 'Alex',
          last_name: 'Doe',
          position: 'Engineer',
          role: 'member',
          presence: 'away',
          status_text: 'Heads down',
          status_emoji: ':coffee:',
          status_expires_at: null,
          created_at: '2026-03-29T10:00:00Z',
        },
      ],
    })

    const { getUserSummarySnapshot, prefetchUserSummaries } = await import('./useUserSummary')

    prefetchUserSummaries(['user-2', 'user-2'])
    await flushPromises()

    expect(getByIdsMock).toHaveBeenCalledTimes(1)
    expect(getByIdsMock).toHaveBeenCalledWith(['user-2'])

    expect(getUserSummarySnapshot('user-2')).toEqual({
      id: 'user-2',
      username: 'alex',
      displayName: 'Alex Doe',
      email: 'alex@example.com',
      nickname: 'Lex',
      firstName: 'Alex',
      lastName: 'Doe',
      position: 'Engineer',
      avatarUrl: '/avatars/alex.png',
      presence: 'away',
      statusText: 'Heads down',
      statusEmoji: ':coffee:',
      statusExpiresAt: undefined,
    })
  })

  it('falls back to single-user hydration for detail surfaces and clears timed status at expiry', async () => {
    getMock.mockResolvedValue({
      data: {
        id: 'user-1',
        username: 'sam',
        email: 'sam@example.com',
        display_name: 'Sam Example',
        avatar_url: '/avatars/sam.png',
        nickname: 'Sam',
        first_name: 'Sam',
        last_name: 'Example',
        position: 'Support',
        role: 'member',
        presence: 'online',
        status_text: 'In focus',
        status_emoji: ':spiral_calendar_pad:',
        status_expires_at: '2026-03-29T12:01:00Z',
        created_at: '2026-03-29T10:00:00Z',
      },
    })

    const { useUserSummary } = await import('./useUserSummary')
    const currentUserId = ref('user-1')
    const { userSummary, isLoading, error } = useUserSummary(() => currentUserId.value)

    await flushPromises()

    expect(getMock).toHaveBeenCalledWith('user-1')
    expect(isLoading.value).toBe(false)
    expect(error.value).toBe('')
    expect(userSummary.value?.statusText).toBe('In focus')
    expect(userSummary.value?.statusEmoji).toBe(':spiral_calendar_pad:')

    vi.advanceTimersByTime(60_000)
    await flushPromises()

    expect(userSummary.value?.statusText).toBeUndefined()
    expect(userSummary.value?.statusEmoji).toBeUndefined()
  })

  it('applies live status snapshots on top of cached user summaries', async () => {
    getByIdsMock.mockResolvedValue({
      data: [
        {
          id: 'user-3',
          username: 'jordan',
          email: 'jordan@example.com',
          display_name: 'Jordan Lee',
          avatar_url: null,
          nickname: null,
          first_name: 'Jordan',
          last_name: 'Lee',
          position: null,
          role: 'member',
          presence: 'offline',
          status_text: null,
          status_emoji: null,
          status_expires_at: null,
          created_at: '2026-03-29T10:00:00Z',
        },
      ],
    })

    const {
      applyUserStatusSnapshot,
      getUserSummarySnapshot,
      prefetchUserSummaries,
    } = await import('./useUserSummary')

    prefetchUserSummaries(['user-3'])
    await flushPromises()

    applyUserStatusSnapshot({
      userId: 'user-3',
      presence: 'online',
      statusText: 'Available now',
      statusEmoji: ':wave:',
      statusExpiresAt: null,
    })

    expect(getUserSummarySnapshot('user-3')).toMatchObject({
      id: 'user-3',
      presence: 'online',
      statusText: 'Available now',
      statusEmoji: ':wave:',
    })
  })
})
