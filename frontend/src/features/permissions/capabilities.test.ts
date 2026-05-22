import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import { reactive } from 'vue'

const getMembersMock = vi.fn()

const authStore = reactive({
  user: {
    id: 'user-1',
    role: 'member',
  } as { id: string; role: string } | null,
})

const teamStore = reactive({
  currentTeamId: 'team-1',
  members: [] as Array<{ user_id: string; role: string }>,
})

vi.mock('../../api/channels', () => ({
  channelsApi: {
    getMembers: getMembersMock,
  },
}))

vi.mock('../../features/auth/stores/authStore', () => ({
  useAuthStore: () => authStore,
}))

vi.mock('@/features/teams/stores/teamStore', () => ({
  useTeamStore: () => teamStore,
}))

async function flushPromises() {
  await Promise.resolve()
  await nextTick()
  await Promise.resolve()
}

describe('permission capabilities', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    authStore.user = {
      id: 'user-1',
      role: 'member',
    }
    teamStore.currentTeamId = 'team-1'
    teamStore.members = []

    const module = await import('./capabilities')
    module.clearChannelPermissionCache()
  })

  it('allows team creation only for team-managing roles', async () => {
    const { canCreateTeam } = await import('./capabilities')

    expect(canCreateTeam('member')).toBe(false)
    expect(canCreateTeam('guest')).toBe(false)
    expect(canCreateTeam('org_admin')).toBe(true)
    expect(canCreateTeam('system_admin')).toBe(true)
  })

  it('allows channel creation for members and admins but not guests', async () => {
    const { canCreateChannel } = await import('./capabilities')

    expect(canCreateChannel('guest')).toBe(false)
    expect(canCreateChannel('member')).toBe(true)
    expect(canCreateChannel('org_admin')).toBe(true)
  })

  it('allows team management for team admins through membership role', async () => {
    const { canManageTeam, useCurrentTeamManagementPermission } = await import('./capabilities')

    expect(
      canManageTeam({
        currentUserRole: 'member',
        currentTeamMembershipRole: 'admin',
      })
    ).toBe(true)

    teamStore.members = [
      { user_id: 'user-1', role: 'admin' },
      { user_id: 'user-2', role: 'member' },
    ]

    const { canManageTeam: canManageCurrentTeam } = useCurrentTeamManagementPermission(
      () => 'team-1'
    )

    await flushPromises()

    expect(canManageCurrentTeam.value).toBe(true)
  })

  it('keeps team management disabled for regular team members', async () => {
    const { useCurrentTeamManagementPermission } = await import('./capabilities')

    teamStore.members = [{ user_id: 'user-1', role: 'member' }]

    const { canManageTeam: canManageCurrentTeam } = useCurrentTeamManagementPermission(
      () => 'team-1'
    )

    await flushPromises()

    expect(canManageCurrentTeam.value).toBe(false)
  })

  it('allows channel management for the channel creator without fetching members', async () => {
    const { useChannelManagementPermission } = await import('./capabilities')

    const { canManageChannel } = useChannelManagementPermission(
      () => 'channel-1',
      () => 'user-1'
    )

    await flushPromises()

    expect(canManageChannel.value).toBe(true)
    expect(getMembersMock).not.toHaveBeenCalled()
  })

  it('loads membership once and grants channel management to channel admins', async () => {
    getMembersMock.mockResolvedValue({
      data: [
        { user_id: 'user-1', role: 'admin' },
        { user_id: 'user-2', role: 'member' },
      ],
    })

    const { useChannelManagementPermission } = await import('./capabilities')
    const { canManageChannel, isLoading } = useChannelManagementPermission(
      () => 'channel-2',
      () => 'user-9'
    )

    expect(isLoading.value).toBe(true)
    await flushPromises()

    expect(getMembersMock).toHaveBeenCalledTimes(1)
    expect(getMembersMock).toHaveBeenCalledWith('channel-2')
    expect(isLoading.value).toBe(false)
    expect(canManageChannel.value).toBe(true)
  })

  it('keeps channel management disabled for a regular member', async () => {
    getMembersMock.mockResolvedValue({
      data: [{ user_id: 'user-1', role: 'member' }],
    })

    const { useChannelManagementPermission } = await import('./capabilities')
    const { canManageChannel } = useChannelManagementPermission(
      () => 'channel-3',
      () => 'user-9'
    )

    await flushPromises()

    expect(canManageChannel.value).toBe(false)
  })

  it('handles API failures gracefully and allows retry via cache clear or channel change', async () => {
    // Setup: API fails
    getMembersMock.mockRejectedValue(new Error('Network Error'))

    const { useChannelManagementPermission } = await import('./capabilities')
    const { canManageChannel, isLoading } = useChannelManagementPermission(
      () => 'channel-fail',
      () => 'user-9'
    )

    // After flush: should have attempted to load but failed
    await flushPromises()
    expect(getMembersMock).toHaveBeenCalled()
    expect(isLoading.value).toBe(false)
    expect(canManageChannel.value).toBe(false)

    // Now simulate recovery: clear cache and change to a new channel with successful API
    const module = await import('./capabilities')
    module.clearChannelPermissionCache()

    // Setup success for the next channel
    getMembersMock.mockResolvedValue({
      data: [{ user_id: 'user-1', role: 'admin' }],
    })

    // Use a new channel ID to trigger fresh loading
    const { canManageChannel: canManageRecovered } = useChannelManagementPermission(
      () => 'channel-recovered',
      () => 'user-9'
    )

    await flushPromises()
    expect(canManageRecovered.value).toBe(true)
  })
})
