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
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
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
