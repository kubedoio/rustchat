<script setup lang="ts">
import { X } from 'lucide-vue-next';

interface Props {
  modelValue: boolean;
  title?: string;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  hideClose?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  size: 'md',
  hideClose: false,
});

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
  (e: 'close'): void;
}>();

const sizeClasses: Record<string, string> = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-2xl',
};

function close() {
  emit('update:modelValue', false);
  emit('close');
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="fixed inset-0 bg-black/50" @click="close"></div>

      <!-- Modal container -->
      <div
        :class="[
          'relative bg-bg-surface-1 rounded-xl shadow-xl w-full flex flex-col max-h-[90vh]',
          sizeClasses[size],
        ]"
      >
        <!-- Default header (title + close) -->
        <div
          v-if="title && !$slots.header"
          class="flex items-center justify-between px-5 py-4 border-b border-border-1"
        >
          <h3 class="text-sm font-bold text-text-1">{{ title }}</h3>
          <button
            v-if="!hideClose"
            @click="close"
            class="p-1 hover:bg-bg-surface-2 rounded-lg transition-colors"
          >
            <X class="w-4 h-4 text-text-3" />
          </button>
        </div>

        <!-- Custom header slot -->
        <div v-else-if="$slots.header">
          <slot name="header" />
        </div>

        <!-- Body -->
        <div class="flex-1 overflow-y-auto">
          <slot />
        </div>

        <!-- Footer -->
        <div v-if="$slots.footer" class="px-5 py-4 border-t border-border-1">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
