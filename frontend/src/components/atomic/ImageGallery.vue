<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { X, ChevronLeft, ChevronRight, Download } from 'lucide-vue-next'
import type { FileUploadResponse } from '../../api/files'

const props = defineProps<{
  images: FileUploadResponse[]
  initialIndex: number
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const currentIndex = ref(props.initialIndex)

const currentImage = computed(() => props.images[currentIndex.value])

function next() {
  currentIndex.value = (currentIndex.value + 1) % props.images.length
}

function prev() {
  currentIndex.value = (currentIndex.value - 1 + props.images.length) % props.images.length
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
  if (e.key === 'ArrowRight') next()
  if (e.key === 'ArrowLeft') prev()
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
  document.body.style.overflow = 'hidden'
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  document.body.style.overflow = ''
})
</script>

<template>
  <div class="fixed inset-0 z-[100] flex flex-col bg-black/95 backdrop-blur-sm animate-in fade-in duration-300">
    <!-- Header -->
    <div class="flex items-center justify-between p-4 text-white z-10">
      <div v-if="currentImage" class="flex flex-col">
        <span class="text-sm font-semibold truncate max-w-md">{{ currentImage.name }}</span>
        <span class="text-[11px] text-gray-400 capitalize">{{ currentIndex + 1 }} of {{ images.length }}</span>
      </div>
      <div class="flex items-center space-x-2">
        <a 
          v-if="currentImage"
          :href="currentImage.url" 
          download 
          class="p-2 hover:bg-white/10 rounded-full transition-colors"
          title="Download"
        >
          <Download class="w-5 h-5" />
        </a>
        <button 
          @click="emit('close')" 
          class="p-2 hover:bg-white/10 rounded-full transition-colors"
          title="Close"
        >
          <X class="w-6 h-6" />
        </button>
      </div>
    </div>

    <!-- Main View -->
    <div class="flex-1 relative flex items-center justify-center overflow-hidden">
      <!-- Navigation -->
      <button 
        v-if="images.length > 1"
        @click="prev" 
        class="absolute left-4 z-20 p-3 bg-black/20 hover:bg-black/40 text-white rounded-full backdrop-blur-md transition-all active:scale-95"
      >
        <ChevronLeft class="w-8 h-8" />
      </button>

      <!-- Image -->
      <transition 
        enter-active-class="transform transition duration-300 ease-out"
        enter-from-class="opacity-0 scale-95"
        leave-active-class="transform transition duration-200 ease-in"
        leave-to-class="opacity-0 scale-95"
        mode="out-in"
      >
        <img 
          v-if="currentImage"
          :key="currentImage.id"
          :src="currentImage.url" 
          :alt="currentImage.name"
          class="max-w-[90vw] max-h-[80vh] object-contain shadow-2xl rounded-sm"
        />
      </transition>

      <button 
        v-if="images.length > 1"
        @click="next" 
        class="absolute right-4 z-20 p-3 bg-black/20 hover:bg-black/40 text-white rounded-full backdrop-blur-md transition-all active:scale-95"
      >
        <ChevronRight class="w-8 h-8" />
      </button>
    </div>

    <!-- Thumbnails Strip -->
    <div v-if="images.length > 1" class="h-24 bg-black/40 backdrop-blur-md flex items-center justify-center p-4 space-x-2 overflow-x-auto">
      <button 
        v-for="(img, index) in images" 
        :key="img.id"
        @click="currentIndex = index"
        class="h-16 aspect-video rounded overflow-hidden border-2 transition-all flex-shrink-0"
        :class="currentIndex === index ? 'border-primary scale-105 shadow-lg' : 'border-transparent opacity-50 hover:opacity-100'"
      >
        <img :src="img.thumbnail_url || img.url" class="w-full h-full object-cover" />
      </button>
    </div>
  </div>
</template>
