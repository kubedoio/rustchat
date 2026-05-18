import type { Message, MessageId, MessageDraft } from '../../../core/entities/Message'
import type { ChannelId } from '../../../core/entities/Channel'
import { postsApi, type Post, type CreatePostRequest } from '../../../api/posts'
import { withRetry } from '../../../core/services/retry'
import { isNotFoundError } from '../../../core/errors/errorUtils'

export interface MessageQueryOptions {
  limit?: number
  before?: MessageId
  after?: MessageId
}

export interface MessageReadState {
  lastReadMessageId?: string
  lastReadSeq?: number
  unreadCount: number
  mentionCount: number
}

// Repository implementation for Messages
export const messageRepository = {
  async findById(id: MessageId): Promise<Message | null> {
    return withRetry(async () => {
      try {
        const response = await postsApi.get(id)
        return response.data ? postToMessage(response.data) : null
      } catch (error: unknown) {
        if (isNotFoundError(error)) {
          return null
        }
        throw error
      }
    })
  },

  async findByChannel(
    channelId: ChannelId,
    options: MessageQueryOptions = {}
  ): Promise<{ messages: Message[]; readState?: MessageReadState }> {
    return withRetry(async () => {
      const params: Record<string, unknown> = {
        limit: options.limit ?? 50,
        before: options.before,
        after: options.after
      }

      const response = await postsApi.list(channelId, params)
      const messages = response.data.messages
        .filter((p: Post) => !p.root_post_id) // Only root messages
        .map(postToMessage)
        .reverse()

      // Convert API ReadState to MessageReadState
      const apiReadState = response.data.read_state
      const readState: MessageReadState | undefined = apiReadState ? {
        lastReadMessageId: apiReadState.last_read_message_id?.toString(),
        lastReadSeq: apiReadState.last_read_message_id ?? undefined,
        unreadCount: 0, // Will be populated from other API calls
        mentionCount: 0
      } : undefined

      return {
        messages,
        readState
      }
    })
  },

  async findThread(
    rootMessageId: MessageId
  ): Promise<Message[]> {
    return withRetry(async () => {
      const response = await postsApi.getThread(rootMessageId)
      return response.data.order
        .filter(id => id !== rootMessageId)
        .map(id => response.data.posts[id])
        .filter((post): post is Post => post !== undefined)
        .map(postToMessage)
    })
  },

  async create(draft: MessageDraft): Promise<Message> {
    return withRetry(async () => {
      const payload: CreatePostRequest = {
        channel_id: draft.channelId,
        message: draft.content,
        root_post_id: draft.rootId,
        file_ids: draft.fileIds,
        client_msg_id: draft.clientId
      }

      const response = await postsApi.create(payload)
      return postToMessage(response.data)
    }, { maxAttempts: 2 }) // Don't retry too many times for creates
  },

  async update(
    id: MessageId,
    changes: { content?: string; isPinned?: boolean }
  ): Promise<Message> {
    return withRetry(async () => {
      if (changes.content !== undefined) {
        const response = await postsApi.update(id, changes.content)
        return postToMessage(response.data)
      }
      
      if (changes.isPinned !== undefined) {
        if (changes.isPinned) {
          await postsApi.pin(id)
        } else {
          await postsApi.unpin(id)
        }
      }
      
      // Fetch updated message
      const response = await postsApi.get(id)
      return postToMessage(response.data)
    })
  },

  async delete(id: MessageId): Promise<void> {
    return withRetry(async () => {
      await postsApi.delete(id)
    })
  },

  async search(
    channelId: ChannelId,
    query: string
  ): Promise<Message[]> {
    return withRetry(async () => {
      const response = await postsApi.list(channelId, { q: query })
      return response.data.messages.map(postToMessage)
    })
  },

  async getPinned(channelId: ChannelId): Promise<Message[]> {
    return withRetry(async () => {
      const response = await postsApi.list(channelId, { is_pinned: true })
      return response.data.messages.map(postToMessage)
    })
  },

  async getSaved(): Promise<Message[]> {
    return withRetry(async () => {
      const response = await postsApi.getSaved()
      return response.data.map(postToMessage)
    })
  },

  async addReaction(messageId: MessageId, emoji: string): Promise<void> {
    await withRetry(() => postsApi.addReaction(messageId, emoji))
  },

  async removeReaction(messageId: MessageId, emoji: string): Promise<void> {
    await withRetry(() => postsApi.removeReaction(messageId, emoji))
  },

  async saveMessage(messageId: MessageId): Promise<void> {
    await withRetry(() => postsApi.save(messageId))
  },

  async unsaveMessage(messageId: MessageId): Promise<void> {
    await withRetry(() => postsApi.unsave(messageId))
  }
}

// Mapper function - converts API response to domain entity
export function postToMessage(post: Post): Message {
  const rawPost = post as Post & {
    root_id?: string
    create_at?: string | number
    pending_post_id?: string
    last_reply_at?: string | number | null
  }

  const rootId = rawPost.root_post_id ?? rawPost.root_id
  const createdAt = normalizeTimestamp(rawPost.created_at ?? rawPost.create_at)

  return {
    id: rawPost.id,
    channelId: rawPost.channel_id,
    userId: rawPost.user_id,
    content: rawPost.message,
    rootId: rootId || undefined,
    replyCount: rawPost.reply_count || 0,
    lastReplyAt: rawPost.last_reply_at 
      ? normalizeTimestamp(rawPost.last_reply_at) 
      : undefined,
    files: (rawPost.files || []).map(f => ({
      id: f.id,
      name: f.name,
      url: f.url,
      size: f.size,
      mimeType: f.mime_type,
      width: (f as unknown as Record<string, unknown>).width as number | undefined,
      height: (f as unknown as Record<string, unknown>).height as number | undefined
    })),
    reactions: (rawPost.reactions || []).map((r: unknown) => {
      const reaction = r as Record<string, unknown>
      return {
        emoji: reaction.emoji as string,
        count: reaction.count as number,
        users: (reaction.users as unknown[]).map((u) => String(u))
      }
    }),
    isPinned: Boolean(rawPost.is_pinned),
    isSaved: rawPost.is_saved || false,
    status: 'delivered',
    clientId: rawPost.client_msg_id ?? rawPost.pending_post_id,
    createdAt,
    type: rawPost.props?.type || '',
    props: rawPost.props
  }
}

function normalizeTimestamp(value: string | number | Date | null | undefined): Date {
  if (!value) return new Date()
  if (value instanceof Date) return value
  if (typeof value === 'number') return new Date(value)
  return new Date(value)
}
