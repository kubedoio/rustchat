export type ChatMember = {
  id?: string
  user_id?: string
  username: string
  displayName?: string
  display_name?: string
}

export type ChatAttachment = {
  id: string
  name?: string
  size?: number
  file?: File
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
