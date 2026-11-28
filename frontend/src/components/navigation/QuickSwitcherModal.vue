<template>
  <Transition name="fade">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-[60] flex items-start justify-center pt-[20vh] bg-black/50"
      role="dialog"
      aria-modal="true"
      aria-label="Quick switcher"
      @click="emit('close')"
    >
      <div class="w-full max-w-lg mx-4 bg-white rounded-xl shadow-2xl overflow-hidden" @click.stop>
        <!-- Input -->
        <div class="flex items-center gap-3 px-4 py-3.5 border-b border-gray-200">
          <Search class="w-5 h-5 text-gray-400 flex-shrink-0" />
          <input
            ref="inputRef"
            v-model="query"
            type="text"
            placeholder="Jump to channel, team..."
            aria-label="Search channels and teams"
            class="flex-1 bg-transparent outline-none text-base placeholder-gray-400"
            @keydown="handleKeydown"
          />
          <kbd
            class="hidden sm:block px-2 py-0.5 text-xs text-gray-400 bg-gray-100 rounded font-mono"
            >ESC</kbd
          >
        </div>

        <!-- Results -->
        <div class="max-h-[360px] overflow-y-auto py-1">
          <div
            v-if="displayedItems.length === 0"
            class="px-4 py-8 text-center text-gray-400 text-sm"
          >
            <p v-if="query">No results for "{{ query }}"</p>
            <p v-else>Start typing to search channels and teams</p>
          </div>

          <template v-else>
            <div
              v-if="!query && displayedItems.length > 0"
              class="px-3 py-1.5 text-[11px] font-semibold text-gray-400 uppercase tracking-wide"
            >
              Recent
            </div>
            <QuickSwitcherItem
              v-for="(item, index) in displayedItems"
              :key="item.id"
              :item="item"
              :selected="selectedIndex === index"
              @click="selectItem(item)"
              @mouseenter="selectedIndex = index"
            />
          </template>
        </div>

        <!-- Footer -->
        <div
          class="flex items-center gap-4 px-4 py-2 text-xs text-gray-400 border-t border-gray-100"
        >
          <span class="flex items-center gap-1">
            <kbd class="px-1.5 py-0.5 bg-gray-100 rounded font-mono">↑↓</kbd>
            navigate
          </span>
          <span class="flex items-center gap-1">
            <kbd class="px-1.5 py-0.5 bg-gray-100 rounded font-mono">↵</kbd>
            select
          </span>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import { Search } from 'lucide-vue-next'
import { matchSorter } from 'match-sorter'
import QuickSwitcherItem from './QuickSwitcherItem.vue'
import type { QuickSwitcherItem as QSItem } from '../../composables/useQuickSwitcher'

const props = defineProps<{
  isOpen: boolean
  items: QSItem[]
  recentItems: QSItem[]
}>()

const emit = defineEmits<{
  close: []
  select: [item: QSItem]
}>()

const query = ref('')
const selectedIndex = ref(0)
const inputRef = ref<HTMLInputElement>()
let previouslyFocused: HTMLElement | null = null
let tabGuardListening = false

const filteredItems = computed((): QSItem[] => {
  if (!query.value) return []
  return matchSorter(props.items, query.value, {
    keys: ['name', 'subtitle'],
    threshold: matchSorter.rankings.CONTAINS,
  }).slice(0, 8)
})

const displayedItems = computed((): QSItem[] => {
  return query.value ? filteredItems.value : props.recentItems.slice(0, 6)
})

watch(query, () => {
  selectedIndex.value = 0
})

watch(
  () => props.isOpen,
  async open => {
    if (open) {
      previouslyFocused = getFocusedElement()
      query.value = ''
      selectedIndex.value = 0
      attachTabGuard()
      await nextTick()
      inputRef.value?.focus()
      return
    }

    detachTabGuard()
    restoreFocus()
  }
)

// The modal can be torn down while still open (logout, route swap, host
// unmount); hand focus back instead of leaking it to <body>.
onBeforeUnmount(() => {
  detachTabGuard()
  if (props.isOpen) restoreFocus()
  previouslyFocused = null
})

function getFocusedElement(): HTMLElement | null {
  const active = document.activeElement
  if (!(active instanceof HTMLElement) || active === document.body) return null
  return active
}

function restoreFocus() {
  const target = previouslyFocused
  previouslyFocused = null
  if (target?.isConnected) target.focus()
}

function selectItem(item: QSItem) {
  emit('select', item)
  emit('close')
}

// Result rows are non-focusable, so the search input is the only tab stop.
// Guard Tab at the document level (capture) rather than on the input: clicking
// non-focusable dialog chrome (footer, list padding) blurs the input to
// <body>, and an input-local handler would never see the following Tab.
function onDocumentKeydown(e: KeyboardEvent) {
  if (e.key !== 'Tab' || !props.isOpen) return
  e.preventDefault()
  inputRef.value?.focus()
}

function attachTabGuard() {
  if (tabGuardListening) return
  document.addEventListener('keydown', onDocumentKeydown, true)
  tabGuardListening = true
}

function detachTabGuard() {
  if (!tabGuardListening) return
  document.removeEventListener('keydown', onDocumentKeydown, true)
  tabGuardListening = false
}

function handleKeydown(e: KeyboardEvent) {
  const items = displayedItems.value
  if (!items.length) return

  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      selectedIndex.value = (selectedIndex.value + 1) % items.length
      break
    case 'ArrowUp':
      e.preventDefault()
      selectedIndex.value = (selectedIndex.value - 1 + items.length) % items.length
      break
    case 'Enter':
      e.preventDefault()
      if (items[selectedIndex.value]) {
        const item = items[selectedIndex.value]
        if (item) selectItem(item)
      }
      break
    case 'Escape':
      e.preventDefault()
      emit('close')
      break
  }
}
</script>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
