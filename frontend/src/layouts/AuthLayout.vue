<script setup lang="ts">
import { computed } from 'vue'
import { useConfigStore } from '../stores/config'
const configStore = useConfigStore()

const siteName = computed(() => configStore.siteConfig.site_name || 'RustChat')
const siteInitial = computed(() => siteName.value.charAt(0).toUpperCase())
</script>

<template>
  <div class="min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
    <div class="sm:mx-auto sm:w-full sm:max-w-md">
      <div class="flex justify-center">
        <img v-if="configStore.siteConfig.logo_url" :src="configStore.siteConfig.logo_url" class="h-16 w-16 object-cover rounded-xl" alt="Logo" />
        <div v-else class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary text-2xl font-bold text-brand-foreground">
          {{ siteInitial }}
        </div>
      </div>
      <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
        <slot name="title">{{ siteName }}</slot>
      </h2>
      <p class="mt-2 text-center text-sm text-gray-600">
        <slot name="subtitle"></slot>
      </p>
    </div>

    <div class="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
      <div class="bg-white py-8 px-4 shadow sm:rounded-lg sm:px-10">
        <slot />
      </div>
    </div>
  </div>
</template>
