// @vitest-environment jsdom
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useQuickSwitcher } from '../../composables/useQuickSwitcher'

/**
 * Host-level regression: production only reaches the switcher through the
 * Cmd+K keydown handler, which calls toggle(). These tests mount the real host
 * (and the real composable) so a toggle() that bypasses open() cannot pass.
 */
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

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
}))

async function mountHost() {
  const QuickSwitcherHost = (await import('./QuickSwitcherHost.vue')).default
  return mount(QuickSwitcherHost, { attachTo: document.body })
}

function pressShortcut(init: KeyboardEventInit) {
  document.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, ...init }))
}

describe('QuickSwitcherHost team loading', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.teamStore.teams = []
    mocks.teamStore.loading = false
    mocks.channelStore.channels = []
    useQuickSwitcher().close()
  })

  it('fetches teams when Cmd+K opens the switcher outside AppShell', async () => {
    const wrapper = await mountHost()

    pressShortcut({ key: 'k', ctrlKey: true })
    await nextTick()

    expect(mocks.teamStore.fetchTeams).toHaveBeenCalledTimes(1)
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(1)

    // Closing must go through close(), not a second fetch.
    pressShortcut({ key: 'k', ctrlKey: true })
    await nextTick()

    expect(mocks.teamStore.fetchTeams).toHaveBeenCalledTimes(1)
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(0)

    wrapper.unmount()
  })

  it('does not fetch when Cmd+K opens with teams already loaded', async () => {
    mocks.teamStore.teams = [{ id: 'team-1', name: 'Engineering' }]
    const wrapper = await mountHost()

    pressShortcut({ key: 'K', metaKey: true })
    await nextTick()

    expect(mocks.teamStore.fetchTeams).not.toHaveBeenCalled()
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(1)

    wrapper.unmount()
  })
})
