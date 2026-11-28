import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope, nextTick, reactive } from 'vue'
import type { EffectScope } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

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

const testScopes: EffectScope[] = []

// Composables rely on their component's effect scope to dispose watchers. In a
// unit test there is no component, so each instance must run inside an owned
// scope that is stopped after the test.
function runInTestScope<T>(factory: () => T): T {
  const scope = effectScope()
  testScopes.push(scope)

  const result = scope.run(factory)
  if (result === undefined) {
    throw new Error('Test effect scope factory returned undefined')
  }

  return result
}

describe('permission capabilities', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    // Clear call history and per-test implementations so a mock configured in
    // one test cannot influence another one.
    getMembersMock.mockReset()
    authStore.user = {
      id: 'user-1',
      role: 'member',
    }
    teamStore.currentTeamId = 'team-1'
    teamStore.members = []

    const module = await import('./capabilities')
    // Clear the module-level cache after mocks reset and before this test's
    // instances are created, so snapshots cannot leak across tests.
    module.clearChannelPermissionCache()
  })

  afterEach(() => {
    for (const scope of testScopes.splice(0)) {
      scope.stop()
    }
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

    const { canManageTeam: canManageCurrentTeam } = runInTestScope(() =>
      useCurrentTeamManagementPermission(() => 'team-1')
    )

    await flushPromises()

    expect(canManageCurrentTeam.value).toBe(true)
  })

  it('keeps team management disabled for regular team members', async () => {
    const { useCurrentTeamManagementPermission } = await import('./capabilities')

    teamStore.members = [{ user_id: 'user-1', role: 'member' }]

    const { canManageTeam: canManageCurrentTeam } = runInTestScope(() =>
      useCurrentTeamManagementPermission(() => 'team-1')
    )

    await flushPromises()

    expect(canManageCurrentTeam.value).toBe(false)
  })

  it('allows channel management for the channel creator without fetching members', async () => {
    const { useChannelManagementPermission } = await import('./capabilities')

    const { canManageChannel } = runInTestScope(() =>
      useChannelManagementPermission(
        () => 'channel-1',
        () => 'user-1'
      )
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
    const { canManageChannel, isLoading } = runInTestScope(() =>
      useChannelManagementPermission(
        () => 'channel-2',
        () => 'user-9'
      )
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
    const { canManageChannel } = runInTestScope(() =>
      useChannelManagementPermission(
        () => 'channel-3',
        () => 'user-9'
      )
    )

    await flushPromises()

    expect(canManageChannel.value).toBe(false)
  })

  it('handles API failures without retrying until the channel changes', async () => {
    getMembersMock.mockRejectedValue(new Error('Network Error'))

    const { useChannelManagementPermission } = await import('./capabilities')
    const { canManageChannel, isLoading } = runInTestScope(() =>
      useChannelManagementPermission(
        () => 'channel-fail',
        () => 'user-9'
      )
    )

    // A failed load is terminal for the same inputs: recording the failure
    // snapshot must not trigger the watcher into another request.
    await flushPromises()
    expect(getMembersMock).toHaveBeenCalledTimes(1)
    expect(getMembersMock).toHaveBeenCalledWith('channel-fail')
    expect(isLoading.value).toBe(false)
    expect(canManageChannel.value).toBe(false)

    await flushPromises()
    await nextTick()
    expect(getMembersMock).toHaveBeenCalledTimes(1)

    // Now simulate recovery: clear the cache and use a new channel with a
    // successful API. The existing failed watcher must not refetch.
    const module = await import('./capabilities')
    module.clearChannelPermissionCache()

    // Setup success for the next channel
    getMembersMock.mockResolvedValue({
      data: [{ user_id: 'user-1', role: 'admin' }],
    })

    // Use a new channel ID to trigger fresh loading
    const { canManageChannel: canManageRecovered } = runInTestScope(() =>
      useChannelManagementPermission(
        () => 'channel-recovered',
        () => 'user-9'
      )
    )

    await flushPromises()
    expect(getMembersMock).toHaveBeenCalledWith('channel-recovered')
    expect(canManageRecovered.value).toBe(true)
    expect(getMembersMock.mock.calls.filter(call => call[0] === 'channel-fail')).toHaveLength(1)
  })

  it('disposes watchers with their effect scope so stale instances cannot fire later', async () => {
    getMembersMock.mockResolvedValue({ data: [] })
    const { useChannelManagementPermission, clearChannelPermissionCache } =
      await import('./capabilities')

    // Simulate the watcher owned by an earlier test that has now finished.
    const previousTestScope = effectScope()
    testScopes.push(previousTestScope)
    previousTestScope.run(() => {
      useChannelManagementPermission(
        () => 'channel-previous',
        () => 'user-9'
      )
    })

    await flushPromises()
    expect(getMembersMock).toHaveBeenCalledTimes(1)
    expect(getMembersMock).toHaveBeenCalledWith('channel-previous')

    previousTestScope.stop()
    // Drop the successful snapshot so a leaked watcher would have to refetch
    // instead of hitting the `loaded` early-return.
    clearChannelPermissionCache()

    // The next test creates its own instance. Mutating the shared stores below
    // must only be seen by the live instance, never by the stopped one.
    runInTestScope(() =>
      useChannelManagementPermission(
        () => 'channel-current',
        () => 'user-9'
      )
    )

    await flushPromises()
    expect(getMembersMock).toHaveBeenCalledTimes(2)
    expect(getMembersMock).toHaveBeenLastCalledWith('channel-current')

    authStore.user = { id: 'user-2', role: 'member' }
    await flushPromises()

    expect(getMembersMock.mock.calls.filter(call => call[0] === 'channel-previous')).toHaveLength(1)
    expect(getMembersMock).toHaveBeenLastCalledWith('channel-current')
  })
})
