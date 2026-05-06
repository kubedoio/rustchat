<template>
  <div
    class="flex items-start gap-3 p-4 cursor-pointer transition-colors"
    :class="{
      'bg-blue-50 hover:bg-blue-100': !activity.read,
      'hover:bg-gray-50': activity.read
    }"
    @click="$emit('click')"
  >
    <!-- Unread indicator -->
    <div class="w-1.5 self-stretch flex items-start pt-1.5">
      <div
        v-if="!activity.read"
        class="w-1.5 h-1.5 bg-blue-500 rounded-full"
      />
    </div>

    <!-- Icon -->
    <ActivityIcon :type="activity.type" />

    <!-- Content -->
    <div class="flex-1 min-w-0">
      <div class="flex items-start justify-between gap-2">
        <p class="text-sm leading-snug">
          <span class="font-semibold">{{ activity.actorUsername }}</span>
          {{ ' ' }}{{ actionText }}
        </p>
        <span class="text-xs text-gray-400 whitespace-nowrap flex-shrink-0 mt-0.5">
          {{ formattedTime }}
        </span>
      </div>

      <p v-if="activity.message" class="text-sm text-gray-600 mt-1 line-clamp-2">
        {{ activity.message }}
      </p>

      <p v-if="activity.type === ActivityType.REACTION && activity.reaction" class="text-base mt-1">
        {{ activity.reaction }}
      </p>

      <p class="text-xs text-gray-400 mt-1">
        #{{ activity.channelName }} · {{ activity.teamName }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { formatDistanceToNow } from 'date-fns'
import type { Activity } from '../../features/activity/types'
import { ActivityType } from '../../features/activity/types'
import ActivityIcon from './ActivityIcon.vue'

const props = defineProps<{
  activity: Activity
}>()

defineEmits<{
  click: []
  'mark-read': []
}>()

const actionText = computed(() => {
  switch (props.activity.type) {
    case ActivityType.MENTION: return 'mentioned you'
    case ActivityType.REPLY: return 'replied to your message'
    case ActivityType.REACTION: return `reacted to your message`
    case ActivityType.DM: return 'sent you a message'
    case ActivityType.THREAD_REPLY: return 'replied in a thread you follow'
    default: return 'interacted with you'
  }
})

const formattedTime = computed(() => {
  return formatDistanceToNow(props.activity.createdAt, { addSuffix: true })
})
</script>
