// @vitest-environment jsdom

import { reactive, ref } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const authStore = reactive({
    user: {
        id: 'user-1',
        username: 'testuser',
        avatar_url: 'https://example.com/avatar.png',
        role: 'member',
        presence: 'online',
        status_emoji: '',
        status_text: '',
    } as Record<string, any>,
    logout: vi.fn(),
    updateStatus: vi.fn<(_: any) => Promise<void>>(),
})

const uiStore = reactive({
    isLhsOpen: false,
    toggleLhs: vi.fn(),
    openSettings: vi.fn(),
})

const configStore = reactive({
    siteConfig: {
        site_name: 'TestChat',
        logo_url: '',
    },
})

const teamStore = reactive({
    currentTeam: {
        id: 'team-1',
        name: 'engineering',
        display_name: 'Engineering',
    } as Record<string, any> | null,
})

const presenceStore = reactive({
    self: {
        presence: 'online',
    } as Record<string, any> | null,
    setSelfPresence: vi.fn(),
    updatePresenceFromEvent: vi.fn(),
})

const unreadStore = reactive({
    totalUnreadCount: 0,
})

const activityStore = reactive({
    unreadCount: 0,
})

const connectionStatus = ref('connected')

const quickSwitcher = {
    isOpen: ref(false),
    allItems: ref([]),
    recentItems: ref([]),
    toggle: vi.fn(),
    close: vi.fn(),
    addRecentItem: vi.fn(),
}

const routerPush = vi.fn()

vi.mock('../../features/auth/stores/authStore', () => ({
    useAuthStore: () => authStore,
}))

vi.mock('../../features/ui/stores/uiStore', () => ({
    useUIStore: () => uiStore,
}))

vi.mock('../../features/config/stores/configStore', () => ({
    useConfigStore: () => configStore,
}))

vi.mock('../../features/teams/stores/teamStore', () => ({
    useTeamStore: () => teamStore,
}))

vi.mock('../../features/presence', () => ({
    usePresenceStore: () => presenceStore,
}))

vi.mock('../../features/unreads/stores/unreadStore', () => ({
    useUnreadStore: () => unreadStore,
}))

vi.mock('../../features/activity/stores/activityStore', () => ({
    useActivityStore: () => activityStore,
}))

vi.mock('../../composables/useBreakpoints', () => ({
    useBreakpoints: () => ({ isMobile: ref(false) }),
}))

vi.mock('../../composables/useWebSocket', () => ({
    useWebSocket: () => ({ connectionStatus }),
}))

vi.mock('../../composables/useQuickSwitcher', () => ({
    useQuickSwitcher: () => quickSwitcher,
}))

vi.mock('../../features/activity/services/activityService', () => ({
    activityService: { openFeed: vi.fn() },
}))

vi.mock('vue-router', () => ({
    useRouter: () => ({ push: routerPush }),
}))

describe('GlobalHeader', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        authStore.user = {
            id: 'user-1',
            username: 'testuser',
            avatar_url: 'https://example.com/avatar.png',
            role: 'member',
            presence: 'online',
            status_emoji: '',
            status_text: '',
        }
        uiStore.isLhsOpen = false
        configStore.siteConfig = { site_name: 'TestChat', logo_url: '' }
        teamStore.currentTeam = { id: 'team-1', name: 'engineering', display_name: 'Engineering' }
        presenceStore.self = { presence: 'online' }
        unreadStore.totalUnreadCount = 0
        activityStore.unreadCount = 0
        connectionStatus.value = 'connected'
        quickSwitcher.isOpen.value = false
        routerPush.mockClear()
    })

    it('renders site name and team label', async () => {
        const GlobalHeader = (await import('./GlobalHeader.vue')).default

        const wrapper = mount(GlobalHeader, {
            global: {
                stubs: {
                    SearchModal: true,
                    QuickSwitcherModal: true,
                    SetStatusModal: true,
                    RcAvatar: true,
                    NotificationsDropdown: true,
                    ActivityFeed: true,
                },
            },
        })

        expect(wrapper.text()).toContain('TestChat')
        expect(wrapper.text()).toContain('Engineering')
    })

    it('renders username in user menu trigger', async () => {
        const GlobalHeader = (await import('./GlobalHeader.vue')).default

        const wrapper = mount(GlobalHeader, {
            global: {
                stubs: {
                    SearchModal: true,
                    QuickSwitcherModal: true,
                    SetStatusModal: true,
                    RcAvatar: true,
                    NotificationsDropdown: true,
                    ActivityFeed: true,
                },
            },
        })

        expect(wrapper.text()).toContain('testuser')
    })

    it('toggles user menu on click', async () => {
        const GlobalHeader = (await import('./GlobalHeader.vue')).default

        const wrapper = mount(GlobalHeader, {
            global: {
                stubs: {
                    SearchModal: true,
                    QuickSwitcherModal: true,
                    SetStatusModal: true,
                    RcAvatar: true,
                    NotificationsDropdown: true,
                    ActivityFeed: true,
                },
            },
        })

        const trigger = wrapper.find('[data-testid="user-menu-trigger"]')
        expect(trigger.exists()).toBe(true)

        await trigger.trigger('click')
        await flushPromises()

        expect(wrapper.text()).toContain('Online')
        expect(wrapper.text()).toContain('Profile')
        expect(wrapper.text()).toContain('Log out')
    })

    it('calls auth.logout when logout is clicked', async () => {
        const GlobalHeader = (await import('./GlobalHeader.vue')).default

        const wrapper = mount(GlobalHeader, {
            global: {
                stubs: {
                    SearchModal: true,
                    QuickSwitcherModal: true,
                    SetStatusModal: true,
                    RcAvatar: true,
                    NotificationsDropdown: true,
                    ActivityFeed: true,
                },
            },
        })

        await wrapper.find('[data-testid="user-menu-trigger"]').trigger('click')
        await flushPromises()

        const logoutButton = wrapper.findAll('button').find((b) => b.text().includes('Log out'))
        expect(logoutButton).toBeDefined()

        await logoutButton!.trigger('click')
        await flushPromises()

        expect(authStore.logout).toHaveBeenCalled()
    })

    it('opens search modal when search button is clicked', async () => {
        const GlobalHeader = (await import('./GlobalHeader.vue')).default

        const wrapper = mount(GlobalHeader, {
            global: {
                stubs: {
                    SearchModal: true,
                    QuickSwitcherModal: true,
                    SetStatusModal: true,
                    RcAvatar: true,
                    NotificationsDropdown: true,
                    ActivityFeed: true,
                },
            },
        })

        const searchButton = wrapper.findAll('button').find((b) => b.attributes('aria-label') === 'Search')
        expect(searchButton).toBeDefined()

        await searchButton!.trigger('click')
        await flushPromises()

        expect(wrapper.vm.showSearch).toBe(true)
    })

    it('shows admin console for admin users', async () => {
        authStore.user.role = 'system_admin'

        const GlobalHeader = (await import('./GlobalHeader.vue')).default

        const wrapper = mount(GlobalHeader, {
            global: {
                stubs: {
                    SearchModal: true,
                    QuickSwitcherModal: true,
                    SetStatusModal: true,
                    RcAvatar: true,
                    NotificationsDropdown: true,
                    ActivityFeed: true,
                },
            },
        })

        await wrapper.find('[data-testid="user-menu-trigger"]').trigger('click')
        await flushPromises()

        expect(wrapper.text()).toContain('Admin Console')
    })
})
