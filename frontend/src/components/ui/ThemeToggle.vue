<script setup lang="ts">
import { Sun, Moon, Monitor } from 'lucide-vue-next';
import { useThemeStore } from '../../stores/theme';

const themeStore = useThemeStore();

const themes = [
  { id: 'light', name: 'Light', icon: Sun },
  { id: 'dark', name: 'Dark', icon: Moon },
  { id: 'system', name: 'System', icon: Monitor },
] as const;
</script>

<template>
  <div class="flex items-center bg-gray-100 dark:bg-gray-800 rounded-lg p-1">
    <button 
      v-for="t in themes" 
      :key="t.id"
      @click="themeStore.setTheme(t.id)"
      class="flex items-center space-x-2 px-3 py-1.5 rounded-md text-sm font-medium transition-all"
      :class="themeStore.theme === t.id 
        ? 'bg-white dark:bg-gray-700 text-primary shadow-sm' 
        : 'text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'"
    >
      <component :is="t.icon" class="w-4 h-4" />
      <span>{{ t.name }}</span>
    </button>
  </div>
</template>
