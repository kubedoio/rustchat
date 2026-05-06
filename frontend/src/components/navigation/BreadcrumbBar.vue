<template>
  <nav aria-label="Breadcrumb" class="flex items-center gap-0.5 text-sm min-w-0">
    <template v-for="(segment, index) in segments" :key="index">
      <!-- Separator -->
      <ChevronRight v-if="index > 0" class="w-3.5 h-3.5 text-gray-400 flex-shrink-0" />

      <!-- Clickable segment (router-link) -->
      <RouterLink
        v-if="segment.to"
        :to="segment.to"
        class="flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-gray-100 text-gray-600 transition-colors truncate max-w-[180px]"
      >
        <component :is="getIcon(segment.icon)" v-if="segment.icon" class="w-3.5 h-3.5 flex-shrink-0" />
        <span class="truncate">{{ segment.label }}</span>
      </RouterLink>

      <!-- Non-clickable segment (current location) -->
      <span
        v-else
        class="flex items-center gap-1 px-1.5 py-0.5 text-gray-900 font-medium truncate max-w-[180px]"
        :aria-current="index === segments.length - 1 ? 'location' : undefined"
      >
        <component :is="getIcon(segment.icon)" v-if="segment.icon" class="w-3.5 h-3.5 flex-shrink-0" />
        <span class="truncate">{{ segment.label }}</span>
      </span>
    </template>
  </nav>
</template>

<script setup lang="ts">
import { ChevronRight, Users, Hash, MessageSquare, User, Lock } from 'lucide-vue-next'
import type { RouteLocationRaw } from 'vue-router'

export interface BreadcrumbSegment {
  label: string
  icon?: string
  to?: RouteLocationRaw
}

defineProps<{
  segments: BreadcrumbSegment[]
}>()

const iconMap: Record<string, unknown> = { Users, Hash, MessageSquare, User, Lock }

function getIcon(name?: string) {
  return name ? iconMap[name] : undefined
}
</script>
