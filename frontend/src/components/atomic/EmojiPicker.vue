<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
    show: boolean
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
</script>

<template>
  <div 
    v-if="show"
    class="absolute bottom-full mb-2 left-1/2 -translate-x-1/2 bg-white dark:bg-gray-800 rounded-xl shadow-2xl border border-gray-200 dark:border-gray-700 w-80 overflow-hidden z-50"
  >
    <!-- Header -->
    <div class="p-2 border-b border-gray-200 dark:border-gray-700">
      <input
        v-model="searchQuery"
        type="text"
        placeholder="Search emoji..."
        class="w-full px-3 py-1.5 text-sm bg-gray-100 dark:bg-gray-700 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary text-gray-900 dark:text-white"
      />
    </div>
    
    <!-- Categories -->
    <div class="flex items-center px-2 py-1 border-b border-gray-100 dark:border-gray-700 space-x-1">
      <button
        v-for="cat in categories"
        :key="cat.id"
        @click="activeCategory = cat.id"
        class="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
        :class="activeCategory === cat.id ? 'bg-gray-200 dark:bg-gray-600' : ''"
      >
        {{ cat.name }}
      </button>
    </div>
    
    <!-- Emojis Grid -->
    <div class="p-2 grid grid-cols-8 gap-1 max-h-48 overflow-y-auto">
      <button
        v-for="emoji in filteredEmojis"
        :key="emoji"
        @click="selectEmoji(emoji)"
        class="p-1.5 text-xl hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
      >
        {{ emoji }}
      </button>
    </div>
    
    <!-- Empty State -->
    <div v-if="filteredEmojis.length === 0" class="p-4 text-center text-gray-500 text-sm">
      No emojis found
    </div>
  </div>
</template>
