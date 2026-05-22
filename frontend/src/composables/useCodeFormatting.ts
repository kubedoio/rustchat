import type { Ref } from 'vue'
import { wrapWithInlineCode, wrapWithCodeBlock } from '../utils/markdownFormatting'

export function useCodeFormatting(textareaRef: Ref<HTMLTextAreaElement | null>) {
  function wrapSelection(content: Ref<string>, wrapper: (text: string) => string): boolean {
    const textarea = textareaRef.value
    if (!textarea) return false

    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const selectedText = content.value.substring(start, end)

    if (selectedText) {
      const wrapped = wrapper(selectedText)
      content.value = content.value.substring(0, start) + wrapped + content.value.substring(end)

      // Restore cursor position after the wrapped text
      setTimeout(() => {
        textarea.focus()
        const newPosition = start + wrapped.length
        textarea.setSelectionRange(newPosition, newPosition)
      }, 0)

      return true
    }

    return false
  }

  function formatInlineCode(content: Ref<string>): boolean {
    return wrapSelection(content, wrapWithInlineCode)
  }

  function formatCodeBlock(content: Ref<string>): boolean {
    return wrapSelection(content, text => wrapWithCodeBlock(text, ''))
  }

  return {
    formatInlineCode,
    formatCodeBlock,
  }
}
