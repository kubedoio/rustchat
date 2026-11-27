// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import QuickSwitcherModal from './QuickSwitcherModal.vue'

describe('QuickSwitcherModal', () => {
  it('exposes dialog semantics instead of relying on the host test stub', () => {
    const wrapper = mount(QuickSwitcherModal, {
      props: { isOpen: true, items: [], recentItems: [] },
    })

    const dialog = wrapper.find('[role="dialog"]')

    expect(dialog.exists()).toBe(true)
    expect(dialog.attributes('aria-modal')).toBe('true')
    expect(dialog.attributes('aria-label')).toBe('Quick switcher')
  })

  it('renders no dialog while closed', () => {
    const wrapper = mount(QuickSwitcherModal, {
      props: { isOpen: false, items: [], recentItems: [] },
    })

    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
  })
})
