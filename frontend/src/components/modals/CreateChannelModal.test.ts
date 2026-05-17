// @vitest-environment jsdom

import { reactive, ref } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const teamStore = reactive({
    currentTeam: {
        id: 'team-1',
        name: 'engineering',
        display_name: 'Engineering',
    } as Record<string, any> | null,
})

const channelStore = reactive({
    createChannel: vi.fn<(_: any) => Promise<void>>(),
})

const authStore = reactive({
    user: {
        id: 'user-1',
        role: 'member',
    } as Record<string, any> | null,
})

let canCreateChannelResult = true

vi.mock('../../features/teams/stores/teamStore', () => ({
    useTeamStore: () => teamStore,
}))

vi.mock('../../features/channels/stores/channelStore', () => ({
    useChannelStore: () => channelStore,
}))

vi.mock('../../features/auth/stores/authStore', () => ({
    useAuthStore: () => authStore,
}))

vi.mock('../../features/permissions/capabilities', () => ({
    canCreateChannel: (role?: string | null) => canCreateChannelResult,
}))

describe('CreateChannelModal', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        teamStore.currentTeam = { id: 'team-1', name: 'engineering', display_name: 'Engineering' }
        authStore.user = { id: 'user-1', role: 'member' }
        canCreateChannelResult = true
    })

    it('emits close when backdrop is clicked', async () => {
        const CreateChannelModal = (await import('./CreateChannelModal.vue')).default

        const wrapper = mount(CreateChannelModal, {
            props: { show: true },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: true,
                    BaseInput: true,
                },
            },
        })

        const backdrop = wrapper.find('.bg-black\/50')
        expect(backdrop.exists()).toBe(true)

        await backdrop.trigger('click')
        await flushPromises()

        expect(wrapper.emitted('close')).toHaveLength(1)
    })

    it('shows permission denied message when user cannot create channels', async () => {
        canCreateChannelResult = false

        const CreateChannelModal = (await import('./CreateChannelModal.vue')).default

        const wrapper = mount(CreateChannelModal, {
            props: { show: true },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: true,
                    BaseInput: true,
                },
            },
        })

        expect(wrapper.text()).toContain('You do not have permission to create channels')
    })

    it('shows no team warning when there is no current team', async () => {
        teamStore.currentTeam = null

        const CreateChannelModal = (await import('./CreateChannelModal.vue')).default

        const wrapper = mount(CreateChannelModal, {
            props: { show: true },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: true,
                    BaseInput: true,
                },
            },
        })

        expect(wrapper.text()).toContain('Please create or select a team first')
    })

    it('submits form with correct payload and emits close', async () => {
        channelStore.createChannel.mockResolvedValue(undefined)

        const CreateChannelModal = (await import('./CreateChannelModal.vue')).default

        const wrapper = mount(CreateChannelModal, {
            props: { show: true },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: true,
                    BaseInput: false,
                },
            },
        })

        const nameInput = wrapper.find('input[placeholder="e.g., general"]')
        expect(nameInput.exists()).toBe(true)

        await nameInput.setValue('My Channel')
        await flushPromises()

        // Select private radio
        const privateRadio = wrapper.find('input[value="private"]')
        expect(privateRadio.exists()).toBe(true)
        await privateRadio.setValue(true)

        // Fill purpose
        const purposeTextarea = wrapper.find('textarea')
        expect(purposeTextarea.exists()).toBe(true)
        await purposeTextarea.setValue('A test channel')

        const form = wrapper.find('form')
        await form.trigger('submit.prevent')
        await flushPromises()

        expect(channelStore.createChannel).toHaveBeenCalledWith({
            team_id: 'team-1',
            name: 'my-channel',
            display_name: 'My Channel',
            channel_type: 'private',
            purpose: 'A test channel',
        })
        expect(wrapper.emitted('close')).toHaveLength(1)
    })

    it('shows error when channel name is empty', async () => {
        const CreateChannelModal = (await import('./CreateChannelModal.vue')).default

        const wrapper = mount(CreateChannelModal, {
            props: { show: true },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: true,
                    BaseInput: false,
                },
            },
        })

        const form = wrapper.find('form')
        await form.trigger('submit.prevent')
        await flushPromises()

        expect(channelStore.createChannel).not.toHaveBeenCalled()
        expect(wrapper.text()).toContain('Channel name is required')
    })

    it('emits close when cancel button is clicked', async () => {
        const CreateChannelModal = (await import('./CreateChannelModal.vue')).default

        const wrapper = mount(CreateChannelModal, {
            props: { show: true },
            global: {
                stubs: {
                    teleport: true,
                    BaseButton: true,
                    BaseInput: true,
                },
            },
        })

        // The cancel button is the first BaseButton stub with "Cancel" text
        const buttons = wrapper.findAllComponents({ name: 'BaseButton' })
        const cancelButton = buttons.find((b) => b.text().includes('Cancel'))
        expect(cancelButton).toBeDefined()

        await cancelButton!.trigger('click')
        await flushPromises()

        expect(wrapper.emitted('close')).toHaveLength(1)
    })
})
