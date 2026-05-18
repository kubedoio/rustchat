// @vitest-environment jsdom

import { reactive } from 'vue'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const presenceStore = reactive({
    getUserPresence: vi.fn<(_: string) => { value: any }>(),
})

const teamStore = reactive({
    members: [] as Array<{ user_id: string; presence?: string }>,
})

vi.mock('../../features/presence', () => ({
    usePresenceStore: () => presenceStore,
}))

vi.mock('../../features/teams/stores/teamStore', () => ({
    useTeamStore: () => teamStore,
}))

vi.mock('../../features/presence/presencePresentation', () => ({
    getPresencePresentation: vi.fn((status: string) => ({
        status: status || 'offline',
        label: status || 'Offline',
        icon: null,
        badgeClass: 'bg-gray-400',
    })),
    normalizePresenceStatus: vi.fn((status: string) => status || 'offline'),
}))

describe('RcAvatar', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        presenceStore.getUserPresence.mockReturnValue({ value: null })
        teamStore.members = []
    })

    it('renders initials from two-part username', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { username: 'John Doe' },
        })
        expect(wrapper.text()).toBe('JD')
    })

    it('renders first two chars for single-part username', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { username: 'Alice' },
        })
        expect(wrapper.text()).toBe('AL')
    })

    it('renders fallback question mark when no username', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {})
        expect(wrapper.text()).toBe('?')
    })

    it('computes consistent background color from username hash', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { username: 'Alice' },
        })
        const root = wrapper.find('div')
        const classes = root.classes()
        const colorClasses = [
            'bg-blue-500', 'bg-green-500', 'bg-red-500', 'bg-yellow-500',
            'bg-purple-500', 'bg-pink-500', 'bg-indigo-500', 'bg-teal-500',
            'bg-orange-500', 'bg-cyan-500',
        ]
        expect(colorClasses.some(c => classes.includes(c))).toBe(true)
    })

    it('renders avatar image when src is provided', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { src: 'https://example.com/avatar.png', username: 'Bob' },
        })
        expect(wrapper.find('img').exists()).toBe(true)
        expect(wrapper.find('img').attributes('src')).toBe('https://example.com/avatar.png')
    })

    it('falls back to initials when image errors', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { src: 'https://example.com/bad.png', username: 'Bob' },
        })
        expect(wrapper.find('img').exists()).toBe(true)
        await wrapper.find('img').trigger('error')
        expect(wrapper.find('img').exists()).toBe(false)
        expect(wrapper.text()).toBe('BO')
    })

    it('appends avatarVersion as query param when provided', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { src: 'https://example.com/avatar.png', avatarVersion: 3 },
        })
        expect(wrapper.find('img').attributes('src')).toBe('https://example.com/avatar.png?v=3')
    })

    it('shows presence dot when showPresence is true and user has presence', async () => {
        presenceStore.getUserPresence.mockReturnValue({ value: { presence: 'online' } })
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { userId: 'u1', username: 'Carl', showPresence: true },
        })
        expect(wrapper.find('div.absolute').exists()).toBe(true)
    })

    it('hides presence dot when showPresence is false', async () => {
        const RcAvatar = (await import('./RcAvatar.vue')).default
        const wrapper = mount(RcAvatar, {
            props: { userId: 'u1', username: 'Carl', showPresence: false },
        })
        expect(wrapper.find('div.absolute').exists()).toBe(false)
    })
})
