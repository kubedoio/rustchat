<script setup lang="ts">
/**
 * Single global owner of the Cmd/Ctrl+K quick switcher.
 *
 * Mounted once from App.vue so the shortcut is available on every
 * authenticated page, including pages that do not render GlobalHeader
 * (admin, playbooks, profile settings).
 */
import { onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import QuickSwitcherModal from './QuickSwitcherModal.vue'
import { useQuickSwitcher } from '../../composables/useQuickSwitcher'
import type { QuickSwitcherItem } from '../../composables/useQuickSwitcher'

const router = useRouter()
const quickSwitcher = useQuickSwitcher()

function handleKeydown(e: KeyboardEvent) {
  // Shift+Cmd/Ctrl+K is intentionally not the switcher shortcut. Keep the
  // branch case-sensitive with respect to Shift so it cannot reopen here.
  if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === 'k') {
    e.preventDefault()
    quickSwitcher.toggle()
    return
  }

  if (e.key === 'Escape' && quickSwitcher.isOpen.value) {
    e.stopPropagation()
    quickSwitcher.close()
  }
}

function handleSelect(item: QuickSwitcherItem) {
  quickSwitcher.addRecentItem(item.id)
  router.push(item.to)
  quickSwitcher.close()
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  // Module-level `isOpen` outlives this component (logout, inactive tab,
  // route swaps). Reset it before detaching so the switcher cannot come back
  // open for the next identity/session.
  quickSwitcher.close()
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <QuickSwitcherModal
    :is-open="quickSwitcher.isOpen.value"
    :items="quickSwitcher.allItems.value"
    :recent-items="quickSwitcher.recentItems.value"
    @select="handleSelect"
    @close="quickSwitcher.close()"
  />
</template>
