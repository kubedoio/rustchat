<template>
  <div
    class="flex-shrink-0 flex items-center justify-center w-9 h-9 rounded-full"
    :class="bgClass"
  >
    <component :is="iconComponent" class="w-4 h-4" :class="colorClass" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { AtSign, MessageCircle, Heart, Mail, MessageSquare } from 'lucide-vue-next'
import { ActivityType } from '../../features/activity/types'

const props = defineProps<{
  type: ActivityType
}>()

type IconConfig = {
  icon: unknown
  bg: string
  color: string
}

const iconConfigs: Record<ActivityType, IconConfig> = {
  [ActivityType.MENTION]: {
    icon: AtSign,
    bg: 'bg-blue-100',
    color: 'text-blue-600'
  },
  [ActivityType.REPLY]: {
    icon: MessageCircle,
    bg: 'bg-green-100',
    color: 'text-green-600'
  },
  [ActivityType.REACTION]: {
    icon: Heart,
    bg: 'bg-pink-100',
    color: 'text-pink-600'
  },
  [ActivityType.DM]: {
    icon: Mail,
    bg: 'bg-purple-100',
    color: 'text-purple-600'
  },
  [ActivityType.THREAD_REPLY]: {
    icon: MessageSquare,
    bg: 'bg-orange-100',
    color: 'text-orange-600'
  }
}

const config = computed(() => iconConfigs[props.type] ?? iconConfigs[ActivityType.MENTION])
const iconComponent = computed(() => config.value.icon)
const bgClass = computed(() => config.value.bg)
const colorClass = computed(() => config.value.color)
</script>
