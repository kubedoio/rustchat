<script setup lang="ts">
import { ref } from 'vue'
import { UploadCloud } from 'lucide-vue-next'

const emit = defineEmits<{
  (e: 'files-selected', files: File[]): void
}>()

const isDragging = ref(false)
const fileInputRef = ref<HTMLInputElement | null>(null)

function onDragEnter(e: DragEvent) {
    if (e.dataTransfer?.types.includes('Files')) {
        isDragging.value = true
    }
}

function onDragLeave() {
    isDragging.value = false
}

function onDrop(e: DragEvent) {
    isDragging.value = false
    const files = e.dataTransfer?.files
    if (files && files.length > 0) {
        emit('files-selected', Array.from(files))
    }
}

function openFilePicker() {
    fileInputRef.value?.click()
}

function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement
    if (input.files && input.files.length > 0) {
        emit('files-selected', Array.from(input.files))
    }
    input.value = ''
}

defineExpose({
    openFilePicker
})
</script>

<template>
  <div 
    class="relative"
    @dragenter.prevent="onDragEnter"
    @dragover.prevent
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
  >
    <!-- Hidden Input -->
    <input
      ref="fileInputRef"
      type="file"
      multiple
      class="hidden"
      @change="onFileSelect"
    />

    <!-- Drop Overlay -->
    <div 
        v-if="isDragging" 
        class="absolute inset-0 z-50 bg-primary/10 border-2 border-dashed border-primary rounded-lg flex flex-col items-center justify-center backdrop-blur-sm pointer-events-none transition-opacity"
    >
        <UploadCloud class="w-10 h-10 text-primary mb-2" />
        <span class="text-primary font-bold">Drop files to upload</span>
    </div>

    <!-- Slot for content (Composer) -->
    <slot></slot>
  </div>
</template>
