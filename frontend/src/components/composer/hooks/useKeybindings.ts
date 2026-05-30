/**
 * Keyboard Shortcuts Hook
 * Mattermost-compatible keyboard bindings for the composer
 */

import { onMounted, onUnmounted } from 'vue'

export interface KeybindingOptions {
  // Send behavior
  enterToSend?: boolean // default: true
  ctrlEnterToSend?: boolean // alternative mode

  // Formatting shortcuts
  onBold?: () => void
  onItalic?: () => void
  onStrikethrough?: () => void
  onHeading?: () => void
  onLink?: () => void
  onInlineCode?: () => void
  onCodeBlock?: () => void
  onQuote?: () => void
  onBulletedList?: () => void
  onNumberedList?: () => void

  // Actions
  onSend?: () => void
  onNewline?: () => void
  onEscape?: () => void
  onToggleFormatting?: () => void
  onTogglePreview?: () => void
  onFocusComposer?: () => void // Ctrl/Cmd+K command menu trigger

  // Autocomplete
  onAutocompleteUp?: () => void
  onAutocompleteDown?: () => void
  onAutocompleteSelect?: () => void
  onAutocompleteClose?: () => void

  // File operations
  onAttachFile?: () => void

  // State
  isAutocompleteOpen?: boolean
  isPreviewOpen?: boolean
}

export function useKeybindings(options: KeybindingOptions) {
  const {
    enterToSend = true,
    ctrlEnterToSend = false,
    onBold,
    onItalic,
    onStrikethrough,
    onLink,
    onInlineCode,
    onCodeBlock,
    onQuote,
    onBulletedList,
    onNumberedList,
    onSend,
    onNewline,
    onEscape,
    onToggleFormatting,
    onTogglePreview,
    onFocusComposer,
    onAutocompleteUp,
    onAutocompleteDown,
    onAutocompleteSelect,
    onAutocompleteClose,
    onAttachFile,
    isAutocompleteOpen = false,
  } = options

  function handleKeydown(event: KeyboardEvent) {
    const { key, ctrlKey, metaKey, altKey, shiftKey } = event
    const isMod = ctrlKey || metaKey
    const isModShift = isMod && shiftKey

    // Autocomplete navigation (takes priority)
    if (isAutocompleteOpen) {
      if (key === 'ArrowUp') {
        event.preventDefault()
        onAutocompleteUp?.()
        return
      }
      if (key === 'ArrowDown') {
        event.preventDefault()
        onAutocompleteDown?.()
        return
      }
      if (key === 'Enter' || key === 'Tab') {
        event.preventDefault()
        onAutocompleteSelect?.()
        return
      }
      if (key === 'Escape') {
        event.preventDefault()
        onAutocompleteClose?.()
        return
      }
    }

    // Formatting shortcuts (Ctrl/Cmd + key)
    if (isMod) {
      switch (key.toLowerCase()) {
        case 'b':
          event.preventDefault()
          onBold?.()
          return
        case 'i':
          event.preventDefault()
          onItalic?.()
          return
        case 'k':
          event.preventDefault()
          onFocusComposer?.()
          return
        case 'u':
          // Mattermost uses Ctrl+U for underline but MM markdown doesn't support it
          // Could be used for something else
          break
        case 'e':
          // Ctrl+E for code (some editors use this)
          event.preventDefault()
          onInlineCode?.()
          return
      }
    }

    // Alt/Option shortcuts
    if (altKey && !isMod) {
      switch (key.toLowerCase()) {
        case 'c':
          event.preventDefault()
          onCodeBlock?.()
          return
        case 'q':
          event.preventDefault()
          onQuote?.()
          return
        case 'k':
          event.preventDefault()
          onLink?.()
          return
      }
    }

    // Ctrl/Cmd + Shift shortcuts
    if (isModShift) {
      switch (key.toLowerCase()) {
        case 'x':
          event.preventDefault()
          onStrikethrough?.()
          return
        case '7':
          event.preventDefault()
          onNumberedList?.()
          return
        case '8':
          event.preventDefault()
          onBulletedList?.()
          return
        case '9':
          event.preventDefault()
          onQuote?.()
          return
        case 't':
          event.preventDefault()
          onToggleFormatting?.()
          return
      }
    }

    // Ctrl/Cmd + Alt shortcuts
    if (isMod && altKey) {
      switch (key.toLowerCase()) {
        case 't':
          event.preventDefault()
          onToggleFormatting?.()
          return
        case 'p':
          event.preventDefault()
          onTogglePreview?.()
          return
      }
    }

    // Enter key handling
    if (key === 'Enter') {
      if (ctrlEnterToSend && isMod) {
        // Ctrl+Enter to send
        event.preventDefault()
        onSend?.()
        return
      }
      if (!ctrlEnterToSend && enterToSend && !shiftKey) {
        // Enter to send, Shift+Enter for newline
        event.preventDefault()
        onSend?.()
        return
      }
      if (ctrlEnterToSend && !isMod) {
        // In ctrl+enter mode, just Enter creates newline
        onNewline?.()
        return
      }
      if (!enterToSend && !shiftKey) {
        // If enterToSend is disabled, Enter creates newline
        onNewline?.()
        return
      }
      // Shift+Enter creates newline (default textarea behavior)
      onNewline?.()
      return
    }

    // Escape key
    if (key === 'Escape') {
      onEscape?.()
      return
    }

    // Attach file (Ctrl/Cmd + U)
    if (isMod && key.toLowerCase() === 'u') {
      event.preventDefault()
      onAttachFile?.()
      return
    }
  }

  // Global shortcuts (work even when composer not focused)
  function handleGlobalKeydown(event: KeyboardEvent) {
    const { key, ctrlKey, metaKey } = event
    const isMod = ctrlKey || metaKey

    // Focus composer / command menu shortcut
    if (isMod && key.toLowerCase() === 'k') {
      // Don't prevent default here - let the focused handler do it
      // This is just a fallback
    }
  }

  onMounted(() => {
    document.addEventListener('keydown', handleGlobalKeydown)
  })

  onUnmounted(() => {
    document.removeEventListener('keydown', handleGlobalKeydown)
  })

  return {
    handleKeydown,
    handleGlobalKeydown,
  }
}

/**
 * Simple composable for just send behavior
 */
export function useSendKeybinding(
  onSend: () => void,
  onNewline: () => void,
  options: { enterToSend?: boolean; ctrlEnterToSend?: boolean } = {}
) {
  const { enterToSend = true, ctrlEnterToSend = false } = options

  function handleKeydown(event: KeyboardEvent) {
    const { key, ctrlKey, metaKey, shiftKey } = event
    const isMod = ctrlKey || metaKey

    if (key === 'Enter') {
      if (ctrlEnterToSend && isMod) {
        event.preventDefault()
        onSend()
        return true
      }
      if (!ctrlEnterToSend && enterToSend && !shiftKey) {
        event.preventDefault()
        onSend()
        return true
      }
      if (ctrlEnterToSend && !isMod) {
        onNewline()
        return false
      }
      onNewline()
      return false
    }
    return false
  }

  return { handleKeydown }
}

/**
 * Get readable shortcut label for UI
 */
export function getShortcutLabel(action: string, isMac: boolean = false): string {
  const mod = isMac ? '⌘' : 'Ctrl'
  const shift = isMac ? '⇧' : 'Shift'
  const alt = isMac ? '⌥' : 'Alt'

  const shortcuts: Record<string, string> = {
    bold: `${mod}+B`,
    italic: `${mod}+I`,
    strikethrough: `${mod}+${shift}+X`,
    link: `${mod}+K`,
    inlineCode: `${mod}+E`,
    codeBlock: `${alt}+C`,
    quote: `${alt}+Q`,
    bulletedList: `${mod}+${shift}+8`,
    numberedList: `${mod}+${shift}+7`,
    heading: `${mod}+${shift}+H`,
    toggleFormatting: `${mod}+${alt}+T`,
    send: 'Enter',
    newline: 'Shift+Enter',
    attach: `${mod}+U`,
    focus: `${mod}+/`,
  }

  return shortcuts[action] || ''
}
