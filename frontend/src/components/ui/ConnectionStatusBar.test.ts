// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

vi.mock('lucide-vue-next', () => ({
    WifiOff: { name: 'WifiOff', render: () => null },
    Loader2: { name: 'Loader2', render: () => null },
    AlertCircle: { name: 'AlertCircle', render: () => null },
}))

describe('ConnectionStatusBar', () => {
    it('renders reconnecting status without retry button', async () => {
        const ConnectionStatusBar = (await import('./ConnectionStatusBar.vue')).default
        const wrapper = mount(ConnectionStatusBar, {
            props: { status: 'reconnecting', nextRetryIn: 5 },
        })
        expect(wrapper.text()).toContain('Reconnecting...')
        expect(wrapper.find('button').exists()).toBe(false)
    })

    it('renders disconnected status with countdown and retry button', async () => {
        const ConnectionStatusBar = (await import('./ConnectionStatusBar.vue')).default
        const wrapper = mount(ConnectionStatusBar, {
            props: { status: 'disconnected', nextRetryIn: 12 },
        })
        expect(wrapper.text()).toContain('Connection lost. Retrying in 12s...')
        expect(wrapper.find('button').text()).toBe('Retry now')
    })

    it('renders failed status with retry button', async () => {
        const ConnectionStatusBar = (await import('./ConnectionStatusBar.vue')).default
        const wrapper = mount(ConnectionStatusBar, {
            props: { status: 'failed', nextRetryIn: 0 },
        })
        expect(wrapper.text()).toContain('Connection failed. Please reconnect.')
        expect(wrapper.find('button').text()).toBe('Retry now')
    })

    it('emits retry when retry button is clicked', async () => {
        const ConnectionStatusBar = (await import('./ConnectionStatusBar.vue')).default
        const wrapper = mount(ConnectionStatusBar, {
            props: { status: 'disconnected', nextRetryIn: 3 },
        })
        await wrapper.find('button').trigger('click')
        expect(wrapper.emitted('retry')).toHaveLength(1)
    })

    it('does not render for unrecognized status', async () => {
        const ConnectionStatusBar = (await import('./ConnectionStatusBar.vue')).default
        const wrapper = mount(ConnectionStatusBar, {
            props: { status: 'unknown' as any, nextRetryIn: 0 },
        })
        expect(wrapper.find('div').exists()).toBe(false)
    })
})
