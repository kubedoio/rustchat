<script setup lang="ts">
import { ref } from 'vue'
import { X, CheckCircle, AlertCircle, Info } from 'lucide-vue-next'

export interface Toast {
  id: number
  type: 'success' | 'error' | 'info'
  title: string
  message?: string
  duration?: number
}

const toasts = ref<Toast[]>([])
let nextId = 0

function add(toast: Omit<Toast, 'id'>) {
  const id = nextId++
  const newToast = { ...toast, id }
  toasts.value.push(newToast)

  if (toast.duration !== 0) {
    setTimeout(() => remove(id), toast.duration || 5000)
  }
  return id
}

function remove(id: number) {
  const index = toasts.value.findIndex(t => t.id === id)
  if (index !== -1) {
    toasts.value.splice(index, 1)
  }
}

// Expose globally via event bus or store later, for now export generic usage
defineExpose({ add, remove })
</script>



<template>
  <div class="fixed top-0 right-0 p-6 z-[100] flex flex-col space-y-4 w-full max-w-sm pointer-events-none">
    <transition-group 
        enter-active-class="transform ease-out duration-300 transition" 
        enter-from-class="translate-y-2 opacity-0 sm:translate-y-0 sm:translate-x-2" 
        enter-to-class="translate-y-0 opacity-100 sm:translate-x-0" 
        leave-active-class="transition ease-in duration-100" 
        leave-from-class="opacity-100" 
        leave-to-class="opacity-0"
    >
        <div 
            v-for="toast in toasts" 
            :key="toast.id" 
            class="pointer-events-auto w-full max-w-sm overflow-hidden rounded-lg bg-white dark:bg-gray-800 shadow-lg ring-1 ring-black ring-opacity-5"
        >
            <div class="p-4">
                <div class="flex items-start">
                    <div class="flex-shrink-0">
                        <CheckCircle v-if="toast.type === 'success'" class="h-6 w-6 text-green-400" />
                        <AlertCircle v-else-if="toast.type === 'error'" class="h-6 w-6 text-red-400" />
                        <Info v-else class="h-6 w-6 text-blue-400" />
                    </div>
                    <div class="ml-3 w-0 flex-1 pt-0.5">
                        <p class="text-sm font-medium text-gray-900 dark:text-white">{{ toast.title }}</p>
                        <p v-if="toast.message" class="mt-1 text-sm text-gray-500 dark:text-gray-400">{{ toast.message }}</p>
                    </div>
                    <div class="ml-4 flex flex-shrink-0">
                        <button @click="remove(toast.id)" type="button" class="inline-flex rounded-md bg-white dark:bg-gray-800 text-gray-400 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2">
                            <span class="sr-only">Close</span>
                            <X class="h-5 w-5" />
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </transition-group>
  </div>
</template>
