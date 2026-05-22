// @vitest-environment jsdom

import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'

vi.mock('lucide-vue-next', () => ({
  X: { name: 'X', render: () => null },
  CheckCircle: { name: 'CheckCircle', render: () => null },
  AlertCircle: { name: 'AlertCircle', render: () => null },
  Info: { name: 'Info', render: () => null },
}))

describe('ToastManager', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders no toasts initially', async () => {
    const ToastManager = (await import('./ToastManager.vue')).default
    const wrapper = mount(ToastManager)
    expect(wrapper.text()).toBe('')
  })

  it('adds and displays a toast via exposed add', async () => {
    const ToastManager = (await import('./ToastManager.vue')).default
    const wrapper = mount(ToastManager)
    ;(wrapper.vm as any).add({ type: 'info', title: 'Hello' })
    await nextTick()
    expect(wrapper.text()).toContain('Hello')
  })

  it('removes a toast via exposed remove', async () => {
    const ToastManager = (await import('./ToastManager.vue')).default
    const wrapper = mount(ToastManager)
    const id = (wrapper.vm as any).add({ type: 'info', title: 'Bye' })
    await nextTick()
    ;(wrapper.vm as any).remove(id)
    await nextTick()
    expect(wrapper.text()).not.toContain('Bye')
  })

  it('removes a toast when close button is clicked', async () => {
    const ToastManager = (await import('./ToastManager.vue')).default
    const wrapper = mount(ToastManager)
    ;(wrapper.vm as any).add({ type: 'info', title: 'Close me' })
    await nextTick()
    await wrapper.find('button').trigger('click')
    await nextTick()
    expect(wrapper.text()).not.toContain('Close me')
  })

  it('auto-removes toast after default duration', async () => {
    const ToastManager = (await import('./ToastManager.vue')).default
    const wrapper = mount(ToastManager)
    ;(wrapper.vm as any).add({ type: 'success', title: 'Timed' })
    await nextTick()
    expect(wrapper.text()).toContain('Timed')
    vi.advanceTimersByTime(5000)
    await nextTick()
    expect(wrapper.text()).not.toContain('Timed')
  })

  it('does not auto-remove when duration is 0', async () => {
    const ToastManager = (await import('./ToastManager.vue')).default
    const wrapper = mount(ToastManager)
    ;(wrapper.vm as any).add({ type: 'error', title: 'Stay', duration: 0 })
    await nextTick()
    vi.advanceTimersByTime(10000)
    await nextTick()
    expect(wrapper.text()).toContain('Stay')
  })
})
