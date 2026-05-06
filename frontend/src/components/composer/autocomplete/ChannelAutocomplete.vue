<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useChannelStore } from '../../../stores/channels'
import { Hash, Lock } from 'lucide-vue-next'

const props = defineProps<{
    query: string
    show: boolean
}>()

const emit = defineEmits<{
    select: [channelName: string]
    close: []
}>()

const channelStore = useChannelStore()
const selectedIndex = ref(0)

// Filter channels based on query
const filteredChannels = computed(() => {
    if (!props.query) return []
    
    const query = props.query.toLowerCase()
    return channelStore.channels
        .filter(channel => 
            channel.name.toLowerCase().includes(query) ||
            (channel.display_name && channel.display_name.toLowerCase().includes(query))
        )
        .slice(0, 8)
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
    if (filteredChannels.value.length === 0) return
    selectedIndex.value = (selectedIndex.value + 1) % filteredChannels.value.length
}

function selectPrevious() {
    if (filteredChannels.value.length === 0) return
    selectedIndex.value = (selectedIndex.value - 1 + filteredChannels.value.length) % filteredChannels.value.length
}

function selectCurrent() {
    if (filteredChannels.value.length === 0) {
        emit('close')
        return
    }
    const channel = filteredChannels.value[selectedIndex.value]
    if (!channel) {
        emit('close')
        return
    }
    emit('select', channel.name)
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
        v-if="show && filteredChannels.length > 0"
        class="absolute bottom-full left-0 mb-2 w-72 max-h-48 overflow-y-auto bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2xl z-[120]"
    >
        <div class="px-2 py-1.5 text-xs font-medium text-text-3 border-b border-border-1">
            Channels matching "{{ query }}"
        </div>
        <button
            v-for="(channel, index) in filteredChannels"
            :key="channel.id"
            @click="emit('select', channel.name)"
            class="w-full flex items-center px-3 py-2 text-left transition-standard"
            :class="index === selectedIndex ? 'bg-bg-surface-2 text-text-1' : 'text-text-2 hover:bg-bg-surface-2 hover:text-text-1'"
        >
            <component 
                :is="channel.channel_type === 'private' ? Lock : Hash" 
                class="w-4 h-4 mr-2 text-text-3"
            />
            <div class="flex-1 min-w-0">
                <div class="text-sm font-medium truncate">
                    {{ channel.display_name || channel.name }}
                </div>
                <div class="text-xs text-text-3 truncate">
                    ~{{ channel.name }}
                </div>
            </div>
        </button>
    </div>
</template>
