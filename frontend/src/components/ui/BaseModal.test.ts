// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent, h, nextTick, ref } from 'vue'
import type { Slot } from 'vue'
import BaseModal from './BaseModal.vue'

vi.mock('lucide-vue-next', () => ({
  X: { name: 'X', render: () => null },
}))

const wrappers: VueWrapper[] = []

function mountModal(
  props: Record<string, unknown> = {},
  slots: Record<string, Slot> = {}
): VueWrapper {
  const wrapper = mount(BaseModal, {
    props: { modelValue: false, ...props },
    slots,
    attachTo: document.body,
  })
  wrappers.push(wrapper)
  return wrapper
}

function getDialog(): HTMLElement {
  const dialog = document.querySelector<HTMLElement>('[role="dialog"]')
  if (!dialog) throw new Error('Dialog is not rendered')
  return dialog
}

function dispatchKeydown(
  target: Element,
  key: string,
  options: KeyboardEventInit = {}
): KeyboardEvent {
  const event = new KeyboardEvent('keydown', {
    key,
    bubbles: true,
    cancelable: true,
    ...options,
  })
  target.dispatchEvent(event)
  return event
}

beforeEach(() => {
  document.body.style.overflow = ''
})

afterEach(() => {
  while (wrappers.length > 0) {
    wrappers.pop()?.unmount()
  }
  document.body.innerHTML = ''
  document.body.style.overflow = ''
})

describe('BaseModal', () => {
  describe('rendering and dialog semantics', () => {
    it('renders nothing while closed', () => {
      mountModal({ modelValue: false, title: 'Settings' })

      expect(document.querySelector('[role="dialog"]')).toBeNull()
    })

    it('renders an accessible modal dialog when open', () => {
      mountModal({ modelValue: true, title: 'Settings' })

      const dialog = getDialog()
      expect(dialog.getAttribute('aria-modal')).toBe('true')
      expect(dialog.getAttribute('tabindex')).toBe('-1')
      expect(dialog.textContent).toContain('Settings')
    })

    it('labels the dialog with the rendered title', () => {
      mountModal({ modelValue: true, title: 'Settings' })

      const dialog = getDialog()
      const labelledBy = dialog.getAttribute('aria-labelledby')
      expect(labelledBy).toBeTruthy()
      expect(dialog.hasAttribute('aria-label')).toBe(false)

      const title = document.getElementById(labelledBy as string)
      expect(title).not.toBeNull()
      expect(title?.textContent).toBe('Settings')
    })

    it('generates a unique title id per instance', () => {
      mountModal({ modelValue: true, title: 'First' })
      const firstId = getDialog().getAttribute('aria-labelledby')

      mountModal({ modelValue: true, title: 'Second' })
      const secondId = document
        .querySelectorAll('[role="dialog"]')[1]
        ?.getAttribute('aria-labelledby')

      expect(firstId).toBeTruthy()
      expect(secondId).toBeTruthy()
      expect(secondId).not.toBe(firstId)
    })

    it('uses the ariaLabel prop when there is no title', () => {
      mountModal({ modelValue: true, ariaLabel: 'Confirm deletion' })

      const dialog = getDialog()
      expect(dialog.getAttribute('aria-label')).toBe('Confirm deletion')
      expect(dialog.hasAttribute('aria-labelledby')).toBe(false)
    })

    it('falls back to aria-label="Dialog" without a title or ariaLabel', () => {
      mountModal({ modelValue: true })

      expect(getDialog().getAttribute('aria-label')).toBe('Dialog')
    })

    it('never emits an empty aria-label for whitespace-only names', () => {
      mountModal(
        { modelValue: true, title: '', ariaLabel: '   ' },
        { header: () => h('h2', 'Header') }
      )

      expect(getDialog().getAttribute('aria-label')).toBe('Dialog')
    })

    it('uses the title as accessible name when a custom header replaces the title', () => {
      mountModal(
        { modelValue: true, title: 'Delete User' },
        { header: () => h('h2', 'Delete User') }
      )

      const dialog = getDialog()
      expect(dialog.getAttribute('aria-label')).toBe('Delete User')
      expect(dialog.hasAttribute('aria-labelledby')).toBe(false)
    })

    it('prefers ariaLabel over title with a custom header slot', () => {
      mountModal(
        { modelValue: true, title: 'Delete User', ariaLabel: 'Delete account' },
        { header: () => h('h2', 'Delete User') }
      )

      const dialog = getDialog()
      expect(dialog.getAttribute('aria-label')).toBe('Delete account')
      expect(dialog.hasAttribute('aria-labelledby')).toBe(false)
    })

    it('hides the backdrop from assistive technology', () => {
      mountModal({ modelValue: true, title: 'Settings' })

      const backdrop = getDialog().previousElementSibling
      expect(backdrop?.getAttribute('aria-hidden')).toBe('true')
    })

    it('gives the close button an accessible name and explicit button type', () => {
      mountModal({ modelValue: true, title: 'Settings' })

      const closeButton = document.querySelector<HTMLButtonElement>(
        'button[aria-label="Close dialog"]'
      )
      expect(closeButton).not.toBeNull()
      expect(closeButton?.getAttribute('type')).toBe('button')
    })
  })

  describe('dismissal', () => {
    it('emits close and update:modelValue on Escape', async () => {
      const wrapper = mountModal({ modelValue: true, title: 'Settings' })

      dispatchKeydown(getDialog(), 'Escape')
      await nextTick()

      expect(wrapper.emitted('close')).toHaveLength(1)
      expect(wrapper.emitted('update:modelValue')).toEqual([[false]])
    })

    it('closes on Escape even when the close button is hidden', async () => {
      const wrapper = mountModal({ modelValue: true, title: 'Settings', hideClose: true })

      dispatchKeydown(getDialog(), 'Escape')
      await nextTick()

      expect(wrapper.emitted('update:modelValue')).toEqual([[false]])
    })

    it('does not close on Escape when a child handler prevents it', async () => {
      const wrapper = mountModal(
        { modelValue: true, title: 'Settings' },
        {
          default: () =>
            h('input', {
              onKeydown: (event: KeyboardEvent) => {
                if (event.key === 'Escape') event.preventDefault()
              },
            }),
        }
      )

      dispatchKeydown(document.querySelector('input') as HTMLInputElement, 'Escape')
      await nextTick()

      expect(wrapper.emitted('update:modelValue')).toBeUndefined()
      expect(wrapper.emitted('close')).toBeUndefined()
    })

    it('does not close on Escape during IME composition', async () => {
      const wrapper = mountModal({ modelValue: true, title: 'Settings' })

      dispatchKeydown(getDialog(), 'Escape', { isComposing: true })
      await nextTick()

      expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    })

    it('closes when the close button is clicked', async () => {
      const wrapper = mountModal({ modelValue: true, title: 'Settings' })

      document.querySelector<HTMLButtonElement>('button[aria-label="Close dialog"]')?.click()
      await nextTick()

      expect(wrapper.emitted('update:modelValue')).toEqual([[false]])
      expect(wrapper.emitted('close')).toHaveLength(1)
    })

    it('closes when the backdrop is clicked', async () => {
      const wrapper = mountModal({ modelValue: true, title: 'Settings' })

      const backdrop = getDialog().previousElementSibling as HTMLElement
      backdrop.click()
      await nextTick()

      expect(wrapper.emitted('update:modelValue')).toEqual([[false]])
      expect(wrapper.emitted('close')).toHaveLength(1)
    })

    it('does not let Escape reach document-level listeners', async () => {
      const wrapper = mountModal({ modelValue: true, title: 'Settings' })
      const documentListener = vi.fn()
      document.addEventListener('keydown', documentListener)

      try {
        dispatchKeydown(getDialog(), 'Escape')
        await nextTick()

        expect(documentListener).not.toHaveBeenCalled()
        expect(wrapper.emitted('update:modelValue')).toEqual([[false]])
      } finally {
        document.removeEventListener('keydown', documentListener)
      }
    })
  })

  describe('focus management', () => {
    it('moves focus into the dialog on open and restores it on close', async () => {
      const trigger = document.createElement('button')
      trigger.textContent = 'Open'
      document.body.appendChild(trigger)
      trigger.focus()
      expect(document.activeElement).toBe(trigger)

      const wrapper = mountModal(
        { modelValue: false, title: 'Settings' },
        { default: () => h('input', { placeholder: 'Name' }) }
      )

      await wrapper.setProps({ modelValue: true })
      await nextTick()
      await nextTick()

      const dialog = getDialog()
      expect(dialog.contains(document.activeElement)).toBe(true)
      // First focusable in DOM order is the header close button.
      expect(document.activeElement?.getAttribute('aria-label')).toBe('Close dialog')

      await wrapper.setProps({ modelValue: false })
      await nextTick()
      await nextTick()

      expect(document.activeElement).toBe(trigger)
    })

    it('focuses the dialog container when it has no focusable children', async () => {
      mountModal({ modelValue: true, title: 'Settings', hideClose: true })
      await nextTick()
      await nextTick()

      expect(document.activeElement).toBe(getDialog())
    })

    it('restores focus after the dialog is unmounted while open', async () => {
      const trigger = document.createElement('button')
      document.body.appendChild(trigger)
      trigger.focus()

      const wrapper = mountModal({ modelValue: true, title: 'Settings' })
      await nextTick()
      await nextTick()
      expect(getDialog().contains(document.activeElement)).toBe(true)

      wrapper.unmount()
      wrappers.splice(wrappers.indexOf(wrapper), 1)

      expect(document.activeElement).toBe(trigger)
    })

    it('does not throw when the trigger was removed before focus restore', async () => {
      const trigger = document.createElement('button')
      document.body.appendChild(trigger)
      trigger.focus()

      const wrapper = mountModal({ modelValue: true, title: 'Settings' })
      await nextTick()
      await nextTick()
      expect(getDialog().contains(document.activeElement)).toBe(true)

      trigger.remove()

      await wrapper.setProps({ modelValue: false })
      await nextTick()
      await nextTick()

      expect(document.activeElement).not.toBe(trigger)
    })

    it('restores focus through a nested modal chain', async () => {
      const trigger = document.createElement('button')
      document.body.appendChild(trigger)
      trigger.focus()

      const outer = mountModal(
        { modelValue: true, title: 'Outer' },
        { default: () => h('button', { 'data-testid': 'outer-action' }, 'Outer action') }
      )
      await nextTick()
      await nextTick()
      expect(getDialog().contains(document.activeElement)).toBe(true)

      const inner = mountModal(
        { modelValue: true, title: 'Inner' },
        { default: () => h('button', { 'data-testid': 'inner-action' }, 'Inner action') }
      )
      await nextTick()
      await nextTick()

      const innerDialog = document.querySelectorAll<HTMLElement>('[role="dialog"]')[1]
      expect(innerDialog.contains(document.activeElement)).toBe(true)

      await inner.setProps({ modelValue: false })
      await nextTick()
      await nextTick()
      expect(getDialog().contains(document.activeElement)).toBe(true)

      await outer.setProps({ modelValue: false })
      await nextTick()
      await nextTick()
      expect(document.activeElement).toBe(trigger)
    })

    it('ignores autofocus on a disabled element and focuses the first focusable', async () => {
      mountModal(
        { modelValue: true, title: 'Settings', hideClose: true },
        {
          default: () =>
            h('div', [
              h('input', { autofocus: true, disabled: true }),
              h('button', { 'data-testid': 'fallback' }, 'Fallback'),
            ]),
        }
      )
      await nextTick()
      await nextTick()

      expect(document.activeElement?.getAttribute('data-testid')).toBe('fallback')
    })
  })

  describe('focus trap', () => {
    const firstButton = () => document.querySelector<HTMLButtonElement>('[data-testid="first"]')
    const lastButton = () => document.querySelector<HTMLButtonElement>('[data-testid="last"]')

    function mountWithButtons() {
      return mountModal(
        { modelValue: true, title: 'Settings', hideClose: true },
        {
          default: () =>
            h('div', [
              h('button', { 'data-testid': 'first' }, 'First'),
              h('button', { 'data-testid': 'last' }, 'Last'),
            ]),
        }
      )
    }

    it('wraps Tab from the last focusable element to the first', async () => {
      mountWithButtons()
      await nextTick()

      lastButton()?.focus()
      dispatchKeydown(lastButton() as HTMLButtonElement, 'Tab')
      await nextTick()

      expect(document.activeElement).toBe(firstButton())
    })

    it('wraps Shift+Tab from the first focusable element to the last', async () => {
      mountWithButtons()
      await nextTick()

      firstButton()?.focus()
      dispatchKeydown(firstButton() as HTMLButtonElement, 'Tab', { shiftKey: true })
      await nextTick()

      expect(document.activeElement).toBe(lastButton())
    })

    it('does not intercept Tab between elements inside the dialog', async () => {
      mountWithButtons()
      await nextTick()

      firstButton()?.focus()
      const event = dispatchKeydown(firstButton() as HTMLButtonElement, 'Tab')
      await nextTick()

      expect(event.defaultPrevented).toBe(false)
      expect(document.activeElement).toBe(firstButton())
    })

    it('keeps focus on the container when there are no focusable elements', async () => {
      mountModal({ modelValue: true, title: 'Settings', hideClose: true })
      await nextTick()

      const dialog = getDialog()
      dialog.focus()
      const event = dispatchKeydown(dialog, 'Tab')

      expect(event.defaultPrevented).toBe(true)
      expect(document.activeElement).toBe(dialog)
    })

    it('intercepts Tab when focus is outside the dialog', async () => {
      mountWithButtons()
      await nextTick()

      const outside = document.createElement('button')
      outside.textContent = 'Outside'
      document.body.appendChild(outside)
      outside.focus()
      expect(document.activeElement).toBe(outside)

      const event = dispatchKeydown(document.body, 'Tab')
      await nextTick()

      expect(event.defaultPrevented).toBe(true)
      expect(document.activeElement).toBe(firstButton())
    })
  })

  describe('background isolation', () => {
    it('locks body scroll while open and restores the previous value on close', async () => {
      document.body.style.overflow = 'scroll'
      const wrapper = mountModal({ modelValue: true, title: 'Settings' })

      expect(document.body.style.overflow).toBe('hidden')

      await wrapper.setProps({ modelValue: false })
      await nextTick()
      await nextTick()

      expect(document.body.style.overflow).toBe('scroll')
    })

    it('restores body scroll when unmounted while open', () => {
      document.body.style.overflow = 'auto'
      const wrapper = mountModal({ modelValue: true, title: 'Settings' })
      expect(document.body.style.overflow).toBe('hidden')

      wrapper.unmount()
      wrappers.splice(wrappers.indexOf(wrapper), 1)

      expect(document.body.style.overflow).toBe('auto')
    })

    it('keeps the scroll lock until the last nested modal closes', async () => {
      const outer = mountModal({ modelValue: true, title: 'Outer' })
      expect(document.body.style.overflow).toBe('hidden')

      const inner = mountModal({ modelValue: true, title: 'Inner' })
      expect(document.body.style.overflow).toBe('hidden')

      await outer.setProps({ modelValue: false })
      await nextTick()
      await nextTick()
      expect(document.body.style.overflow).toBe('hidden')

      await inner.setProps({ modelValue: false })
      await nextTick()
      await nextTick()
      expect(document.body.style.overflow).toBe('')
    })
  })

  describe('public API', () => {
    it('renders header, default, and footer slots', () => {
      mountModal(
        { modelValue: true },
        {
          header: () => h('h2', { id: 'custom-header' }, 'Header'),
          default: () => h('p', 'Body'),
          footer: () => h('button', 'Save'),
        }
      )

      expect(document.body.textContent).toContain('Header')
      expect(document.body.textContent).toContain('Body')
      expect(document.body.textContent).toContain('Save')
    })

    it('keeps the modelValue controlled by the parent', async () => {
      const open = ref(false)
      const Host = defineComponent({
        setup() {
          return () =>
            h(BaseModal, {
              modelValue: open.value,
              title: 'Controlled',
              'onUpdate:modelValue': (value: boolean) => {
                open.value = value
              },
            })
        },
      })

      const wrapper = mount(Host, { attachTo: document.body })
      wrappers.push(wrapper)

      expect(document.querySelector('[role="dialog"]')).toBeNull()

      // Opening is parent-driven.
      open.value = true
      await nextTick()
      await nextTick()
      expect(document.querySelector('[role="dialog"]')).not.toBeNull()

      dispatchKeydown(getDialog(), 'Escape')
      await nextTick()
      await nextTick()

      expect(document.querySelector('[role="dialog"]')).toBeNull()
    })
  })
})
