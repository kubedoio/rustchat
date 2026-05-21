import { type ComputedRef, type Ref } from 'vue'

export interface AutocompleteRef {
  selectPrevious: () => void
  selectNext: () => void
  selectCurrent: () => void
}

interface UseComposerKeyboardOptions {
  textareaRef: Ref<HTMLTextAreaElement | null>
  content: Ref<string>
  sendOnCtrlEnter: ComputedRef<boolean>
  showMentionMenu: ComputedRef<boolean>
  hasMentionSuggestions: ComputedRef<boolean>
  showEmojiAutocomplete: ComputedRef<boolean>
  hasEmojiSuggestions: ComputedRef<boolean>
  showChannelAutocomplete: ComputedRef<boolean>
  hasChannelSuggestions: ComputedRef<boolean>
  showCommandAutocomplete: ComputedRef<boolean>
  hasCommandSuggestions: ComputedRef<boolean>
  autocompleteRef: Ref<AutocompleteRef | null>
  onSend: () => void
  onOpenCommandMenu: () => void
  onCloseAllMenus: () => void
  onToggleFormatting: () => void
  onSaveDraft: () => void
  onAutoResize: () => void
}

export function useComposerKeyboard(options: UseComposerKeyboardOptions) {
  function applyFormat(type: string) {
    const textarea = options.textareaRef.value
    if (!textarea) return
    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const selectedText = options.content.value.substring(start, end)
    let before = ''
    let after = ''
    let prefix = ''
    switch (type) {
      case 'bold':
        before = '**'
        after = '**'
        break
      case 'italic':
        before = '*'
        after = '*'
        break
      case 'strike':
        before = '~~'
        after = '~~'
        break
      case 'heading':
        prefix = '### '
        break
      case 'code':
        before = '`'
        after = '`'
        break
      case 'codeblock':
        before = '```\n'
        after = '\n```'
        break
      case 'link':
        before = '['
        after = '](url)'
        break
      case 'quote':
        prefix = '> '
        break
      case 'bullet':
        prefix = '- '
        break
      case 'numbered':
        prefix = '1. '
        break
    }
    if (prefix) {
      const lineStart = options.content.value.lastIndexOf('\n', start - 1) + 1
      options.content.value =
        options.content.value.substring(0, lineStart) +
        prefix +
        options.content.value.substring(lineStart)
    } else {
      options.content.value =
        options.content.value.substring(0, start) +
        before +
        selectedText +
        after +
        options.content.value.substring(end)
      textarea.focus()
      setTimeout(() => {
        textarea.setSelectionRange(start + before.length, end + before.length)
      }, 0)
    }
    options.onSaveDraft()
    options.onAutoResize()
  }

  function handleKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === 'k') {
      event.preventDefault()
      options.onOpenCommandMenu()
      return
    }

    if (options.hasCommandSuggestions.value) {
      if (event.key === 'ArrowUp') {
        event.preventDefault()
        options.autocompleteRef.value?.selectPrevious()
        return
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        options.autocompleteRef.value?.selectNext()
        return
      }
      if (event.key === 'Enter' || event.key === 'Tab') {
        event.preventDefault()
        options.autocompleteRef.value?.selectCurrent()
        return
      }
    }

    if (options.hasEmojiSuggestions.value) {
      if (event.key === 'ArrowUp') {
        event.preventDefault()
        options.autocompleteRef.value?.selectPrevious()
        return
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        options.autocompleteRef.value?.selectNext()
        return
      }
      if (event.key === 'Enter' || event.key === 'Tab') {
        event.preventDefault()
        options.autocompleteRef.value?.selectCurrent()
        return
      }
    }

    if (options.hasChannelSuggestions.value) {
      if (event.key === 'ArrowUp') {
        event.preventDefault()
        options.autocompleteRef.value?.selectPrevious()
        return
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        options.autocompleteRef.value?.selectNext()
        return
      }
      if (event.key === 'Enter' || event.key === 'Tab') {
        event.preventDefault()
        options.autocompleteRef.value?.selectCurrent()
        return
      }
    }

    if (options.showMentionMenu.value && options.hasMentionSuggestions.value) {
      if (
        event.key === 'ArrowUp' ||
        event.key === 'ArrowDown' ||
        event.key === 'Enter' ||
        event.key === 'Tab'
      ) {
        event.preventDefault()
        return
      }
    }

    if (event.key === 'Escape') {
      options.onCloseAllMenus()
      event.preventDefault()
      return
    }

    if ((event.ctrlKey || event.metaKey) && !event.altKey) {
      if (event.key.toLowerCase() === 'b') {
        event.preventDefault()
        applyFormat('bold')
        return
      }

      if (event.key.toLowerCase() === 'i') {
        event.preventDefault()
        applyFormat('italic')
        return
      }

      if (event.shiftKey && event.key.toLowerCase() === 'x') {
        event.preventDefault()
        applyFormat('strike')
        return
      }
    }

    if ((event.ctrlKey || event.metaKey) && event.shiftKey) {
      if (event.key === '7') {
        event.preventDefault()
        applyFormat('numbered')
        return
      }
      if (event.key === '8') {
        event.preventDefault()
        applyFormat('bullet')
        return
      }
    }

    if (event.key !== 'Enter' || event.shiftKey) return

    if (options.sendOnCtrlEnter.value) {
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault()
        options.onSend()
      }
      return
    }

    if (!event.ctrlKey && !event.metaKey) {
      event.preventDefault()
      options.onSend()
    }
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.altKey && event.key.toLowerCase() === 't') {
      event.preventDefault()
      options.onToggleFormatting()
    }
  }

  return { handleKeydown, handleGlobalKeydown }
}
