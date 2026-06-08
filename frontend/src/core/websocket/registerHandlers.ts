// WebSocket Handler Registration — UNFINISHED REFACTORING, NOT WIRED INTO THE APP
//
// The active WebSocket implementation is in frontend/src/composables/useWebSocket.ts.
// registerWebSocketHandlers() is never imported or called. Do not expect runtime
// changes from this file until the refactoring is completed and wired into main.ts/App.vue.

import { log } from '@/utils/log'
import { wsManager, type WebSocketEvent } from './WebSocketManager'
import { handleMessageWebSocketEvent, registerThreadHandlers } from '../../features/messages'
import { handleCallWebSocketEvent } from '../../features/calls'
import { handleChannelWebSocketEvent } from '../../features/channels'
import {
  handleActivityCreated,
  handleActivityRead,
} from '../../features/activity/handlers/activitySocketHandlers'
import type { Post } from '../../api/posts'
import type { WebSocketMessageEvent } from '../../features/messages/handlers/messageSocketHandlers'
import type { WebSocketCallEvent } from '../../features/calls/handlers/callSocketHandlers'
import type { WebSocketChannelEvent } from '../../features/channels/handlers/channelSocketHandlers'

/**
 * Register all WebSocket event handlers
 * Call this once during app initialization
 */
export function registerWebSocketHandlers(): void {
  // Initialize thread handlers
  const threadHandlers = registerThreadHandlers()

  // Message events
  wsManager.on('posted', (event: WebSocketEvent) => {
    handleMessageWebSocketEvent(event as unknown as WebSocketMessageEvent)
    // Also handle for thread updates
    const data = JSON.parse(event.data as string)
    const post: Post = JSON.parse(data.post)
    threadHandlers.handleNewPost(post)
  })
  wsManager.on('post_edited', (event: WebSocketEvent) => {
    handleMessageWebSocketEvent(event as unknown as WebSocketMessageEvent)
    // Also handle for thread updates
    const data = JSON.parse(event.data as string)
    const post: Post = JSON.parse(data.post)
    threadHandlers.handlePostUpdated(post)
  })
  wsManager.on('post_deleted', (event: WebSocketEvent) => {
    handleMessageWebSocketEvent(event as unknown as WebSocketMessageEvent)
    // Also handle for thread updates
    const data = JSON.parse(event.data as string)
    threadHandlers.handlePostDeleted(data.post_id)
  })
  wsManager.on('reaction_added', (event: WebSocketEvent) =>
    handleMessageWebSocketEvent(event as unknown as WebSocketMessageEvent)
  )
  wsManager.on('reaction_removed', (event: WebSocketEvent) =>
    handleMessageWebSocketEvent(event as unknown as WebSocketMessageEvent)
  )

  // Call events
  wsManager.on('custom_com.mattermost.calls_call_start', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_call_end', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_user_joined', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_user_left', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_user_muted', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_user_unmuted', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_raise_hand', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_lower_hand', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_user_voice_on', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_user_voice_off', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_host_mute', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_host_removed', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_host_changed', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_ringing', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_screen_on', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_screen_off', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )
  wsManager.on('custom_com.mattermost.calls_signal', (event: WebSocketEvent) =>
    handleCallWebSocketEvent(event as WebSocketCallEvent)
  )

  // Channel events
  wsManager.on('channel_created', (event: WebSocketEvent) =>
    handleChannelWebSocketEvent(event as WebSocketChannelEvent)
  )
  wsManager.on('channel_updated', (event: WebSocketEvent) =>
    handleChannelWebSocketEvent(event as WebSocketChannelEvent)
  )
  wsManager.on('channel_deleted', (event: WebSocketEvent) =>
    handleChannelWebSocketEvent(event as WebSocketChannelEvent)
  )
  wsManager.on('channel_restored', (event: WebSocketEvent) =>
    handleChannelWebSocketEvent(event as WebSocketChannelEvent)
  )
  wsManager.on('user_added', (event: WebSocketEvent) =>
    handleChannelWebSocketEvent(event as WebSocketChannelEvent)
  )
  wsManager.on('user_removed', (event: WebSocketEvent) =>
    handleChannelWebSocketEvent(event as WebSocketChannelEvent)
  )
  wsManager.on('channel_viewed', (event: WebSocketEvent) =>
    handleChannelWebSocketEvent(event as WebSocketChannelEvent)
  )

  // Activity feed events
  wsManager.on('activity_created', (event: WebSocketEvent) => {
    const data = JSON.parse(event.data as string)
    handleActivityCreated(data)
  })
  wsManager.on('activity_read', (event: WebSocketEvent) => {
    const data = JSON.parse(event.data as string)
    handleActivityRead(data)
  })

  log.debug('[WebSocket] All handlers registered')
}

/**
 * Unregister all handlers (useful for testing or hot reload)
 */
export function unregisterWebSocketHandlers(): void {
  // Note: Current implementation doesn't support unregistering
  // Would need to store unsubscribe functions from wsManager.on()
  log.debug('[WebSocket] Handler unregistration not implemented')
}
