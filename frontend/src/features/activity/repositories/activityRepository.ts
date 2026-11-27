/**
 * Activity Repository - API layer for activity feed
 */

import { v4Api } from '../../../api/client'
import type { Activity, ActivityFeedResponse, ActivityQueryParams } from '../types'
import { transformActivity } from '../activityMapper'

export const activityRepository = {
  async getFeed(userId: string, params: ActivityQueryParams = {}): Promise<ActivityFeedResponse> {
    const queryParams = new URLSearchParams()
    if (params.cursor) queryParams.set('cursor', params.cursor)
    if (params.limit) queryParams.set('limit', params.limit.toString())
    if (params.type) queryParams.set('type', params.type)
    if (params.unreadOnly) queryParams.set('unread_only', 'true')

    const qs = queryParams.toString()
    const url = `/users/${userId}/activity${qs ? `?${qs}` : ''}`
    const response = await v4Api.get(url)

    const activities: Record<string, Activity> = {}
    for (const [id, activity] of Object.entries(response.data.activities || {})) {
      activities[id] = transformActivity(activity as Record<string, unknown>)
    }

    return {
      order: response.data.order || [],
      activities,
      unreadCount: response.data.unread_count || 0,
      nextCursor: response.data.next_cursor,
    }
  },

  async markRead(userId: string, activityIds: string[]): Promise<number> {
    const response = await v4Api.post(`/users/${userId}/activity/read`, {
      activity_ids: activityIds,
    })
    return response.data.updated
  },

  async markAllRead(userId: string): Promise<number> {
    const response = await v4Api.post(`/users/${userId}/activity/read-all`)
    return response.data.updated
  },
}
