/**
 * Shared mapper for the snake_case ActivityResponse payload used by the REST
 * feed and realtime `activity_created` events.
 */

import type { Activity } from './types'
import { ActivityType } from './types'

export function transformActivity(apiActivity: Record<string, unknown>): Activity {
  return {
    id: apiActivity.id as string,
    type: apiActivity.type as ActivityType,
    actorId: apiActivity.actor_id as string,
    actorUsername: apiActivity.actor_username as string,
    actorAvatarUrl: apiActivity.actor_avatar_url as string | undefined,
    channelId: apiActivity.channel_id as string,
    channelName: apiActivity.channel_name as string,
    teamId: apiActivity.team_id as string,
    teamName: apiActivity.team_name as string,
    postId: apiActivity.post_id as string,
    rootId: apiActivity.root_id as string | undefined,
    message: apiActivity.message_text as string | undefined,
    reaction: apiActivity.reaction as string | undefined,
    read: apiActivity.read as boolean,
    createdAt: new Date(apiActivity.created_at as string),
  }
}
