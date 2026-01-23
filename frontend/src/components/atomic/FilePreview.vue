<script setup lang="ts">
import { File as FileIcon, Download, ExternalLink } from 'lucide-vue-next';
import type { FileUploadResponse } from '../../api/files';

const props = defineProps<{
    file: FileUploadResponse
}>()

const emit = defineEmits<{
    (e: 'preview', file: FileUploadResponse): void
}>()

const formatSize = (bytes: number): string => {
    if (bytes < 1024) return bytes + ' B'
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

const isImage = (mimeType: string): boolean => {
    return mimeType.startsWith('image/')
}

function handlePreview() {
    if (isImage(props.file.mime_type)) {
        emit('preview', props.file)
    }
}
</script>

<template>
  <div 
    class="group relative flex flex-col p-1 rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:shadow-md transition-all duration-200 max-w-sm overflow-hidden"
    :class="{ 'cursor-pointer hover:border-indigo-400 dark:hover:border-indigo-500': isImage(file.mime_type) }"
    @click="handlePreview"
  >
      
      <!-- Icon or Thumbnail -->
      <div class="relative aspect-video w-full rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-900 border border-gray-100 dark:border-gray-800">
          <img 
            v-if="isImage(file.mime_type)" 
            :src="file.thumbnail_url || file.url" 
            :alt="file.name" 
            class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105" 
            loading="lazy" 
          />
          <div v-else class="w-full h-full flex flex-col items-center justify-center text-gray-400">
              <FileIcon class="w-10 h-10 mb-2 stroke-1" />
              <span class="text-[10px] uppercase font-bold tracking-widest">{{ file.mime_type.split('/')[1] || 'FILE' }}</span>
          </div>

          <!-- Hover Overlay -->
          <div v-if="isImage(file.mime_type)" class="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors flex items-center justify-center">
              <ExternalLink class="w-6 h-6 text-white opacity-0 group-hover:opacity-100 transition-opacity drop-shadow-lg" />
          </div>
      </div>

      <!-- Details -->
      <div class="p-2 flex items-center justify-between">
          <div class="flex-1 min-w-0 mr-2">
              <div class="text-xs font-semibold text-gray-900 dark:text-gray-100 truncate" :title="file.name">
                  {{ file.name }}
              </div>
              <div class="text-[10px] text-gray-500 flex items-center space-x-2 font-medium">
                  <span>{{ formatSize(file.size) }}</span>
              </div>
          </div>

          <a 
            :href="file.url" 
            download 
            @click.stop
            class="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg text-gray-500 hover:text-indigo-600 dark:text-gray-400 dark:hover:text-indigo-400 transition-colors"
          >
              <Download class="w-4 h-4" />
          </a>
      </div>
  </div>
</template>
