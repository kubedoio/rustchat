// @vitest-environment jsdom
import { computed, nextTick, ref } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useComposerKeyboard } from './useComposerKeyboard'

const textareaRef = ref<HTMLTextAreaElement | null>(null)
const onOpenCommandMenu = vi.fn()

function createKeyboard() {
  const textarea = document.createElement('textarea')
  textareaRef.value = textarea
  return useComposerKeyboard({
    textareaRef,
    content: ref(''),
    sendOnCtrlEnter: computed(() => false),
    showMentionMenu: computed(() => false),
    hasMentionSuggestions: computed(() => false),
    showEmojiAutocomplete: computed(() => false),
    hasEmojiSuggestions: computed(() => false),
    showChannelAutocomplete: computed(() => false),
    hasChannelSuggestions: computed(() => false),
    showCommandAutocomplete: computed(() => false),
    hasCommandSuggestions: computed(() => false),
    autocompleteRef: ref(null),
    onSend: vi.fn(),
    onOpenCommandMenu: onOpenCommandMenu,
    onCloseAllMenus: vi.fn(),
    onToggleFormatting: vi.fn(),
    onSaveDraft: vi.fn(),
    onAutoResize: vi.fn(),
  })
}

describe('useComposerKeyboard Cmd/Ctrl+K', () => {
  let textarea: HTMLTextAreaElement
  let documentListener: ReturnType<typeof vi.fn>

  beforeEach(() => {
    vi.clearAllMocks()
    const keyboard = createKeyboard()
    textarea = textareaRef.value!
    textarea.addEventListener('keydown', keyboard.handleKeydown)
    document.body.appendChild(textarea)
    documentListener = vi.fn()
    document.addEventListener('keydown', documentListener)
  })

  afterEach(() => {
    document.removeEventListener('keydown', documentListener)
    textarea.remove()
    textareaRef.value = null
  })

  function press(init: KeyboardEventInit) {
    return textarea.dispatchEvent(
      new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init })
    )
  }

  it('opens the command menu and stops the event from reaching the document', async () => {
    // Control: an unrelated key does reach the document listener.
    press({ key: 'a' })
    expect(documentListener).toHaveBeenCalledTimes(1)

    const notCanceled = press({ key: 'k', ctrlKey: true })
    await nextTick()

    expect(onOpenCommandMenu).toHaveBeenCalledTimes(1)
    // dispatchEvent returns false when preventDefault was called.
    expect(notCanceled).toBe(false)
    // The document-level QuickSwitcherHost must never see this keydown.
    expect(documentListener).toHaveBeenCalledTimes(1)
  })

  it('also stops Meta+K propagation', () => {
    press({ key: 'k', metaKey: true })

    expect(onOpenCommandMenu).toHaveBeenCalledTimes(1)
    expect(documentListener).not.toHaveBeenCalled()
  })

  it('leaves Shift+Cmd+K alone so it propagates to other handlers', () => {
    press({ key: 'k', ctrlKey: true, shiftKey: true })

    expect(onOpenCommandMenu).not.toHaveBeenCalled()
    expect(documentListener).toHaveBeenCalledTimes(1)
  })
})
