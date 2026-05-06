<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'

const props = defineProps<{
    show: boolean
    anchorEl?: HTMLElement | null
}>()

const emit = defineEmits<{
    (e: 'select', emoji: string): void
    (e: 'close'): void
}>()

const categories = [
    { id: 'frequent', name: '👍', emojis: ['👍', '❤️', '😂', '🎉', '🤔', '👀', '🙌', '💯'] },
    { id: 'smileys', name: '😀', emojis: ['😀', '😃', '😄', '😁', '😆', '😅', '🤣', '😂', '🙂', '😊', '😇', '🥰', '😍', '🤩', '😘', '😗', '😚', '😋', '😛', '😜', '🤪', '😝', '🤑', '🤗', '🤭', '🤫', '🤔', '🤐', '🤨', '😐', '😑', '😶', '😏', '😒', '🙄', '😬', '🤥', '😌', '😔', '😪', '🤤', '😴', '😷'] },
    { id: 'gestures', name: '👋', emojis: ['👋', '🤚', '🖐️', '✋', '🖖', '👌', '🤌', '🤏', '✌️', '🤞', '🤟', '🤘', '🤙', '👈', '👉', '👆', '🖕', '👇', '☝️', '👍', '👎', '✊', '👊', '🤛', '🤜', '👏', '🙌', '👐', '🤲', '🤝', '🙏'] },
    { id: 'hearts', name: '❤️', emojis: ['❤️', '🧡', '💛', '💚', '💙', '💜', '🖤', '🤍', '🤎', '💔', '❤️‍🔥', '❤️‍🩹', '❣️', '💕', '💞', '💓', '💗', '💖', '💘', '💝'] },
    { id: 'objects', name: '💡', emojis: ['⭐', '🌟', '✨', '⚡', '🔥', '💫', '🎯', '🎪', '🎨', '🎬', '🎤', '🎧', '🎵', '🎶', '🎹', '🥁', '🎸', '🎺', '🎻', '🎲', '🎮', '🕹️', '🎰', '🧩'] },
    { id: 'symbols', name: '✅', emojis: ['✅', '❌', '❓', '❗', '💯', '🔴', '🟠', '🟡', '🟢', '🔵', '🟣', '⚫', '⚪', '🟤', '🔶', '🔷', '🔸', '🔹', '▪️', '▫️', '◾', '◽', '◼️', '◻️', '⬛', '⬜'] },
]

const activeCategory = ref('frequent')
const searchQuery = ref('')
const pickerRef = ref<HTMLElement | null>(null)
const pickerStyle = ref<Record<string, string>>({})

const filteredEmojis = computed(() => {
    const cat = categories.find(c => c.id === activeCategory.value)
    if (!cat) return []
    
    if (searchQuery.value) {
        return cat.emojis.filter(e => e.includes(searchQuery.value))
    }
    return cat.emojis
})

function selectEmoji(emoji: string) {
    emit('select', emoji)
    emit('close')
}

function updatePosition() {
    if (!props.show || !props.anchorEl || !pickerRef.value) return

    const anchorRect = props.anchorEl.getBoundingClientRect()
    const panelRect = pickerRef.value.getBoundingClientRect()
    const viewportPadding = 8
    const gap = 10

    let left = anchorRect.right - panelRect.width
    left = Math.max(viewportPadding, Math.min(left, window.innerWidth - panelRect.width - viewportPadding))

    let top = anchorRect.top - panelRect.height - gap
    if (top < viewportPadding) {
        top = anchorRect.bottom + gap
    }
    top = Math.max(viewportPadding, Math.min(top, window.innerHeight - panelRect.height - viewportPadding))

    pickerStyle.value = {
        left: `${Math.round(left)}px`,
        top: `${Math.round(top)}px`,
    }
}

function handlePointerDown(event: MouseEvent) {
    if (!props.show) return
    const target = event.target as Node | null
    if (!target) return
    if (pickerRef.value?.contains(target)) return
    if (props.anchorEl?.contains(target)) return
    emit('close')
}

function handleKeyDown(event: KeyboardEvent) {
    if (props.show && event.key === 'Escape') {
        emit('close')
    }
}

watch(
    () => props.show,
    async (visible) => {
        if (!visible) return
        await nextTick()
        updatePosition()
    }
)

watch(
    () => props.anchorEl,
    () => {
        if (!props.show) return
        void nextTick(updatePosition)
    }
)

onMounted(() => {
    window.addEventListener('resize', updatePosition)
    window.addEventListener('scroll', updatePosition, true)
    document.addEventListener('mousedown', handlePointerDown)
    document.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
    window.removeEventListener('resize', updatePosition)
    window.removeEventListener('scroll', updatePosition, true)
    document.removeEventListener('mousedown', handlePointerDown)
    document.removeEventListener('keydown', handleKeyDown)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show"
      ref="pickerRef"
      :style="pickerStyle"
      class="fixed z-[9999] w-[22rem] max-w-[calc(100vw-1rem)] overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 text-text-1 shadow-2xl animate-fade-in"
    >
      <!-- Header -->
      <div class="border-b border-border-1 p-2">
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Search emoji..."
          class="w-full rounded-r-2 border border-border-1 bg-bg-surface-2 px-3 py-1.5 text-sm text-text-1 placeholder:text-text-3 focus:border-brand focus:outline-none focus:ring-2 focus:ring-brand/15"
        />
      </div>

      <!-- Categories -->
      <div class="flex items-center space-x-1 border-b border-border-1 px-2 py-1">
        <button
          v-for="cat in categories"
          :key="cat.id"
          @click="activeCategory = cat.id"
          class="rounded-r-1 p-1.5 text-lg transition-standard hover:bg-bg-surface-2"
          :class="activeCategory === cat.id ? 'bg-bg-surface-2 text-brand' : 'text-text-2'"
        >
          {{ cat.name }}
        </button>
      </div>

      <!-- Emojis Grid -->
      <div class="p-2 grid grid-cols-8 gap-1 max-h-56 overflow-y-auto">
        <button
          v-for="emoji in filteredEmojis"
          :key="emoji"
          @click="selectEmoji(emoji)"
          class="rounded-r-1 p-1.5 text-xl transition-standard hover:bg-bg-surface-2"
        >
          {{ emoji }}
        </button>
      </div>

      <!-- Empty State -->
      <div v-if="filteredEmojis.length === 0" class="p-4 text-center text-sm text-text-3">
        No emojis found
      </div>
    </div>
  </Teleport>
</template>
