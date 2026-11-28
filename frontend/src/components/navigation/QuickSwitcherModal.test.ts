// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'
import { nextTick } from 'vue'
import QuickSwitcherModal from './QuickSwitcherModal.vue'

const cleanup: Array<() => void> = []

afterEach(() => {
  while (cleanup.length) cleanup.pop()?.()
})

function createTrigger(): HTMLButtonElement {
  const trigger = document.createElement('button')
  trigger.textContent = 'Open switcher'
  document.body.appendChild(trigger)
  trigger.focus()
  cleanup.push(() => trigger.remove())
  return trigger
}

async function mountOpen() {
  const trigger = createTrigger()
  const wrapper = mount(QuickSwitcherModal, {
    props: { isOpen: false, items: [], recentItems: [] },
    attachTo: document.body,
  })

  await wrapper.setProps({ isOpen: true })
  await flushPromises()

  cleanup.push(() => wrapper.unmount())
  return { trigger, wrapper, input: wrapper.find('input').element as HTMLInputElement }
}

describe('QuickSwitcherModal', () => {
  it('exposes dialog semantics instead of relying on the host test stub', () => {
    const wrapper = mount(QuickSwitcherModal, {
      props: { isOpen: true, items: [], recentItems: [] },
    })

    const dialog = wrapper.find('[role="dialog"]')

    expect(dialog.exists()).toBe(true)
    expect(dialog.attributes('aria-modal')).toBe('true')
    expect(dialog.attributes('aria-label')).toBe('Quick switcher')
    expect(wrapper.find('input').attributes('aria-label')).toBe('Search channels and teams')

    wrapper.unmount()
  })

  it('renders no dialog while closed', () => {
    const wrapper = mount(QuickSwitcherModal, {
      props: { isOpen: false, items: [], recentItems: [] },
    })

    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)

    wrapper.unmount()
  })

  it('moves focus to the input on open and restores the trigger on close', async () => {
    const trigger = createTrigger()
    const wrapper = mount(QuickSwitcherModal, {
      props: { isOpen: false, items: [], recentItems: [] },
      attachTo: document.body,
    })
    cleanup.push(() => wrapper.unmount())

    expect(document.activeElement).toBe(trigger)

    await wrapper.setProps({ isOpen: true })
    await flushPromises()

    const input = wrapper.find('input').element
    expect(document.activeElement).toBe(input)

    await wrapper.setProps({ isOpen: false })
    await nextTick()

    expect(document.activeElement).toBe(trigger)
  })

  it('traps Tab and Shift+Tab inside the dialog', async () => {
    const { input } = await mountOpen()

    for (const shiftKey of [false, true]) {
      input.focus()
      const event = new KeyboardEvent('keydown', {
        key: 'Tab',
        shiftKey,
        bubbles: true,
        cancelable: true,
      })
      input.dispatchEvent(event)

      expect(event.defaultPrevented).toBe(true)
      expect(document.activeElement).toBe(input)
    }
  })

  it('traps Tab when focus was lost to the document body', async () => {
    const { input } = await mountOpen()

    // Clicking non-focusable dialog chrome blurs the input to <body>; the
    // document-level guard must still pull focus back instead of letting Tab
    // walk into the page behind the overlay.
    ;(document.activeElement as HTMLElement | null)?.blur()
    expect(document.activeElement).toBe(document.body)

    const event = new KeyboardEvent('keydown', {
      key: 'Tab',
      bubbles: true,
      cancelable: true,
    })
    document.body.dispatchEvent(event)

    expect(event.defaultPrevented).toBe(true)
    expect(document.activeElement).toBe(input)
  })

  it('stops trapping Tab after the dialog closes', async () => {
    const { wrapper } = await mountOpen()

    await wrapper.setProps({ isOpen: false })
    await nextTick()

    const event = new KeyboardEvent('keydown', {
      key: 'Tab',
      bubbles: true,
      cancelable: true,
    })
    document.body.dispatchEvent(event)

    expect(event.defaultPrevented).toBe(false)
  })

  it('does not throw when unmounted while open and still restores focus', async () => {
    const trigger = createTrigger()
    const wrapper = mount(QuickSwitcherModal, {
      props: { isOpen: false, items: [], recentItems: [] },
      attachTo: document.body,
    })

    await wrapper.setProps({ isOpen: true })
    await flushPromises()
    expect(document.activeElement).toBe(wrapper.find('input').element)

    expect(() => wrapper.unmount()).not.toThrow()
    expect(document.activeElement).toBe(trigger)
  })
})
