/**
 * Activity Store - Pure state management for activity feed
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Activity } from '../types'
import { ActivityType } from '../types'

export const useActivityStore = defineStore('activityStore', () => {
  // State
  const activities = ref<Map<string, Activity>>(new Map())
  const order = ref<string[]>([])
  const unreadCount = ref(0)
  const hasMore = ref(false)
  const cursor = ref<string | null>(null)
  const filter = ref<ActivityType | null>(null)
  const isLoading = ref(false)
  const isOpen = ref(false)

  // Getters
  const getActivities = computed((): Activity[] => {
    return order.value
      .map(id => activities.value.get(id))
      .filter((a): a is Activity => a !== undefined)
  })

  const unreadActivities = computed((): Activity[] => {
    return getActivities.value.filter(a => !a.read)
  })

  // Actions
  function setActivities(newActivities: Activity[], newOrder: string[]) {
    activities.value.clear()
    for (const activity of newActivities) {
      activities.value.set(activity.id, activity)
    }
    order.value = newOrder
  }

  function appendActivities(newActivities: Activity[], newOrder: string[]) {
    for (const activity of newActivities) {
      if (!activities.value.has(activity.id)) {
        activities.value.set(activity.id, activity)
      }
    }
    order.value = [...order.value, ...newOrder.filter(id => !order.value.includes(id))]
  }

  function addActivity(activity: Activity) {
    if (!activities.value.has(activity.id)) {
      activities.value.set(activity.id, activity)
      order.value.unshift(activity.id)
      if (!activity.read) {
        unreadCount.value++
      }
    }
  }

  function markActivityRead(activityId: string) {
    const activity = activities.value.get(activityId)
    if (activity && !activity.read) {
      activity.read = true
      unreadCount.value = Math.max(0, unreadCount.value - 1)
    }
  }

  function markAllActivitiesRead() {
    for (const activity of activities.value.values()) {
      activity.read = true
    }
    unreadCount.value = 0
  }

  function setUnreadCount(count: number) {
    unreadCount.value = count
  }

  function setHasMore(value: boolean) {
    hasMore.value = value
  }

  function setCursor(value: string | null) {
    cursor.value = value
  }

  function setFilter(type: ActivityType | null) {
    filter.value = type
  }

  function setLoading(value: boolean) {
    isLoading.value = value
  }

  function openFeed() {
    isOpen.value = true
  }

  function closeFeed() {
    isOpen.value = false
  }

  function clearActivities() {
    activities.value.clear()
    order.value = []
  }

  return {
    activities,
    order,
    unreadCount,
    hasMore,
    cursor,
    filter,
    isLoading,
    isOpen,
    getActivities,
    unreadActivities,
    setActivities,
    appendActivities,
    addActivity,
    markActivityRead,
    markAllActivitiesRead,
    setUnreadCount,
    setHasMore,
    setCursor,
    setFilter,
    setLoading,
    openFeed,
    closeFeed,
    clearActivities,
  }
})
