export type ChatMember = {
  id?: string
  user_id?: string
  username: string
  displayName?: string
  display_name?: string
  presence?: 'online' | 'away' | 'dnd' | 'offline'
  statusText?: string | null
  status_text?: string | null
  statusEmoji?: string | null
  status_emoji?: string | null
}

export type ChatAttachment = {
  id: string
  name?: string
  size?: number
  file?: File
  mimeType?: string
  mime_type?: string
  url?: string
  fileId?: string
  uploading?: boolean
  uploadError?: boolean
  progress?: number
}

export type ChatReaction = {
  emoji: string
  count: number
  users: string[]
}

export type ChatMessage = {
  id: string
  channelId?: string
  channel_id?: string
  authorName?: string
  user_id?: string
  body?: string
  message?: string
  createdAt?: string | Date
  created_at?: string
  attachments?: ChatAttachment[]
  files?: ChatAttachment[]
  status?: 'sending' | 'failed' | 'delivered'
  editedAt?: string
  isPinned?: boolean
  isSaved?: boolean
  reactions?: ChatReaction[]
  threadCount?: number
  lastReplyAt?: string
  props?: Record<string, unknown>
  avatarUrl?: string
  username?: string
}

export type ChatChannel = {
  id: string
  name: string
  displayName?: string
  display_name?: string
  team_id?: string
  unreadCount?: number
}

export type ChatTeam = {
  id: string
  name: string
  displayName?: string
  display_name?: string
  channels?: ChatChannel[]
}

export type ComposerSubmit = {
  channelId: string
  content: string
  body: string
  attachments: ChatAttachment[]
  file_ids?: string[]
}
