// @vitest-environment jsdom

import { nextTick, reactive, ref } from 'vue'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const routerPush = vi.fn()
const prefetchUserSummaries = vi.fn()
const canManageTeam = ref(false)

let userSummaries: Record<string, Record<string, unknown>> = {}
let favoriteChannelIds = new Set<string>()
let typingChannelIds = new Set<string>()

const authStore = reactive({
  user: { id: 'user-1', role: 'member', username: 'me' } as Record<string, unknown> | null,
})

const teamStore = reactive({
  currentTeamId: 'team-1' as string | null,
  currentTeam: { id: 'team-1', name: 'team-1', display_name: 'Team 1' } as Record<
    string,
    unknown
  > | null,
  members: [] as Array<Record<string, unknown>>,
  fetchMembers: vi.fn(),
  removeTeam: vi.fn(),
  leaveTeam: vi.fn(),
})

const channelStore = reactive({
  publicChannels: [] as Array<Record<string, unknown>>,
  privateChannels: [] as Array<Record<string, unknown>>,
  directMessages: [] as Array<Record<string, unknown>>,
  channels: [] as Array<Record<string, unknown>>,
  currentChannelId: null as string | null,
  loading: false,
  fetchChannels: vi.fn(),
  clearChannels: vi.fn(),
  selectChannel: vi.fn(),
  removeChannel: vi.fn(),
  clearCounts: vi.fn(),
})

const unreadStore = reactive({
  markAsRead: vi.fn(),
  markAllAsRead: vi.fn(),
})

const channelPrefsStore = reactive({
  isFavorite: vi.fn((channelId: string) => favoriteChannelIds.has(channelId)),
  fetchPreferences: vi.fn(),
})

const presenceStore = reactive({
  hasTypingUsers: vi.fn((channelId: string) => typingChannelIds.has(channelId)),
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
}))

vi.mock('../../features/auth/stores/authStore', () => ({
  useAuthStore: () => authStore,
}))

vi.mock('@/features/teams/stores/teamStore', () => ({
  useTeamStore: () => teamStore,
}))

vi.mock('@/features/channels/stores/channelStore', () => ({
  useChannelStore: () => channelStore,
}))

vi.mock('@/features/unreads/stores/unreadStore', () => ({
  useUnreadStore: () => unreadStore,
}))

vi.mock('@/features/channels/stores/channelPreferencesStore', () => ({
  useChannelPreferencesStore: () => channelPrefsStore,
}))

vi.mock('../../features/presence', () => ({
  usePresenceStore: () => presenceStore,
}))

vi.mock('../../features/channels/repositories/channelRepository', () => ({
  channelRepository: { updateCategories: vi.fn() },
}))

vi.mock('../../composables/useUserSummary', () => ({
  getUserSummarySnapshot: (userId: string) => userSummaries[userId] ?? null,
  prefetchUserSummaries,
}))

// The capabilities module is hand-mocked: canManageTeam is driven directly
// instead of deriving it from team membership. The real derivation has its
// own coverage in capabilities.test.ts; here we only need the sidebar's
// gating behavior for each canManageTeam state.
vi.mock('../../features/permissions/capabilities', () => ({
  canCreateChannel: (role?: string | null) =>
    ['system_admin', 'org_admin', 'team_admin', 'admin', 'member'].includes(role ?? ''),
  useCurrentTeamManagementPermission: () => ({
    canManageTeam,
    currentTeamMembershipRole: ref(null),
  }),
}))

// Channel fixtures: display_name and channel_type drive the sidebar's
// category grouping and sort; unreadCount/mentionCount are zeroed by default
// so badge tests only need to set the counts they assert. The DM fixture's
// `user-1__user-2` name encodes the member pair that the display-name
// fallback chain resolves against.
function generalChannel(): Record<string, unknown> {
  return {
    id: 'ch-general',
    name: 'general',
    display_name: 'General',
    channel_type: 'public',
    unreadCount: 0,
    mentionCount: 0,
  }
}

function zetaChannel(): Record<string, unknown> {
  return {
    id: 'ch-zeta',
    name: 'zeta',
    display_name: 'Zeta Labs',
    channel_type: 'public',
    unreadCount: 0,
    mentionCount: 0,
  }
}

function alphaChannel(): Record<string, unknown> {
  return {
    id: 'ch-alpha',
    name: 'alpha',
    display_name: 'Alpha Plans',
    channel_type: 'private',
    unreadCount: 0,
    mentionCount: 0,
  }
}

function betaChannel(): Record<string, unknown> {
  return {
    id: 'ch-beta',
    name: 'beta',
    display_name: 'Beta Ops',
    channel_type: 'private',
    unreadCount: 0,
    mentionCount: 0,
  }
}

function dmChannel(): Record<string, unknown> {
  return {
    id: 'dm-1',
    name: 'user-1__user-2',
    channel_type: 'direct',
    unreadCount: 0,
    mentionCount: 0,
  }
}

function installChannels(channels: Array<Record<string, unknown>>) {
  channelStore.publicChannels = channels.filter(c => c.channel_type === 'public')
  channelStore.privateChannels = channels.filter(c => c.channel_type === 'private')
  channelStore.directMessages = channels.filter(c => c.channel_type === 'direct')
  channelStore.channels = [...channels]
}

async function mountSidebar(): Promise<VueWrapper> {
  const ChannelSidebar = (await import('./ChannelSidebar.vue')).default
  return mount(ChannelSidebar, {
    global: {
      stubs: {
        CreateChannelModal: true,
        DirectMessageModal: true,
        TeamSettingsModal: true,
        BrowseTeamsModal: true,
        BrowseChannelsModal: true,
        AddChannelMembersModal: true,
        EditChannelModal: true,
        ChannelContextMenu: true,
        RcAvatar: true,
        teleport: true,
      },
    },
  })
}

function rowNames(wrapper: VueWrapper, testId: 'channel-sidebar-row' | 'dm-sidebar-row') {
  return wrapper.findAll(`[data-testid="${testId}"]`).map(row => row.find('span.text-sm').text())
}

async function openTeamMenu(wrapper: VueWrapper) {
  // The team header is the only .group containing 'Workspace'; category
  // headers share the class but never contain that text.
  const header = wrapper.findAll('.group').find(el => el.text().includes('Workspace'))
  expect(header).toBeDefined()
  await header!.trigger('click')
}

describe('ChannelSidebar', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    authStore.user = { id: 'user-1', role: 'member', username: 'me' }
    teamStore.currentTeamId = 'team-1'
    teamStore.currentTeam = { id: 'team-1', name: 'team-1', display_name: 'Team 1' }
    teamStore.members = []
    channelStore.publicChannels = []
    channelStore.privateChannels = []
    channelStore.directMessages = []
    channelStore.channels = []
    channelStore.currentChannelId = null
    channelStore.loading = false
    canManageTeam.value = false
    userSummaries = {}
    favoriteChannelIds = new Set()
    typingChannelIds = new Set()
  })

  // The next three tests assert store wiring only: the currentTeamId watch
  // has no direct DOM counterpart, so the fetch/clear calls ARE the behavior.
  it('fetches channels, members and preferences for the current team on mount', async () => {
    const wrapper = await mountSidebar()

    expect(channelStore.fetchChannels).toHaveBeenCalledWith('team-1')
    expect(teamStore.fetchMembers).toHaveBeenCalledWith('team-1')
    expect(channelPrefsStore.fetchPreferences).toHaveBeenCalledTimes(1)

    wrapper.unmount()
  })

  it('reloads channels when the team changes and clears them when the team is unset', async () => {
    const wrapper = await mountSidebar()
    expect(channelStore.fetchChannels).toHaveBeenCalledTimes(1)

    teamStore.currentTeamId = null
    await nextTick()

    expect(channelStore.clearChannels).toHaveBeenCalledTimes(1)
    expect(channelStore.fetchChannels).toHaveBeenCalledTimes(1)
    expect(teamStore.fetchMembers).toHaveBeenCalledTimes(1)

    teamStore.currentTeamId = 'team-2'
    await nextTick()

    expect(channelStore.fetchChannels).toHaveBeenCalledWith('team-2')
    expect(teamStore.fetchMembers).toHaveBeenCalledWith('team-2')

    wrapper.unmount()
  })

  it('clears channels immediately when mounted without a team', async () => {
    teamStore.currentTeamId = null
    const wrapper = await mountSidebar()

    expect(channelStore.clearChannels).toHaveBeenCalledTimes(1)
    expect(channelStore.fetchChannels).not.toHaveBeenCalled()
    expect(teamStore.fetchMembers).not.toHaveBeenCalled()

    wrapper.unmount()
  })

  it('merges public and private channels into Channels sorted by display name', async () => {
    installChannels([zetaChannel(), betaChannel(), alphaChannel(), generalChannel()])
    const wrapper = await mountSidebar()

    expect(rowNames(wrapper, 'channel-sidebar-row')).toEqual([
      'Alpha Plans',
      'Beta Ops',
      'General',
      'Zeta Labs',
    ])
    expect(wrapper.find('[data-testid="dm-sidebar-row"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('Favorites')

    wrapper.unmount()
  })

  it('shows a Favorites section and excludes favorited channels from Channels', async () => {
    installChannels([generalChannel(), zetaChannel(), betaChannel(), alphaChannel()])
    favoriteChannelIds.add('ch-alpha')
    const wrapper = await mountSidebar()

    expect(wrapper.text()).toContain('Favorites')

    const names = rowNames(wrapper, 'channel-sidebar-row')
    expect(names).toEqual(['Alpha Plans', 'Beta Ops', 'General', 'Zeta Labs'])
    expect(names.filter(name => name === 'Alpha Plans')).toHaveLength(1)

    wrapper.unmount()
  })

  it('renders a DM row with the summary display name and custom status', async () => {
    installChannels([dmChannel()])
    userSummaries['user-2'] = {
      displayName: 'Alice Wonder',
      username: 'alice',
      avatarUrl: '',
      presence: 'online',
      statusText: 'In a meeting',
      statusEmoji: '☕',
    }
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-testid="dm-sidebar-row"]')
    expect(row.attributes('data-user-id')).toBe('user-2')
    expect(row.find('span.text-sm').text()).toBe('Alice Wonder')

    const status = row.get('[data-testid="dm-sidebar-status"]')
    expect(status.text()).toContain('In a meeting')
    expect(status.text()).toContain('☕')

    expect(prefetchUserSummaries).toHaveBeenCalledWith(['user-2'])

    wrapper.unmount()
  })

  it('falls back to the team member display name for DMs without a summary', async () => {
    installChannels([dmChannel()])
    teamStore.members = [
      { user_id: 'user-2', username: 'bob', display_name: 'Bob Builder', presence: 'away' },
    ]
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-testid="dm-sidebar-row"]')
    expect(row.find('span.text-sm').text()).toBe('Bob Builder')
    expect(row.get('[data-testid="dm-sidebar-status"]').text()).toContain('Away')

    wrapper.unmount()
  })

  it('falls back to the summary username when no display names are known', async () => {
    installChannels([dmChannel()])
    userSummaries['user-2'] = { username: 'dave', presence: 'online' }
    const wrapper = await mountSidebar()

    expect(wrapper.get('[data-testid="dm-sidebar-row"]').find('span.text-sm').text()).toBe('dave')

    wrapper.unmount()
  })

  it('falls back to the member username for DMs without display names', async () => {
    installChannels([dmChannel()])
    teamStore.members = [{ user_id: 'user-2', username: 'carol' }]
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-testid="dm-sidebar-row"]')
    expect(row.find('span.text-sm').text()).toBe('carol')
    expect(row.get('[data-testid="dm-sidebar-status"]').text()).toContain('Offline')

    wrapper.unmount()
  })

  it('falls back to the raw channel display name when no user info is available', async () => {
    installChannels([{ ...dmChannel(), display_name: 'Fallback Name' }])
    const wrapper = await mountSidebar()

    expect(wrapper.get('[data-testid="dm-sidebar-row"]').find('span.text-sm').text()).toBe(
      'Fallback Name'
    )

    wrapper.unmount()
  })

  it('highlights the current channel and suppresses its unread indicators', async () => {
    installChannels([{ ...generalChannel(), unreadCount: 5, mentionCount: 3 }])
    channelStore.currentChannelId = 'ch-general'
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-channel-id="ch-general"]')
    expect(row.classes()).toContain('border-brand/25')
    expect(row.find('.bg-brand').exists()).toBe(true)
    expect(row.classes()).not.toContain('border-transparent')
    expect(row.find('button[title="Mark as read"]').exists()).toBe(false)
    expect(row.find('.bg-danger').exists()).toBe(false)
    expect(row.find('.w-2.h-2').exists()).toBe(false)
    expect(row.text()).not.toContain('3')

    wrapper.unmount()
  })

  it('caps the mention badge at 99+', async () => {
    installChannels([{ ...generalChannel(), unreadCount: 2, mentionCount: 150 }])
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-channel-id="ch-general"]')
    expect(row.get('.bg-danger').text()).toBe('99+')
    expect(row.find('.w-2.h-2').exists()).toBe(false)

    wrapper.unmount()
  })

  it('shows an unread dot and mark-as-read button for unreads without mentions', async () => {
    installChannels([{ ...generalChannel(), unreadCount: 4 }])
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-channel-id="ch-general"]')
    expect(row.find('.bg-danger').exists()).toBe(false)
    expect(row.find('.w-2.h-2.rounded-full').exists()).toBe(true)
    expect(row.find('button[title="Mark as read"]').exists()).toBe(true)

    wrapper.unmount()
  })

  it('shows a typing indicator on a channel where someone is typing', async () => {
    installChannels([generalChannel()])
    typingChannelIds.add('ch-general')
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-channel-id="ch-general"]')
    expect(row.find('[title="Someone is typing..."]').exists()).toBe(true)

    wrapper.unmount()
  })

  it('hides the typing indicator on the current channel', async () => {
    installChannels([generalChannel()])
    typingChannelIds.add('ch-general')
    channelStore.currentChannelId = 'ch-general'
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-channel-id="ch-general"]')
    expect(row.find('[title="Someone is typing..."]').exists()).toBe(false)

    wrapper.unmount()
  })

  it('marks a channel as read optimistically before calling the unread store', async () => {
    installChannels([{ ...betaChannel(), unreadCount: 3 }])
    const wrapper = await mountSidebar()

    const row = wrapper.get('[data-channel-id="ch-beta"]')
    await row.get('button[title="Mark as read"]').trigger('click')
    await flushPromises()

    expect(channelStore.clearCounts).toHaveBeenCalledTimes(1)
    expect(channelStore.clearCounts).toHaveBeenCalledWith('ch-beta')
    expect(unreadStore.markAsRead).toHaveBeenCalledTimes(1)
    expect(unreadStore.markAsRead).toHaveBeenCalledWith('ch-beta')
    expect(channelStore.clearCounts.mock.invocationCallOrder[0]).toBeLessThan(
      unreadStore.markAsRead.mock.invocationCallOrder[0]
    )

    wrapper.unmount()
  })

  it('hides Mark all as read when no channel has unreads', async () => {
    installChannels([generalChannel(), betaChannel()])
    const wrapper = await mountSidebar()

    expect(wrapper.text()).not.toContain('Mark all as read')

    wrapper.unmount()
  })

  it('shows Mark all as read when there are unreads and clears every channel', async () => {
    const beta = { ...betaChannel(), unreadCount: 2, mentionCount: 1 }
    installChannels([generalChannel(), beta])
    const wrapper = await mountSidebar()

    const button = wrapper.findAll('button').find(b => b.text().includes('Mark all as read'))
    expect(button).toBeDefined()

    await button!.trigger('click')
    await flushPromises()

    expect(unreadStore.markAllAsRead).toHaveBeenCalledTimes(1)
    expect(beta.unreadCount).toBe(0)
    expect(beta.mentionCount).toBe(0)

    wrapper.unmount()
  })

  it('shows System Console for system admins and routes to the admin area', async () => {
    authStore.user = { id: 'user-1', role: 'system_admin', username: 'me' }
    const wrapper = await mountSidebar()

    await openTeamMenu(wrapper)
    expect(wrapper.text()).toContain('System Console')

    const consoleButton = wrapper.findAll('button').find(b => b.text().includes('System Console'))
    expect(consoleButton).toBeDefined()
    await consoleButton!.trigger('click')

    expect(routerPush).toHaveBeenCalledWith('/admin')

    wrapper.unmount()
  })

  it('hides System Console for members', async () => {
    const wrapper = await mountSidebar()

    await openTeamMenu(wrapper)
    expect(wrapper.text()).not.toContain('System Console')

    wrapper.unmount()
  })

  it('hides Team Settings when the user cannot manage the current team', async () => {
    const wrapper = await mountSidebar()

    await openTeamMenu(wrapper)
    expect(wrapper.text()).not.toContain('Team Settings')

    wrapper.unmount()
  })

  it('shows Team Settings for team managers', async () => {
    canManageTeam.value = true
    const wrapper = await mountSidebar()

    await openTeamMenu(wrapper)
    expect(wrapper.text()).toContain('Team Settings')

    wrapper.unmount()
  })

  it('renders a loading state instead of the channel categories', async () => {
    installChannels([generalChannel()])
    channelStore.loading = true
    const wrapper = await mountSidebar()

    expect(wrapper.text()).toContain('Loading channels...')
    expect(wrapper.find('.animate-spin').exists()).toBe(true)
    expect(wrapper.find('[data-testid="channel-sidebar-row"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="dm-sidebar-row"]').exists()).toBe(false)

    wrapper.unmount()
  })

  it('selects a channel when its row is clicked', async () => {
    installChannels([generalChannel()])
    const wrapper = await mountSidebar()

    await wrapper.get('[data-channel-id="ch-general"]').trigger('click')

    expect(channelStore.selectChannel).toHaveBeenCalledTimes(1)
    expect(channelStore.selectChannel).toHaveBeenCalledWith('ch-general')

    wrapper.unmount()
  })
})
