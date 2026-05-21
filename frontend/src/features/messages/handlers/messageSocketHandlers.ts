// Message WebSocket Handlers - Feature-specific WebSocket event handling
// Replaces the centralized useWebSocket.ts message handling logic

import { log } from '@/utils/log'
import { messageService } from '../services/messageService'
import type { FileAttachment, Message, MessageId } from '../../../core/entities/Message'
import type { ChannelId } from '../../../core/entities/Channel'

export interface WebSocketMessageEvent {
  event: 'posted' | 'post_edited' | 'post_deleted' | 'reaction_added' | 'reaction_removed'
  data: string // JSON stringified data
  broadcast: {
    channel_id: string
    user_id: string
  }
}

interface PostData {
  post: string // JSON stringified Message
}

export function handleWebSocketEvent(event: WebSocketMessageEvent) {
  switch (event.event) {
    case 'posted':
      handlePost(event)
      break
    case 'post_edited':
      handlePostEdit(event)
      break
    case 'post_deleted':
      handlePostDelete(event)
      break
    case 'reaction_added':
      handleReactionAdded(event)
      break
    case 'reaction_removed':
      handleReactionRemoved(event)
      break
  }
}

function handlePost(event: WebSocketMessageEvent) {
  try {
    const data: PostData = JSON.parse(event.data)
    const post: Message = JSON.parse(data.post)

    // Normalize the post
    const normalizedPost = normalizePost(post)
    messageService.handleIncomingMessage(normalizedPost)
  } catch (err) {
    log.error('Failed to handle post:', err)
  }
}

function handlePostEdit(event: WebSocketMessageEvent) {
  try {
    const data: PostData = JSON.parse(event.data)
    const post: Message = JSON.parse(data.post)

    const normalizedPost = normalizePost(post)
    messageService.handleMessageUpdate(normalizedPost.id, normalizedPost)
  } catch (err) {
    log.error('Failed to handle post edit:', err)
  }
}

function handlePostDelete(event: WebSocketMessageEvent) {
  try {
    const data = JSON.parse(event.data)
    const channelId = data.channel_id as ChannelId
    const messageId = data.post_id as MessageId

    messageService.handleMessageDelete(messageId, channelId)
  } catch (err) {
    log.error('Failed to handle post delete:', err)
  }
}

function handleReactionAdded(event: WebSocketMessageEvent) {
  try {
    const data = JSON.parse(event.data)
    const reaction = JSON.parse(data.reaction)

    messageService.handleReactionAdded(
      reaction.post_id as MessageId,
      reaction.emoji_name,
      reaction.user_id
    )
  } catch (err) {
    log.error('Failed to handle reaction added:', err)
  }
}

function handleReactionRemoved(event: WebSocketMessageEvent) {
  try {
    const data = JSON.parse(event.data)
    const reaction = JSON.parse(data.reaction)

    messageService.handleReactionRemoved(
      reaction.post_id as MessageId,
      reaction.emoji_name,
      reaction.user_id
    )
  } catch (err) {
    log.error('Failed to handle reaction removed:', err)
  }
}

// Normalize WebSocket post format to our Message entity
function normalizePost(post: unknown): Message {
  const p = post as Record<string, unknown>
  return {
    id: p.id as string,
    channelId: p.channel_id as string,
    userId: p.user_id as string,
    content: p.message as string,
    rootId: p.root_id as string | undefined,
    replyCount: (p.reply_count as number | undefined) ?? 0,
    reactions: normalizeReactions(p.reactions),
    files: normalizeFiles(
      ((p.metadata as Record<string, unknown> | undefined)?.files || p.files || []) as unknown[]
    ),
    isPinned: (p.is_pinned as boolean | undefined) ?? false,
    isSaved: (p.is_saved as boolean | undefined) ?? false,
    status: 'delivered',
    clientId: (p.props as Record<string, unknown> | undefined)?.client_msg_id as string | undefined,
    createdAt: new Date(p.create_at as string | number),
    updatedAt: p.update_at ? new Date(p.update_at as string | number) : undefined,
    props: p.props as Record<string, unknown> | undefined,
  }
}

function normalizeReactions(
  reactions: unknown
): { emoji: string; count: number; users: string[] }[] {
  if (!reactions) return []

  if (Array.isArray(reactions)) {
    return reactions.map(r => {
      const reaction = r as Record<string, unknown>
      return {
        emoji: reaction.emoji_name as string,
        count: reaction.count as number,
        users: (reaction.users as string[] | undefined) || [],
      }
    })
  }

  // Handle object format
  return Object.entries(reactions as Record<string, unknown>).map(([emoji, data]) => {
    const d = data as Record<string, unknown>
    return {
      emoji,
      count: (d.count as number | undefined) || 0,
      users: (d.users as string[] | undefined) || [],
    }
  })
}

function normalizeFiles(files: unknown[]): FileAttachment[] {
  return files.map(f => {
    const file = f as Record<string, unknown>
    return {
      id: file.id as string,
      name: file.name as string,
      url: file.url as string,
      size: file.size as number,
      mimeType: (file.mime_type || file.mimeType) as string,
    }
  })
}
