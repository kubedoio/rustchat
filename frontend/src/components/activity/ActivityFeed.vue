<template>
  <Transition name="slide">
    <div
      v-if="isOpen"
      class="fixed inset-y-0 right-0 w-[400px] bg-white border-l border-gray-200 shadow-xl z-50 flex flex-col"
      role="dialog"
      aria-label="Activity feed"
      aria-modal="true"
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-gray-200">
        <div class="flex items-center gap-2">
          <Bell class="w-5 h-5" />
          <h2 class="text-lg font-semibold">
            Activity
            <span v-if="unreadCount > 0" class="ml-2 text-sm text-red-500 font-normal">
              ({{ unreadCount }} unread)
            </span>
          </h2>
        </div>
        <div class="flex items-center gap-2">
          <button
            v-if="unreadCount > 0"
            class="text-sm text-blue-500 hover:text-blue-600 px-2 py-1 rounded"
            @click="handleMarkAllRead"
          >
            Mark all read
          </button>
          <button
            class="p-1 hover:bg-gray-100 rounded"
            aria-label="Close activity feed"
            @click="close"
          >
            <X class="w-5 h-5" />
          </button>
        </div>
      </div>

      <!-- Filters -->
      <ActivityFilters
        :model-value="filter"
        @update:model-value="handleFilterChange"
      />

      <!-- Activity List -->
      <div class="flex-1 overflow-y-auto">
        <div v-if="isLoading && activities.length === 0" class="flex items-center justify-center py-12 text-gray-500">
          <Loader2 class="w-6 h-6 animate-spin mr-2" />
          Loading...
        </div>

        <div v-else-if="activities.length === 0" class="text-center py-12 text-gray-500 px-4">
          <Inbox class="w-12 h-12 mx-auto mb-3 opacity-40" />
          <p class="font-medium">No activity yet</p>
          <p class="text-sm mt-1">Mentions, replies, and reactions will appear here</p>
        </div>

        <div v-else class="divide-y divide-gray-100">
          <ActivityItem
            v-for="activity in activities"
            :key="activity.id"
            :activity="activity"
            @click="handleActivityClick(activity)"
            @mark-read="handleMarkRead(activity.id)"
          />

          <div v-if="hasMore" class="p-4 text-center">
            <button
              class="text-blue-500 hover:text-blue-600 text-sm disabled:opacity-50"
              :disabled="isLoading"
              @click="loadMore"
            >
              {{ isLoading ? 'Loading...' : 'Load more' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Transition>

  <!-- Backdrop -->
  <Transition name="fade">
    <div
      v-if="isOpen"
      class="fixed inset-0 bg-black/20 z-40"
      @click="close"
    />
  </Transition>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { Bell, X, Inbox, Loader2 } from 'lucide-vue-next'
import { useRouter } from 'vue-router'
import { useActivityStore } from '../../features/activity/stores/activityStore'
import { activityService } from '../../features/activity/services/activityService'
import type { Activity } from '../../features/activity/types'
import { ActivityType } from '../../features/activity/types'
import ActivityItem from './ActivityItem.vue'
import ActivityFilters from './ActivityFilters.vue'

const store = useActivityStore()
const router = useRouter()

const isOpen = computed(() => store.isOpen)
const activities = computed(() => store.getActivities)
const unreadCount = computed(() => store.unreadCount)
const hasMore = computed(() => store.hasMore)
const isLoading = computed(() => store.isLoading)
const filter = computed(() => store.filter)

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isOpen.value) {
    close()
  }
}

onMounted(() => document.addEventListener('keydown', handleKeydown))
onUnmounted(() => document.removeEventListener('keydown', handleKeydown))

function close() {
  activityService.closeFeed()
}

function loadMore() {
  activityService.loadMore()
}

function handleFilterChange(type: ActivityType | null) {
  activityService.setFilter(type)
}

async function handleMarkRead(activityId: string) {
  try {
    await activityService.markRead(activityId)
  } catch (error) {
    console.error('Failed to mark activity as read:', error)
  }
}

async function handleMarkAllRead() {
  try {
    await activityService.markAllRead()
  } catch (error) {
    console.error('Failed to mark all as read:', error)
  }
}

function handleActivityClick(activity: Activity) {
  if (!activity.read) {
    handleMarkRead(activity.id)
  }
  if (activity.rootId) {
    router.push(`/channels/${activity.channelId}?thread=${activity.rootId}`)
  } else {
    router.push(`/channels/${activity.channelId}?post=${activity.postId}`)
  }
  activityService.closeFeed()
}
</script>

<style scoped>
.slide-enter-active,
.slide-leave-active {
  transition: transform 0.2s ease-out;
}
.slide-enter-from,
.slide-leave-to {
  transform: translateX(100%);
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease-out;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
