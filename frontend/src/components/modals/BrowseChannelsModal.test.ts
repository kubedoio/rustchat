// @vitest-environment jsdom

import { reactive } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

type Team = {
    id: string
    name: string
    display_name: string
}

const channelStore = reactive({
    loading: false,
    joinableChannels: [] as Array<{
        id: string
        team_id?: string
        name: string
        display_name: string
        purpose?: string
    }>,
    fetchJoinableChannels: vi.fn<(_: string) => Promise<void>>(),
    joinChannel: vi.fn<(_: string) => Promise<void>>(),
    fetchChannels: vi.fn<(_: string) => Promise<void>>(),
    setJoinableChannels: vi.fn<(items: any[]) => void>((items) => {
        channelStore.joinableChannels = items;
    }),
})

const teamStore = reactive({
    currentTeamId: 'team-1' as string | null,
    currentTeam: null as Team | null,
})

const toast = {
    success: vi.fn(),
    error: vi.fn(),
}

vi.mock('@/features/channels/stores/channelStore', () => ({
    useChannelStore: () => channelStore,
}))

vi.mock('@/features/teams/stores/teamStore', () => ({
    useTeamStore: () => teamStore,
}))

vi.mock('../../composables/useToast', () => ({
    useToast: () => toast,
}))

describe('BrowseChannelsModal', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        channelStore.loading = false
        channelStore.joinableChannels = []
        teamStore.currentTeamId = 'team-1'
        teamStore.currentTeam = null
    })

    it('fetches joinable channels immediately when opened', async () => {
        const BrowseChannelsModal = (await import('./BrowseChannelsModal.vue')).default

        mount(BrowseChannelsModal, {
            props: { open: true },
            global: {
                stubs: {
                    teleport: true,
                },
            },
        })

        await flushPromises()

        expect(channelStore.fetchJoinableChannels).toHaveBeenCalledWith('team-1')
    })

    it('refreshes channel data after joining with the active team id', async () => {
        const BrowseChannelsModal = (await import('./BrowseChannelsModal.vue')).default

        const wrapper = mount(BrowseChannelsModal, {
            props: { open: true },
            global: {
                stubs: {
                    teleport: true,
                },
            },
        })

        // Set joinable channels AFTER mounting, so that the immediate watcher doesn't wipe them
        channelStore.joinableChannels = [
            {
                id: 'channel-1',
                team_id: 'team-1',
                name: 'town-square',
                display_name: 'Town Square',
                purpose: 'General chat',
            },
        ]

        await flushPromises()

        const joinButton = wrapper.findAll('button').find((button) => button.text().includes('Join'))
        expect(joinButton).toBeDefined()

        await joinButton!.trigger('click')
        await flushPromises()

        expect(channelStore.joinChannel).toHaveBeenCalledWith('channel-1')
        expect(channelStore.fetchChannels).toHaveBeenCalledWith('team-1')
        expect(channelStore.fetchJoinableChannels).toHaveBeenLastCalledWith('team-1')
        expect(toast.success).toHaveBeenCalled()
    })
})
