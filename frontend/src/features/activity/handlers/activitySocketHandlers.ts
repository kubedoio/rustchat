/**
 * Activity Socket Handlers - WebSocket event handlers for activity feed
 */

import { log } from '@/utils/log'
import { activityService } from '../services/activityService'
import type { Activity } from '../types'
import { ActivityType } from '../types'

function parseActivityType(raw: string): ActivityType {
  const map: Record<string, ActivityType> = {
    mention: ActivityType.MENTION,
    reply: ActivityType.REPLY,
    reaction: ActivityType.REACTION,
    dm: ActivityType.DM,
    thread_reply: ActivityType.THREAD_REPLY,
  }
  const type = map[raw]
  if (type === undefined) {
    log.warn(`[ActivitySocket] Unknown activity type: ${raw}, defaulting to MENTION`)
    return ActivityType.MENTION
  }
  return type
}

export function handleActivityCreated(data: Record<string, unknown>): void {
  const activity: Activity = {
    id: data.id as string,
    type: parseActivityType(data.type as string),
    actorId: data.actor_id as string,
    actorUsername: (data.actor_username as string) ?? '',
    actorAvatarUrl: data.actor_avatar_url as string | undefined,
    channelId: data.channel_id as string,
    channelName: (data.channel_name as string) ?? '',
    teamId: data.team_id as string,
    teamName: (data.team_name as string) ?? '',
    postId: data.post_id as string,
    rootId: data.root_id as string | undefined,
    message: data.message_text as string | undefined,
    reaction: data.reaction as string | undefined,
    read: false,
    createdAt: data.created_at ? new Date(data.created_at as string) : new Date(),
  }

  activityService.handleNewActivity(activity)
}

export function handleActivityRead(_data: Record<string, unknown>): void {
  // Multi-device sync: another session marked activities as read
  // Reload to sync state
  activityService.syncIfOpen()
}
