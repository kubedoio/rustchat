// @vitest-environment jsdom

import { reactive, ref } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const callsStore = reactive({
    currentChannelCall: vi.fn<(_: string) => any>(),
    startCall: vi.fn<(_: string) => Promise<void>>(),
    joinCall: vi.fn<(_: string) => Promise<void>>(),
    isInCall: false,
    currentCall: null as { channelId: string } | null,
    isExpanded: false,
})

const channelStore = reactive({
    leaveChannel: vi.fn<(_: string, __: string) => Promise<void>>(),
    channels: [] as Array<{ id: string }>,
    selectChannel: vi.fn(),
    clearChannels: vi.fn(),
})

const authStore = reactive({
    user: {
        id: 'user-1',
        username: 'testuser',
    } as Record<string, any> | null,
})

const uiStore = reactive({
    rhsView: null as string | null,
    toggleRhs: vi.fn(),
    toggleLhs: vi.fn(),
})

vi.mock('../../features/calls/stores/callStore', () => ({
    useCallsStore: () => callsStore,
}))

vi.mock('@/features/channels/stores/channelStore', () => ({
    useChannelStore: () => channelStore,
}))

vi.mock('../../features/auth/stores/authStore', () => ({
    useAuthStore: () => authStore,
}))

vi.mock('../../features/ui/stores/uiStore', () => ({
    useUIStore: () => uiStore,
}))

vi.mock('../../composables/useBreakpoints', () => ({
    useBreakpoints: () => ({ isMobile: ref(false) }),
}))

describe('ChannelHeader', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        callsStore.currentChannelCall.mockReturnValue(undefined)
        callsStore.isInCall = false
        callsStore.currentCall = null
        callsStore.isExpanded = false
        channelStore.channels = []
        uiStore.rhsView = null
    })

    it('renders channel name and public indicator', async () => {
        const ChannelHeader = (await import('./ChannelHeader.vue')).default

        const wrapper = mount(ChannelHeader, {
            props: {
                name: 'general',
                topic: 'General discussion',
                channelType: 'public',
                channelId: 'ch-1',
            },
        })

        expect(wrapper.text()).toContain('general')
        expect(wrapper.text()).toContain('Channel')
        expect(wrapper.text()).toContain('General discussion')
    })

    it('renders private channel indicator', async () => {
        const ChannelHeader = (await import('./ChannelHeader.vue')).default

        const wrapper = mount(ChannelHeader, {
            props: {
                name: 'secret',
                topic: '',
                channelType: 'private',
                channelId: 'ch-2',
            },
        })

        expect(wrapper.text()).toContain('secret')
        expect(wrapper.text()).toContain('Private channel')
        expect(wrapper.text()).toContain('No topic set yet')
    })

    it('toggles members RHS when members button is clicked', async () => {
        const ChannelHeader = (await import('./ChannelHeader.vue')).default

        const wrapper = mount(ChannelHeader, {
            props: {
                name: 'general',
                channelId: 'ch-1',
            },
        })

        const membersButton = wrapper.findAll('button').find((b) => b.attributes('aria-label') === 'Members')
        expect(membersButton).toBeDefined()

        await membersButton!.trigger('click')
        await flushPromises()

        expect(uiStore.toggleRhs).toHaveBeenCalledWith('members')
    })

    it('toggles search RHS when search button is clicked', async () => {
        const ChannelHeader = (await import('./ChannelHeader.vue')).default

        const wrapper = mount(ChannelHeader, {
            props: {
                name: 'general',
                channelId: 'ch-1',
            },
        })

        const searchButton = wrapper.findAll('button').find((b) => b.attributes('aria-label') === 'Search')
        expect(searchButton).toBeDefined()

        await searchButton!.trigger('click')
        await flushPromises()

        expect(uiStore.toggleRhs).toHaveBeenCalledWith('search')
    })

    it('starts a call when no active call exists', async () => {
        callsStore.currentChannelCall.mockReturnValue(undefined)
        callsStore.isInCall = false

        const ChannelHeader = (await import('./ChannelHeader.vue')).default

        const wrapper = mount(ChannelHeader, {
            props: {
                name: 'general',
                channelId: 'ch-1',
            },
        })

        const callButton = wrapper.findAll('button').find((b) => b.attributes('aria-label') === 'Start audio call')
        expect(callButton).toBeDefined()

        await callButton!.trigger('click')
        await flushPromises()

        expect(callsStore.startCall).toHaveBeenCalledWith('ch-1')
    })

    it('joins an existing call when active call exists and user is not in it', async () => {
        callsStore.currentChannelCall.mockReturnValue({ id: 'call-1' })
        callsStore.isInCall = false

        const ChannelHeader = (await import('./ChannelHeader.vue')).default

        const wrapper = mount(ChannelHeader, {
            props: {
                name: 'general',
                channelId: 'ch-1',
            },
        })

        const callButton = wrapper.findAll('button').find((b) => b.attributes('aria-label') === 'Join active call')
        expect(callButton).toBeDefined()

        await callButton!.trigger('click')
        await flushPromises()

        expect(callsStore.joinCall).toHaveBeenCalledWith('ch-1')
    })

    it('opens more options menu and shows channel details button', async () => {
        const ChannelHeader = (await import('./ChannelHeader.vue')).default

        const wrapper = mount(ChannelHeader, {
            props: {
                name: 'general',
                channelId: 'ch-1',
            },
        })

        const menuButton = wrapper.find('[data-testid="channel-header-menu"]')
        expect(menuButton.exists()).toBe(true)

        await menuButton.trigger('click')
        await flushPromises()

        const detailsButton = wrapper.find('[data-testid="channel-details-button"]')
        expect(detailsButton.exists()).toBe(true)
    })
})
