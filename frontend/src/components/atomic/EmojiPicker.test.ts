// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

describe('EmojiPicker', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('does not render when show is false', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: false },
      global: { stubs: { teleport: true } },
    })
    expect(wrapper.find('input').exists()).toBe(false)
  })

  it('renders emoji grid when show is true', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: true },
      global: { stubs: { teleport: true } },
    })
    expect(wrapper.find('input').exists()).toBe(true)
    expect(wrapper.findAll('button').length).toBeGreaterThan(0)
  })

  it('switches category and updates emoji grid', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: true },
      global: { stubs: { teleport: true } },
    })
    const categoryButtons = wrapper.findAll('.flex.items-center.space-x-1.border-b button')
    expect(categoryButtons.length).toBeGreaterThan(1)

    const firstGridText = wrapper.findAll('.grid button').map(b => b.text())
    await categoryButtons[1].trigger('click')
    const secondGridText = wrapper.findAll('.grid button').map(b => b.text())

    expect(secondGridText).not.toEqual(firstGridText)
  })

  it('filters emojis based on search query', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: true },
      global: { stubs: { teleport: true } },
    })
    const input = wrapper.find('input')
    await input.setValue('👍')
    const emojiButtons = wrapper.findAll('.grid button')
    expect(emojiButtons.length).toBeGreaterThan(0)
    expect(emojiButtons.some(b => b.text() === '👍')).toBe(true)
  })

  it('shows empty state when search returns no results', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: true },
      global: { stubs: { teleport: true } },
    })
    const input = wrapper.find('input')
    await input.setValue('zzz')
    expect(wrapper.text()).toContain('No emojis found')
  })

  it('emits select and close when an emoji is clicked', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: true },
      global: { stubs: { teleport: true } },
    })
    const emojiButton = wrapper.findAll('.grid button').find(b => b.text() === '👍')
    expect(emojiButton).toBeDefined()
    await emojiButton!.trigger('click')
    expect(wrapper.emitted('select')).toHaveLength(1)
    expect(wrapper.emitted('select')![0]).toEqual(['👍'])
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('emits close on Escape key', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: true },
      global: { stubs: { teleport: true } },
    })
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('emits close on click outside', async () => {
    const EmojiPicker = (await import('./EmojiPicker.vue')).default
    const wrapper = mount(EmojiPicker, {
      props: { show: true },
      global: { stubs: { teleport: true } },
    })
    document.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))
    expect(wrapper.emitted('close')).toHaveLength(1)
  })
})
