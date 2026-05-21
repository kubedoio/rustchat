<script setup lang="ts">
import { ref, computed } from 'vue'
import { File as FileIcon, X } from 'lucide-vue-next'
import { filesApi, type FileUploadResponse } from '../../api/files'
import FileUploader from '../atomic/FileUploader.vue'
import { getErrorMessage } from '@/core/errors/errorUtils'

const props = defineProps<{
  modelValue: {
    file: File
    uploading: boolean
    progress: number
    uploaded?: FileUploadResponse
  }[]
  channelId?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: typeof props.modelValue]
  'upload-success': [fileName: string]
  'upload-error': [message: string]
}>()

const fileUploaderRef = ref<InstanceType<typeof FileUploader> | null>(null)

const uploadInProgressCount = computed(
  () => props.modelValue.filter((attachment) => attachment.uploading).length
)

function openFilePicker() {
  fileUploaderRef.value?.openFilePicker()
}

function removeAttachment(index: number) {
  if (props.modelValue[index]?.uploading) return
  const newValue = [...props.modelValue]
  newValue.splice(index, 1)
  emit('update:modelValue', newValue)
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function handleFiles(files: File[]) {
  for (const file of files) {
    const attachment = {
      file,
      uploading: true,
      progress: 0,
      uploaded: undefined as FileUploadResponse | undefined,
    }
    const newValue = [...props.modelValue, attachment]
    emit('update:modelValue', newValue)
    const attachmentIndex = newValue.length - 1

    try {
      const response = await filesApi.upload(file, props.channelId, (progressEvent) => {
        if (progressEvent.total) {
          const updated = [...newValue]
          const current = updated[attachmentIndex]
          if (current) {
            updated[attachmentIndex] = {
              ...current,
              progress: Math.round((progressEvent.loaded * 100) / progressEvent.total),
            }
            emit('update:modelValue', updated)
          }
        }
      })

      const updated = [...newValue]
      const current = updated[attachmentIndex]
      if (current) {
        updated[attachmentIndex] = {
          ...current,
          uploaded: response.data,
          uploading: false,
        }
        emit('update:modelValue', updated)
      }
      emit('upload-success', file.name)
    } catch (error: unknown) {
      emit('upload-error', getErrorMessage(error, 'Unknown error'))
      emit('update:modelValue', newValue.filter((_, i) => i !== attachmentIndex))
    }
  }
}

defineExpose({ openFilePicker })
</script>

<template>
  <FileUploader
    ref="fileUploaderRef"
    class="shrink-0 z-[80] border-t border-border-1 bg-bg-surface-1 p-3"
    @files-selected="handleFiles"
  >
    <div
      class="relative overflow-visible rounded-r-2 border border-border-1 bg-bg-surface-1 transition-standard focus-within:border-brand/60 focus-within:ring-2 focus-within:ring-brand/10"
    >
      <slot name="toolbar" />

      <!-- Attached Files -->
      <div
        v-if="modelValue.length > 0"
        class="flex flex-wrap gap-2 border-b border-border-1 bg-bg-surface-2/30 px-3 py-2"
      >
        <div
          v-for="(attachment, index) in modelValue"
          :key="attachment.file.name"
          class="relative flex min-w-[200px] max-w-[300px] items-center gap-2 rounded-r-1 border border-border-1 bg-bg-surface-1 px-3 py-2"
        >
          <FileIcon class="h-4 w-4 shrink-0 text-text-3" />
          <div class="min-w-0 flex-1">
            <p class="truncate text-xs font-medium text-text-1">{{ attachment.file.name }}</p>
            <p class="text-[10px] text-text-3">{{ formatFileSize(attachment.file.size) }}</p>
          </div>
          <button
            class="rounded p-1 text-text-3 transition-standard hover:bg-danger/10 hover:text-danger focus-ring disabled:cursor-not-allowed disabled:opacity-40"
            aria-label="Remove attachment"
            :disabled="attachment.uploading"
            @click="removeAttachment(index)"
          >
            <X class="h-3.5 w-3.5" />
          </button>

          <div v-if="attachment.uploading" class="absolute inset-x-0 bottom-0 h-0.5 bg-border-1">
            <div
              class="h-full bg-brand transition-standard"
              :style="{ width: `${attachment.progress}%` }"
            ></div>
          </div>
        </div>
      </div>

      <slot />

      <slot name="bottom" />

      <!-- Upload Progress -->
      <div
        v-if="uploadInProgressCount > 0"
        class="border-t border-border-1/50 px-3 py-1 text-[11px] text-text-3"
      >
        Uploading {{ uploadInProgressCount }} file{{ uploadInProgressCount > 1 ? 's' : '' }}...
      </div>
    </div>
  </FileUploader>
</template>
