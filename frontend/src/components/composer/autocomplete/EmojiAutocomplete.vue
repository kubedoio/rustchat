<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { searchEmojis } from '../../../utils/emoji'

const props = defineProps<{
    query: string
    show: boolean
}>()

const emit = defineEmits<{
    select: [emojiName: string]
    close: []
}>()

const selectedIndex = ref(0)
const containerRef = ref<HTMLElement | null>(null)

// Filter emojis based on query
const filteredEmojis = computed(() => {
    if (!props.query) return []
    return searchEmojis(props.query, 8)
})

// Reset selection when query changes
watch(() => props.query, () => {
    selectedIndex.value = 0
})

watch(() => props.show, (isShown) => {
    if (isShown) {
        selectedIndex.value = 0
    }
})

// Navigation methods
function selectNext() {
    if (filteredEmojis.value.length === 0) return
    selectedIndex.value = (selectedIndex.value + 1) % filteredEmojis.value.length
    scrollToSelected()
}

function selectPrevious() {
    if (filteredEmojis.value.length === 0) return
    selectedIndex.value = (selectedIndex.value - 1 + filteredEmojis.value.length) % filteredEmojis.value.length
    scrollToSelected()
}

function selectCurrent() {
    if (filteredEmojis.value.length === 0) {
        emit('close')
        return
    }
    const emoji = filteredEmojis.value[selectedIndex.value]
    if (!emoji) {
        emit('close')
        return
    }
    emit('select', emoji.name)
}

function scrollToSelected() {
    const container = containerRef.value
    if (!container) return
    
    const selectedEl = container.children[selectedIndex.value] as HTMLElement
    if (!selectedEl) return

    const containerRect = container.getBoundingClientRect()
    const selectedRect = selectedEl.getBoundingClientRect()

    if (selectedRect.bottom > containerRect.bottom) {
        container.scrollTop += selectedRect.bottom - containerRect.bottom
    } else if (selectedRect.top < containerRect.top) {
        container.scrollTop -= containerRect.top - selectedRect.top
    }
}

// Expose methods for parent
defineExpose({
    selectNext,
    selectPrevious,
    selectCurrent
})
</script>

<template>
    <div 
        v-if="show && filteredEmojis.length > 0"
        ref="containerRef"
        class="absolute bottom-full left-0 mb-2 w-64 max-h-48 overflow-y-auto bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2xl z-[120]"
    >
        <div class="px-2 py-1.5 text-xs font-medium text-text-3 border-b border-border-1">
            Emoji matching "{{ query }}"
        </div>
        <button
            v-for="(emoji, index) in filteredEmojis"
            :key="emoji.name"
            @click="emit('select', emoji.name)"
            class="w-full flex items-center px-3 py-2 text-left transition-standard"
            :class="index === selectedIndex ? 'bg-bg-surface-2 text-text-1' : 'text-text-2 hover:bg-bg-surface-2 hover:text-text-1'"
        >
            <span class="text-xl mr-3">{{ emoji.char }}</span>
            <span class="text-sm">:{{ emoji.name }}:</span>
        </button>
    </div>
</template>
