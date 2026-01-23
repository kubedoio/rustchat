<script setup lang="ts">
import { useConfigStore } from '../stores/config'
const configStore = useConfigStore()
</script>

<template>
  <div class="min-h-screen bg-gray-50 dark:bg-gray-900 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
    <div class="sm:mx-auto sm:w-full sm:max-w-md">
      <div class="flex justify-center">
        <img v-if="configStore.siteConfig.logo_url" :src="configStore.siteConfig.logo_url" class="h-16 w-16 object-cover rounded-xl" alt="Logo" />
        <div v-else class="h-12 w-12 bg-primary rounded-xl flex items-center justify-center text-white text-2xl font-bold">
          {{ configStore.siteConfig.site_name.charAt(0).toUpperCase() }}
        </div>
      </div>
      <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900 dark:text-white">
        <slot name="title">{{ configStore.siteConfig.site_name }}</slot>
      </h2>
      <p class="mt-2 text-center text-sm text-gray-600 dark:text-gray-400">
        <slot name="subtitle"></slot>
      </p>
    </div>

    <div class="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
      <div class="bg-white dark:bg-gray-800 py-8 px-4 shadow sm:rounded-lg sm:px-10">
        <slot />
      </div>
    </div>
  </div>
</template>
