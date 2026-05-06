// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'

const mocks = vi.hoisted(() => ({
    authStore: {
        user: {
            id: 'user-1',
            role: 'member',
        } as { id: string; role: string } | null,
    },
    teamStore: {
        teams: [],
        loading: false,
        currentTeamId: 'team-1',
        fetchTeams: vi.fn(),
        fetchMembers: vi.fn(),
        selectTeam: vi.fn(),
        members: [],
    },
    unreadStore: {
        getTeamUnreadCount: vi.fn(() => 0),
        markAsRead: vi.fn(),
        markAsUnread: vi.fn(),
    },
    channelStore: {
        updateChannel: vi.fn(),
        channels: [],
        currentChannelId: null,
        publicChannels: [],
        privateChannels: [],
        directMessages: [],
        fetchChannels: vi.fn(),
        selectChannel: vi.fn(),
        clearChannels: vi.fn(),
        removeChannel: vi.fn(),
        clearCounts: vi.fn(),
    },
    channelPrefsStore: {
        isFavorite: vi.fn(() => false),
        isMuted: vi.fn(() => false),
        toggleFavorite: vi.fn(),
        toggleMute: vi.fn(),
        fetchPreferences: vi.fn(),
    },
    channelRepository: {
        getCategories: vi.fn(async () => []),
        leave: vi.fn(),
    },
    postsApi: {
        list: vi.fn(async () => ({ data: { messages: [] } })),
    },
    channelsApi: {
        getMembers: vi.fn(async () => ({ data: [] })),
        update: vi.fn(),
        delete: vi.fn(),
        join: vi.fn(),
        removeMember: vi.fn(),
    },
    toast: {
        success: vi.fn(),
        error: vi.fn(),
    },
    canManageTeam: { value: false, __v_isRef: true },
    canManageChannel: { value: false, __v_isRef: true },
    isLoading: { value: false, __v_isRef: true },
}))

vi.mock('../../stores/auth', () => ({
    useAuthStore: () => mocks.authStore,
}))

vi.mock('../../stores/teams', () => ({
    useTeamStore: () => mocks.teamStore,
}))

vi.mock('../../stores/unreads', () => ({
    useUnreadStore: () => mocks.unreadStore,
}))

vi.mock('../../stores/channels', () => ({
    useChannelStore: () => mocks.channelStore,
}))

vi.mock('../../stores/channelPreferences', () => ({
    useChannelPreferencesStore: () => mocks.channelPrefsStore,
}))

vi.mock('../../features/channels/repositories/channelRepository', () => ({
    channelRepository: mocks.channelRepository,
}))

vi.mock('../../api/posts', () => ({
    postsApi: mocks.postsApi,
}))

vi.mock('../../api/channels', () => ({
    channelsApi: mocks.channelsApi,
}))

vi.mock('../../composables/useToast', () => ({
    useToast: () => mocks.toast,
}))

vi.mock('../../features/presence', () => ({
    usePresenceStore: () => ({
        hasTypingUsers: vi.fn(() => false),
        getTypingUsersForChannel: vi.fn(() => ({ value: [] })),
        typingUsers: { value: new Map() },
    }),
}))

vi.mock('./capabilities', () => ({
    canCreateTeam: (role?: string | null) =>
        ['system_admin', 'org_admin', 'team_admin', 'admin'].includes(role ?? ''),
    canCreateChannel: (role?: string | null) =>
        ['system_admin', 'org_admin', 'team_admin', 'admin', 'member'].includes(role ?? ''),
    useCurrentTeamManagementPermission: () => ({
        canManageTeam: mocks.canManageTeam,
        currentTeamMembershipRole: { value: null, __v_isRef: true },
    }),
    useChannelManagementPermission: () => ({
        canManageChannel: mocks.canManageChannel,
        isLoading: mocks.isLoading,
    }),
    clearChannelPermissionCache: vi.fn(),
}))

describe('permission UI guardrails', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mocks.authStore.user = {
            id: 'user-1',
            role: 'member',
        }
        mocks.teamStore.currentTeamId = 'team-1'
        mocks.teamStore.currentTeam = {
            id: 'team-1',
            name: 'team-1',
            display_name: 'Team 1',
        }
        mocks.canManageTeam.value = false
        mocks.canManageChannel.value = false
        mocks.isLoading.value = false
    })

    it('hides the create team affordance from members', async () => {
        const TeamRail = (await import('../../components/layout/TeamRail.vue')).default

        const wrapper = mount(TeamRail, {
            global: {
                stubs: {
                    CreateTeamModal: true,
                },
            },
        })

        expect(wrapper.find('button[title="Create Team"]').exists()).toBe(false)
    })

    it('shows the create team affordance for org admins', async () => {
        mocks.authStore.user = {
            id: 'user-1',
            role: 'org_admin',
        }

        const TeamRail = (await import('../../components/layout/TeamRail.vue')).default

        const wrapper = mount(TeamRail, {
            global: {
                stubs: {
                    CreateTeamModal: true,
                },
            },
        })

        expect(wrapper.find('button[title="Create Team"]').exists()).toBe(true)
    })

    it('hides the team settings entry for users who cannot manage the current team', async () => {
        const ChannelSidebar = (await import('../../components/layout/ChannelSidebar.vue')).default

        const wrapper = mount(ChannelSidebar, {
            global: {
                stubs: {
                    CreateChannelModal: true,
                    DirectMessageModal: true,
                    TeamSettingsModal: true,
                    BrowseTeamsModal: true,
                    BrowseChannelsModal: true,
                    AddChannelMembersModal: true,
                    ChannelContextMenu: true,
                    RcAvatar: true,
                    teleport: true,
                },
                mocks: {
                    $router: {
                        push: vi.fn(),
                    },
                },
            },
        })

        await wrapper.find('.group').trigger('click')

        expect(wrapper.text()).not.toContain('Team Settings')
    })

    it('shows the team settings entry for team managers', async () => {
        mocks.canManageTeam.value = true

        const ChannelSidebar = (await import('../../components/layout/ChannelSidebar.vue')).default

        const wrapper = mount(ChannelSidebar, {
            global: {
                stubs: {
                    CreateChannelModal: true,
                    DirectMessageModal: true,
                    TeamSettingsModal: true,
                    BrowseTeamsModal: true,
                    BrowseChannelsModal: true,
                    AddChannelMembersModal: true,
                    ChannelContextMenu: true,
                    RcAvatar: true,
                    teleport: true,
                },
                mocks: {
                    $router: {
                        push: vi.fn(),
                    },
                },
            },
        })

        await wrapper.find('.group').trigger('click')

        expect(wrapper.text()).toContain('Team Settings')
    })

    it('hides create channel affordances for guests', async () => {
        mocks.authStore.user = {
            id: 'user-1',
            role: 'guest',
        }

        const ChannelSidebar = (await import('../../components/layout/ChannelSidebar.vue')).default

        const wrapper = mount(ChannelSidebar, {
            global: {
                stubs: {
                    CreateChannelModal: true,
                    DirectMessageModal: true,
                    TeamSettingsModal: true,
                    BrowseTeamsModal: true,
                    BrowseChannelsModal: true,
                    AddChannelMembersModal: true,
                    ChannelContextMenu: true,
                    RcAvatar: true,
                    teleport: true,
                },
                mocks: {
                    $router: {
                        push: vi.fn(),
                    },
                },
            },
        })

        expect(wrapper.text()).not.toContain('Create channel')
    })

    it('hides add-members actions in the channel context menu for unauthorized members', async () => {
        const ChannelContextMenu = (await import('../../components/channels/ChannelContextMenu.vue')).default

        const wrapper = mount(ChannelContextMenu, {
            props: {
                channelId: 'channel-1',
                channelName: 'town-square',
                channelType: 'public',
                creatorId: 'user-9',
                unreadCount: 0,
            },
        })

        expect(wrapper.text()).not.toContain('Add Members')
    })

    it('shows a locked state instead of edit controls in channel settings for unauthorized members', async () => {
        const ChannelSettingsModal = (await import('../../components/modals/ChannelSettingsModal.vue')).default

        const wrapper = mount(ChannelSettingsModal, {
            props: {
                isOpen: true,
                channel: {
                    id: 'channel-1',
                    team_id: 'team-1',
                    name: 'town-square',
                    display_name: 'Town Square',
                    channel_type: 'public',
                    creator_id: 'user-9',
                    created_at: new Date().toISOString(),
                },
            },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: {
                        template: '<button><slot /></button>',
                    },
                    BaseInput: {
                        template: '<input />',
                    },
                },
            },
        })

        expect(wrapper.text()).toContain('You do not have permission to manage this channel.')
        expect(wrapper.text()).not.toContain('Save Changes')
        expect(wrapper.text()).not.toContain('Add Member')
    })

    it('shows a locked state in team settings for unauthorized members', async () => {
        const TeamSettingsModal = (await import('../../components/modals/TeamSettingsModal.vue')).default

        const wrapper = mount(TeamSettingsModal, {
            props: {
                isOpen: true,
                team: {
                    id: 'team-1',
                    name: 'team-1',
                    display_name: 'Team 1',
                    description: null,
                    invite_id: null,
                    is_public: true,
                    allow_open_invite: true,
                    created_at: new Date().toISOString(),
                },
            },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: {
                        template: '<button><slot /></button>',
                    },
                    BaseInput: {
                        template: '<input />',
                    },
                },
            },
        })

        expect(wrapper.text()).toContain('You do not have permission to manage this team.')
        expect(wrapper.text()).not.toContain('Save Changes')
        expect(wrapper.text()).not.toContain('Add Member')
    })

    it('shows a locked state in create channel modal for unauthorized users', async () => {
        mocks.authStore.user = {
            id: 'user-1',
            role: 'guest',
        }

        const CreateChannelModal = (await import('../../components/modals/CreateChannelModal.vue')).default

        const wrapper = mount(CreateChannelModal, {
            props: {
                show: true,
            },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: {
                        template: '<button><slot /></button>',
                    },
                    BaseInput: {
                        template: '<input />',
                    },
                },
            },
        })

        expect(wrapper.text()).toContain('You do not have permission to create channels.')
        expect(wrapper.text()).not.toContain('Channel Type')
        expect(wrapper.text()).not.toContain('Channel Name')
    })
})
