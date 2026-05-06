export function focusTrap(node: HTMLElement) {
  const focusableElements = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'

  function getFocusable() {
    return Array.from(node.querySelectorAll<HTMLElement>(focusableElements)).filter(
      (el) => !el.hasAttribute('disabled') && !el.getAttribute('aria-hidden')
    )
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return
    const focusable = getFocusable()
    if (focusable.length === 0) return
    const first = focusable[0]
    const last = focusable[focusable.length - 1]

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault()
      first.focus()
    }
  }

  node.addEventListener('keydown', handleKeydown)

  // Auto-focus first element
  requestAnimationFrame(() => {
    const focusable = getFocusable()
    if (focusable.length > 0) focusable[0].focus()
  })

  return {
    destroy() {
      node.removeEventListener('keydown', handleKeydown)
    }
  }
}
