// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useQuickSwitcher } from './useQuickSwitcher'

const mocks = vi.hoisted(() => ({
  teamStore: {
    teams: [] as Array<Record<string, unknown>>,
    loading: false,
    fetchTeams: vi.fn(),
  },
  channelStore: {
    channels: [] as Array<Record<string, unknown>>,
  },
}))

vi.mock('@/features/teams/stores/teamStore', () => ({
  useTeamStore: () => mocks.teamStore,
}))

vi.mock('@/features/channels/stores/channelStore', () => ({
  useChannelStore: () => mocks.channelStore,
}))

describe('useQuickSwitcher team loading', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.teamStore.teams = []
    mocks.teamStore.loading = false
    mocks.channelStore.channels = []
    useQuickSwitcher().close()
  })

  it('fetches teams on open when the store is empty (non-AppShell pages)', () => {
    const switcher = useQuickSwitcher()

    switcher.open()

    expect(mocks.teamStore.fetchTeams).toHaveBeenCalledTimes(1)
    expect(switcher.isOpen.value).toBe(true)
  })

  it('does not refetch teams that are already loaded', () => {
    mocks.teamStore.teams = [{ id: 'team-1', name: 'Engineering' }]

    useQuickSwitcher().open()

    expect(mocks.teamStore.fetchTeams).not.toHaveBeenCalled()
  })

  it('does not stack fetches while a team fetch is already in flight', () => {
    mocks.teamStore.loading = true

    useQuickSwitcher().open()
    useQuickSwitcher().open()

    expect(mocks.teamStore.fetchTeams).not.toHaveBeenCalled()
  })
})
