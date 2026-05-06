<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Command } from 'lucide-vue-next'

const props = defineProps<{
    query: string
    show: boolean
}>()

const emit = defineEmits<{
    select: [command: string]
    close: []
}>()

type CommandItem = {
    command: string
    description: string
}

const commands: CommandItem[] = [
    { command: 'call start', description: 'Start a call in this channel' },
    { command: 'call join', description: 'Join the active channel call' },
    { command: 'call leave', description: 'Leave your current call' },
    { command: 'call end', description: 'End the active channel call' },
]

const selectedIndex = ref(0)

const filteredCommands = computed(() => {
    const query = props.query.trim().toLowerCase().replace(/^\^k\s*/, '')
    if (!query) return commands
    return commands.filter((item) => item.command.toLowerCase().startsWith(query))
})

watch(
    () => props.query,
    () => {
        selectedIndex.value = 0
    }
)

watch(
    () => props.show,
    (isShown) => {
        if (isShown) {
            selectedIndex.value = 0
        }
    }
)

function selectNext() {
    if (filteredCommands.value.length === 0) return
    selectedIndex.value = (selectedIndex.value + 1) % filteredCommands.value.length
}

function selectPrevious() {
    if (filteredCommands.value.length === 0) return
    selectedIndex.value = (selectedIndex.value - 1 + filteredCommands.value.length) % filteredCommands.value.length
}

function selectCurrent() {
    if (filteredCommands.value.length === 0) {
        emit('close')
        return
    }

    const selected = filteredCommands.value[selectedIndex.value]
    if (!selected) {
        emit('close')
        return
    }

    emit('select', selected.command)
}

defineExpose({
    selectNext,
    selectPrevious,
    selectCurrent,
})
</script>

<template>
    <div
        v-if="show && filteredCommands.length > 0"
        class="absolute bottom-full left-0 mb-2 w-80 max-h-60 overflow-y-auto rounded-r-2 border border-border-1 bg-bg-surface-1 shadow-2xl z-[120]"
    >
        <div class="flex items-center gap-2 border-b border-border-1 px-3 py-2 text-xs font-medium text-text-3">
            <Command class="h-3.5 w-3.5" />
            <span>Command menu (^k)</span>
        </div>

        <button
            v-for="(item, index) in filteredCommands"
            :key="item.command"
            class="w-full px-3 py-2 text-left transition-standard"
            :class="index === selectedIndex ? 'bg-bg-surface-2 text-text-1' : 'text-text-2 hover:bg-bg-surface-2 hover:text-text-1'"
            @click="$emit('select', item.command)"
        >
            <div class="text-sm font-semibold">^k {{ item.command }}</div>
            <div class="text-xs text-text-3">{{ item.description }}</div>
        </button>
    </div>
</template>
