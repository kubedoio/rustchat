<script setup lang="ts">
import { ref, watch } from 'vue';
import { X } from 'lucide-vue-next';
import { useAuthStore } from '../../stores/auth';

const props = defineProps<{
  show: boolean
}>();

const emit = defineEmits<{
  (e: 'close'): void
}>();

const auth = useAuthStore();
const emoji = ref('💬');
const text = ref('');
const duration = ref<string>('');

// Initialize from current status when opening
watch(() => props.show, (isOpen) => {
  if (isOpen && auth.user) {
    text.value = auth.user.status_text || '';
    emoji.value = auth.user.status_emoji || '💬';
    duration.value = '';
  }
});

// Mattermost-compatible duration options
const durations = [
  { label: "Don't clear", value: '' },
  { label: '30 minutes', value: 'thirty_minutes' },
  { label: '1 hour', value: 'one_hour' },
  { label: '4 hours', value: 'four_hours' },
  { label: 'Today', value: 'today' },
  { label: 'This week', value: 'this_week' },
  { label: 'Custom date and time', value: 'custom_date_time' },
];

async function save() {
    await auth.updateStatus({
        text: text.value,
        emoji: emoji.value,
        duration: duration.value || undefined
    });
    emit('close');
}

async function clear() {
    await auth.updateStatus({
        text: '',
        emoji: '',
        duration: undefined
    });
    emit('close');
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/60 backdrop-blur-sm transition-opacity" @click="emit('close')"></div>

      <!-- Modal Panel -->
      <div class="relative bg-white rounded-lg shadow-xl w-full max-w-lg overflow-hidden transform transition-all">
        <!-- Header -->
        <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200">
            <h3 class="text-lg font-semibold text-gray-900">Set a status</h3>
            <button @click="emit('close')" class="text-gray-400 hover:text-gray-500 transition-colors">
                <X class="w-5 h-5" />
            </button>
        </div>
        
        <!-- Content -->
        <div class="p-6 space-y-4">
            <!-- Status Input -->
            <div class="flex space-x-2">
                <button class="flex-shrink-0 w-10 h-10 flex items-center justify-center rounded-md border border-gray-300 hover:bg-gray-50 text-xl transition-colors">
                    {{ emoji }}
                </button>
                <input 
                    v-model="text"
                    type="text" 
                    placeholder="What's your status?" 
                    class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                    @keydown.enter="save"
                    autofocus
                />
            </div>

            <!-- Clear After -->
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Clear after</label>
                <select v-model="duration" class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6">
                    <option v-for="opt in durations" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
                </select>
            </div>
        </div>

        <!-- Footer -->
        <div class="bg-gray-50 px-4 py-3 sm:flex sm:flex-row-reverse border-t border-gray-200">
            <button 
                type="button" 
                class="inline-flex w-full justify-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 sm:ml-3 sm:w-auto transition-colors" 
                @click="save"
            >
                Save
            </button>
            <button 
                v-if="auth.user?.status_text"
                type="button" 
                class="mt-3 inline-flex w-full justify-center rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 sm:mt-0 sm:w-auto sm:mr-auto transition-colors" 
                @click="clear"
            >
                Clear Status
            </button>
            <button 
                type="button" 
                class="mt-3 inline-flex w-full justify-center rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 sm:mt-0 sm:w-auto transition-colors" 
                @click="emit('close')"
            >
                Cancel
            </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
