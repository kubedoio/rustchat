// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

describe('BaseButton', () => {
  it('renders default slot content', async () => {
    const BaseButton = (await import('./BaseButton.vue')).default
    const wrapper = mount(BaseButton, {
      slots: { default: 'Save' },
    })
    expect(wrapper.text()).toBe('Save')
  })

  it('defaults to type button and allows override', async () => {
    const BaseButton = (await import('./BaseButton.vue')).default
    const wrapperDefault = mount(BaseButton)
    expect(wrapperDefault.find('button').attributes('type')).toBe('button')

    const wrapperSubmit = mount(BaseButton, { props: { type: 'submit' } })
    expect(wrapperSubmit.find('button').attributes('type')).toBe('submit')
  })

  it('applies variant classes', async () => {
    const BaseButton = (await import('./BaseButton.vue')).default
    const primary = mount(BaseButton)
    expect(primary.find('button').classes()).toContain('bg-primary')

    const secondary = mount(BaseButton, { props: { variant: 'secondary' } })
    expect(secondary.find('button').classes()).toContain('bg-white')

    const danger = mount(BaseButton, { props: { variant: 'danger' } })
    expect(danger.find('button').classes()).toContain('bg-red-600')
  })

  it('applies block class when block is true', async () => {
    const BaseButton = (await import('./BaseButton.vue')).default
    const wrapper = mount(BaseButton, { props: { block: true } })
    expect(wrapper.find('button').classes()).toContain('w-full')
  })

  it('disables button and shows spinner when loading', async () => {
    const BaseButton = (await import('./BaseButton.vue')).default
    const wrapper = mount(BaseButton, { props: { loading: true } })
    const button = wrapper.find('button')
    expect(button.attributes('disabled')).toBeDefined()
    expect(button.find('svg').exists()).toBe(true)
  })

  it('forwards native click and suppresses it while loading', async () => {
    const BaseButton = (await import('./BaseButton.vue')).default
    const onClick = vi.fn()
    const wrapper = mount(BaseButton, {
      attrs: { onClick },
      slots: { default: 'Click' },
    })
    await wrapper.find('button').trigger('click')
    expect(onClick).toHaveBeenCalledOnce()

    const wrapperLoading = mount(BaseButton, {
      props: { loading: true },
      attrs: { onClick },
    })
    onClick.mockClear()
    await wrapperLoading.find('button').trigger('click')
    expect(onClick).not.toHaveBeenCalled()
  })
})
