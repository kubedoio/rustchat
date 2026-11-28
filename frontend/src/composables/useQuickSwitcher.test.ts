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

  // The production shortcut path (QuickSwitcherHost keydown) only calls
  // toggle(), so fetch coverage must go through toggle as well; exercising
  // open() alone hid the bug where toggle flipped isOpen without fetching.
  it('fetches teams when toggle opens the switcher (non-AppShell pages)', () => {
    const switcher = useQuickSwitcher()

    switcher.toggle()

    expect(mocks.teamStore.fetchTeams).toHaveBeenCalledTimes(1)
    expect(switcher.isOpen.value).toBe(true)
  })

  it('does not refetch teams that are already loaded', () => {
    mocks.teamStore.teams = [{ id: 'team-1', name: 'Engineering' }]

    useQuickSwitcher().toggle()

    expect(mocks.teamStore.fetchTeams).not.toHaveBeenCalled()
  })

  it('does not stack fetches while a team fetch is already in flight', () => {
    mocks.teamStore.loading = true
    const switcher = useQuickSwitcher()

    // Rapid Cmd+K presses: open, close, open again while the first fetch is
    // still running.
    switcher.toggle()
    switcher.toggle()
    switcher.toggle()

    expect(mocks.teamStore.fetchTeams).not.toHaveBeenCalled()
    expect(switcher.isOpen.value).toBe(true)
  })

  it('closes on toggle while open without fetching again', () => {
    const switcher = useQuickSwitcher()

    switcher.toggle()
    expect(switcher.isOpen.value).toBe(true)

    switcher.toggle()

    expect(switcher.isOpen.value).toBe(false)
    expect(mocks.teamStore.fetchTeams).toHaveBeenCalledTimes(1)
  })
})
