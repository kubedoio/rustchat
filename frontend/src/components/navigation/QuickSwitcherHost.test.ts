// @vitest-environment jsdom
import { defineComponent, h, nextTick, ref } from 'vue'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const quickSwitcher = {
  isOpen: ref(false),
  allItems: ref([]),
  recentItems: ref([]),
  toggle: vi.fn(() => {
    quickSwitcher.isOpen.value = !quickSwitcher.isOpen.value
  }),
  close: vi.fn(() => {
    quickSwitcher.isOpen.value = false
  }),
  addRecentItem: vi.fn(),
}

const routerPush = vi.fn()

vi.mock('../../composables/useQuickSwitcher', () => ({
  useQuickSwitcher: () => quickSwitcher,
}))

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
}))

const QuickSwitcherModalStub = defineComponent({
  name: 'QuickSwitcherModal',
  props: {
    isOpen: { type: Boolean, default: false },
    items: { type: Array, default: () => [] },
    recentItems: { type: Array, default: () => [] },
  },
  emits: ['close', 'select'],
  setup(props) {
    return () => (props.isOpen ? h('div', { role: 'dialog' }) : null)
  },
})

async function mountHost() {
  const QuickSwitcherHost = (await import('./QuickSwitcherHost.vue')).default
  return mount(QuickSwitcherHost, {
    global: {
      stubs: {
        QuickSwitcherModal: QuickSwitcherModalStub,
      },
    },
  })
}

function pressShortcut(init: KeyboardEventInit) {
  document.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, ...init }))
}

describe('QuickSwitcherHost', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    quickSwitcher.isOpen.value = false
  })

  it('opens exactly one dialog per Cmd+K keypress', async () => {
    const wrapper = await mountHost()

    pressShortcut({ key: 'k', ctrlKey: true })
    await nextTick()

    expect(quickSwitcher.toggle).toHaveBeenCalledTimes(1)
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(1)

    pressShortcut({ key: 'k', ctrlKey: true })
    await nextTick()

    expect(quickSwitcher.toggle).toHaveBeenCalledTimes(2)
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(0)

    wrapper.unmount()
  })

  it('supports Meta+K and closes on Escape', async () => {
    const wrapper = await mountHost()

    pressShortcut({ key: 'K', metaKey: true })
    await nextTick()
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(1)

    pressShortcut({ key: 'Escape' })
    await nextTick()
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(0)

    wrapper.unmount()
  })

  it('ignores Shift+Cmd+K', async () => {
    const wrapper = await mountHost()

    pressShortcut({ key: 'k', ctrlKey: true, shiftKey: true })
    await nextTick()

    expect(quickSwitcher.toggle).not.toHaveBeenCalled()
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(0)

    wrapper.unmount()
  })

  it('closes and unregisters on unmount so it cannot reopen after logout', async () => {
    quickSwitcher.isOpen.value = true
    const wrapper = await mountHost()
    await nextTick()
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(1)

    wrapper.unmount()

    expect(quickSwitcher.close).toHaveBeenCalledTimes(1)
    expect(quickSwitcher.isOpen.value).toBe(false)

    pressShortcut({ key: 'k', ctrlKey: true })

    expect(quickSwitcher.toggle).not.toHaveBeenCalled()
  })
})
