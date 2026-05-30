import type { UserId, UserRef } from './User'
import type { ChannelId } from './Channel'

export type MessageId = string
export type MessageStatus = 'sending' | 'delivered' | 'failed'

export interface FileAttachment {
  id: string
  name: string
  url: string
  size: number
  mimeType: string
  width?: number
  height?: number
}

export interface Reaction {
  emoji: string
  count: number
  users: UserId[]
  didReact?: boolean // Computed for current user
}

export interface Message {
  id: MessageId
  channelId: ChannelId

  // Author - reference by ID, details in separate store
  userId: UserId
  author?: UserRef // Optional denormalized data

  // Content
  content: string
  html?: string // Rendered markdown

  // Threading
  rootId?: MessageId
  replyCount: number
  lastReplyAt?: Date
  participants?: UserId[] // Latest thread participants

  // Media
  files: FileAttachment[]

  // Engagement
  reactions: Reaction[]
  isPinned: boolean

  // Metadata
  editedAt?: Date
  updatedAt?: Date // Alias for editedAt
  createdAt: Date

  // Client state
  clientId?: string // For optimistic updates
  status?: MessageStatus
  isSaved?: boolean

  // Extended props (for integrations, calls, etc)
  props?: Record<string, any>
  type?:
    | ''
    | 'system_join_leave'
    | 'system_purpose'
    | 'system_header'
    | 'system_channel_archived'
    | 'system_channel_restored'
    | 'calls'
}

// For optimistic updates
export interface MessageDraft {
  channelId: ChannelId
  content: string
  rootId?: MessageId
  fileIds?: string[]
  clientId?: string
  props?: Record<string, any>
}

// Helper to check if message is a thread reply
export function isThreadReply(message: Message): boolean {
  return !!message.rootId && message.rootId !== message.id
}

// Helper to check if message is a root post with replies
export function hasReplies(message: Message): boolean {
  return message.replyCount > 0 && !message.rootId
}
