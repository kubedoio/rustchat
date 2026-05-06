<script setup lang="ts">
import { Moon, Sun, Sparkles } from 'lucide-vue-next'
import { useThemeStore } from '../../stores/theme'

const themeStore = useThemeStore()

const themes = [
  { id: 'light', name: 'Light', icon: Sun },
  { id: 'dark', name: 'Dark', icon: Moon },
  { id: 'modern', name: 'Modern', icon: Sparkles },
] as const
</script>

<template>
  <div class="flex items-center rounded-r-1 border border-border-1 bg-bg-surface-2 p-1">
    <button
      v-for="t in themes"
      :key="t.id"
      @click="themeStore.setTheme(t.id)"
      class="flex min-h-10 items-center space-x-2 rounded-r-1 px-3 py-1.5 text-sm font-medium transition-standard"
      :class="themeStore.theme === t.id
        ? 'bg-bg-surface-1 text-brand shadow-1'
        : 'text-text-2 hover:bg-bg-surface-1 hover:text-text-1'"
    >
      <component :is="t.icon" class="w-4 h-4" />
      <span class="hidden sm:inline">{{ t.name }}</span>
    </button>
  </div>
</template>
