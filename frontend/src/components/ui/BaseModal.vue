<script lang="ts">
// Module-level state shared by every BaseModal instance. In `<script setup>`
// top-level bindings are per-instance, so shared state lives in this block.

// Stable, unique id per mounted instance (the app is client-rendered only).
let modalInstanceCount = 0

// A module-level counter keeps nested modals safe: only the first open modal
// captures the original overflow value, and only the last close restores it.
let scrollLockCount = 0
let savedBodyOverflow = ''
</script>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, useSlots, watch } from 'vue'
import { X } from 'lucide-vue-next'

interface Props {
  modelValue: boolean
  /** Rendered as the default header and used as the dialog's accessible name. */
  title?: string
  /**
   * Explicit accessible-name override. When no default title header is rendered
   * (custom `header` slot or no `title`), the name resolves as
   * `ariaLabel ?? title ?? "Dialog"`.
   */
  ariaLabel?: string
  size?: 'sm' | 'md' | 'lg' | 'xl'
  hideClose?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  title: undefined,
  ariaLabel: undefined,
  size: 'md',
  hideClose: false,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'close'): void
}>()

const slots = useSlots()
const dialogRef = ref<HTMLElement | null>(null)
const titleId = `base-modal-title-${++modalInstanceCount}`

const hasDefaultTitle = computed(() => Boolean(props.title) && !slots.header)
const labelledBy = computed(() => (hasDefaultTitle.value ? titleId : undefined))
const ariaLabel = computed(() =>
  hasDefaultTitle.value ? undefined : props.ariaLabel?.trim() || props.title?.trim() || 'Dialog'
)

const sizeClasses: Record<string, string> = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-2xl',
}

// ---------------------------------------------------------------------------
// Scroll locking
// ---------------------------------------------------------------------------
let scrollLocked = false

function lockBodyScroll() {
  if (scrollLocked) return
  if (scrollLockCount === 0) {
    savedBodyOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
  }
  scrollLockCount += 1
  scrollLocked = true
}

function unlockBodyScroll() {
  if (!scrollLocked) return
  scrollLocked = false
  scrollLockCount = Math.max(0, scrollLockCount - 1)
  if (scrollLockCount === 0) {
    document.body.style.overflow = savedBodyOverflow
    savedBodyOverflow = ''
  }
}

// ---------------------------------------------------------------------------
// Focus management
// ---------------------------------------------------------------------------
const FOCUSABLE_SELECTOR = [
  'a[href]',
  'area[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  'iframe',
  'object',
  'embed',
  '[contenteditable]:not([contenteditable="false"])',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

let previouslyFocused: HTMLElement | null = null

function getFocusableElements(): HTMLElement[] {
  const root = dialogRef.value
  if (!root) return []

  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(element => {
    if (element.hasAttribute('disabled') || element.hasAttribute('hidden')) return false
    if (element.getAttribute('aria-hidden') === 'true') return false
    if (element.closest('[inert]')) return false
    // `checkVisibility` is not implemented by jsdom; guard on its presence
    // rather than using `getClientRects()`, which is always empty there.
    if (typeof element.checkVisibility === 'function' && !element.checkVisibility()) return false
    return true
  })
}

function focusInitialElement() {
  const root = dialogRef.value
  if (!root) return

  const focusable = getFocusableElements()
  const autofocus = focusable.find(element => element.hasAttribute('autofocus'))
  const target = autofocus ?? focusable[0] ?? root
  target.focus()
}

function restoreFocus() {
  const target = previouslyFocused
  previouslyFocused = null
  if (target?.isConnected && typeof target.focus === 'function') {
    target.focus()
  }
}

// ---------------------------------------------------------------------------
// Document-level keyboard listener
// ---------------------------------------------------------------------------
// The dialog container is not guaranteed to own focus (focus may sit on
// `document.body`), so Tab and Escape are handled at the document level while
// this modal is open. Nested modals: only the topmost dialog reacts.
let keydownListening = false

function isTopmostDialog(): boolean {
  const root = dialogRef.value
  if (!root) return false
  const dialogs = document.querySelectorAll<HTMLElement>('[role="dialog"][aria-modal="true"]')
  return dialogs.length > 0 && dialogs[dialogs.length - 1] === root
}

function attachKeydownListener() {
  if (keydownListening) return
  document.addEventListener('keydown', onKeydown)
  keydownListening = true
}

function detachKeydownListener() {
  if (!keydownListening) return
  document.removeEventListener('keydown', onKeydown)
  keydownListening = false
}

watch(
  () => props.modelValue,
  async isOpen => {
    if (isOpen) {
      previouslyFocused = document.activeElement as HTMLElement | null
      lockBodyScroll()
      attachKeydownListener()
      await nextTick()
      focusInitialElement()
      return
    }

    detachKeydownListener()
    unlockBodyScroll()
    await nextTick()
    restoreFocus()
  },
  { immediate: true }
)

onBeforeUnmount(() => {
  detachKeydownListener()
  unlockBodyScroll()
  // Only move focus back if focus is still inside this dialog; otherwise a
  // newer dialog (or the user) owns it.
  if (dialogRef.value?.contains(document.activeElement)) {
    restoreFocus()
  } else {
    previouslyFocused = null
  }
})

// ---------------------------------------------------------------------------
// Keyboard behavior
// ---------------------------------------------------------------------------
function close() {
  emit('update:modelValue', false)
  emit('close')
}

function onKeydown(event: KeyboardEvent) {
  // Respect child handlers (e.g. an editor consuming Tab or Escape) and IME
  // composition, where Escape only cancels the composition.
  if (event.defaultPrevented || event.isComposing) return

  // A later modal in the DOM is stacked on top of this one and owns keyboard
  // interaction; do not intercept its events.
  if (!isTopmostDialog()) return

  if (event.key === 'Escape') {
    // Escape always closes, even when `hideClose` hides the close button:
    // keyboard users still need a way out, and `hideClose` expresses a visual
    // preference rather than disabling dismissal. Stop the event so document
    // and window listeners do not also act on the dismissal.
    event.preventDefault()
    event.stopPropagation()
    event.stopImmediatePropagation()
    close()
    return
  }

  if (event.key !== 'Tab') return

  const root = dialogRef.value
  if (!root) return

  const focusable = getFocusableElements()

  if (focusable.length === 0) {
    event.preventDefault()
    root.focus()
    return
  }

  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  const active = document.activeElement
  const focusIsInside = root.contains(active)

  if (event.shiftKey) {
    if (!focusIsInside || active === first || active === root) {
      event.preventDefault()
      last.focus()
    }
    return
  }

  if (!focusIsInside || active === last) {
    event.preventDefault()
    first.focus()
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop: decorative, not exposed to assistive technology. -->
      <div aria-hidden="true" class="fixed inset-0 bg-black/50" @click="close"></div>

      <!-- Modal container -->
      <div
        ref="dialogRef"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="labelledBy"
        :aria-label="ariaLabel"
        tabindex="-1"
        :class="[
          'relative bg-bg-surface-1 rounded-xl shadow-xl w-full flex flex-col max-h-[90vh]',
          sizeClasses[size],
        ]"
      >
        <!-- Default header (title + close) -->
        <div
          v-if="title && !$slots.header"
          class="flex items-center justify-between px-5 py-4 border-b border-border-1"
        >
          <h3 :id="titleId" class="text-sm font-bold text-text-1">{{ title }}</h3>
          <button
            v-if="!hideClose"
            type="button"
            aria-label="Close dialog"
            class="p-1 hover:bg-bg-surface-2 rounded-lg transition-colors"
            @click="close"
          >
            <X class="w-4 h-4 text-text-3" />
          </button>
        </div>

        <!-- Custom header slot -->
        <div v-else-if="$slots.header">
          <slot name="header" />
        </div>

        <!-- Body -->
        <div class="flex-1 overflow-y-auto">
          <slot />
        </div>

        <!-- Footer -->
        <div v-if="$slots.footer" class="px-5 py-4 border-t border-border-1">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
