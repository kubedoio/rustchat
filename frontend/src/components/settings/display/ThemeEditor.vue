<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Check } from 'lucide-vue-next'
import { THEME_OPTIONS, getThemeColors, type Theme } from '../../../stores/theme'

const props = defineProps<{
  modelValue: Theme
}>()

const emit = defineEmits<{
  'update:modelValue': [theme: Theme]
  save: [theme: Theme]
  cancel: []
}>()

const selectedTheme = ref<Theme>(props.modelValue)

watch(
  () => props.modelValue,
  (nextTheme) => {
    selectedTheme.value = nextTheme
  },
  { immediate: true }
)

const themeCards = computed(() =>
  THEME_OPTIONS.map((theme) => ({
    ...theme,
    preview: getThemeColors(theme.id),
  }))
)

function selectTheme(theme: Theme) {
  selectedTheme.value = theme
  emit('update:modelValue', theme)
}

function handleSave() {
  emit('save', selectedTheme.value)
}

function handleCancel() {
  selectedTheme.value = props.modelValue
  emit('cancel')
}
</script>

<template>
  <div class="space-y-6">
    <div>
      <h5 class="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-text-3">
        Theme Presets
      </h5>
      <p class="text-sm text-text-2">
        Presets use the supported app tokens, so text and controls stay readable across the shell.
      </p>
    </div>

    <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
      <button
        v-for="theme in themeCards"
        :key="theme.id"
        type="button"
        @click="selectTheme(theme.id)"
        class="relative overflow-hidden rounded-r-2 border p-3 text-left transition-standard"
        :class="
          selectedTheme === theme.id
            ? 'border-brand bg-brand/8 ring-2 ring-brand/20'
            : 'border-border-1 bg-bg-surface-1 hover:border-border-2 hover:bg-bg-surface-2'
        "
      >
        <div class="overflow-hidden rounded-r-1 border border-border-1">
          <div class="flex h-16">
            <div
              class="flex w-[32%] items-start p-2"
              :style="{ backgroundColor: theme.preview.sidebarBg }"
            >
              <span
                class="truncate text-[10px] font-semibold uppercase tracking-[0.14em]"
                :style="{ color: theme.preview.sidebarText }"
              >
                Chat
              </span>
            </div>
            <div
              class="relative flex-1 p-2"
              :style="{ backgroundColor: theme.preview.centerChannelBg }"
            >
              <div
                class="h-2.5 w-3/5 rounded-full"
                :style="{ backgroundColor: theme.swatches.primary }"
              />
              <div
                class="mt-2 h-2 w-4/5 rounded-full opacity-75"
                :style="{ backgroundColor: theme.preview.centerChannelColor }"
              />
              <div
                class="mt-1.5 h-2 w-2/3 rounded-full opacity-45"
                :style="{ backgroundColor: theme.preview.centerChannelColor }"
              />
            </div>
          </div>
        </div>

        <div class="mt-3 flex items-center justify-between gap-2">
          <span class="text-sm font-semibold text-text-1">
            {{ theme.label }}
          </span>
          <span class="flex items-center gap-1">
            <span
              class="h-2.5 w-2.5 rounded-full border border-black/5"
              :style="{ backgroundColor: theme.swatches.primary }"
            />
            <span
              class="h-2.5 w-2.5 rounded-full border border-black/5"
              :style="{ backgroundColor: theme.swatches.accent }"
            />
          </span>
        </div>

        <div
          v-if="selectedTheme === theme.id"
          class="absolute right-2 top-2 flex h-5 w-5 items-center justify-center rounded-full bg-brand text-brand-foreground"
        >
          <Check class="h-3 w-3" />
        </div>
      </button>
    </div>

    <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 p-4">
      <p class="text-sm font-medium text-text-1">Note</p>
      <p class="mt-1 text-sm text-text-2">
        Custom color editing is hidden for now because the app only persists the supported preset themes.
      </p>
    </div>

    <div class="flex items-center justify-end gap-2 pt-2">
      <button
        type="button"
        @click="handleCancel"
        class="rounded-r-1 px-4 py-2 text-sm font-medium text-text-2 transition-standard hover:bg-bg-surface-2 hover:text-text-1"
      >
        Cancel
      </button>
      <button
        type="button"
        @click="handleSave"
        class="rounded-r-1 bg-brand px-4 py-2 text-sm font-medium text-brand-foreground shadow-1 transition-standard hover:bg-brand-hover"
      >
        Save
      </button>
    </div>
  </div>
</template>
