import { defineStore } from 'pinia'
import { ref, computed, type ComputedRef } from 'vue'
import { postsApi, type Post } from '../../../api/posts'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { useUnreadStore } from '@/features/unreads/stores/unreadStore'
import { useAuthStore } from '../../auth/stores/authStore'
import { useTeamStore } from '@/features/teams/stores/teamStore'
import { normalizeEntityId } from '../../../utils/idCompat'
import { getPreferredEmojiName, getReactionEmojiKey } from '../../../utils/emoji'
import { getApiErrorMessage, getErrorMessage } from '../../../core/errors/errorUtils'
import { DEFAULT_MESSAGE_LIMIT } from '../../../constants'

export interface MessageReaction {
  emoji: string
  count: number
  users: string[]
  apiKey?: string
}

export interface Message {
  id: string
  channelId: string
  userId: string
  username: string
  avatarUrl?: string
  email?: string
  content: string
  timestamp: string
  reactions: MessageReaction[]
  threadCount?: number
  lastReplyAt?: string
  rootId?: string
  files?: { id: string; name: string; url: string; size: number; mime_type: string }[]
  isPinned: boolean
  isSaved: boolean
  status?: 'sending' | 'delivered' | 'failed'
  clientMsgId?: string
  props?: any
  seq: number | string
  editedAt?: string
}

function toIsoTimestamp(value: unknown): string {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return new Date(value).toISOString()
  }
  if (typeof value === 'string' && value.length > 0) {
    return value
  }
  return new Date().toISOString()
}

function toOptionalIsoTimestamp(value: unknown): string | undefined {
  if (
    value === null ||
    value === undefined ||
    value === '' ||
    value === 0 ||
    value === '0'
  ) {
    return undefined
  }
  return toIsoTimestamp(value)
}

function comparableId(value: unknown): string | undefined {
  if (typeof value !== 'string' || value.length === 0) {
    return undefined
  }
  return normalizeEntityId(value) ?? value
}

function idsMatch(left: unknown, right: unknown): boolean {
  const lhs = comparableId(left)
  const rhs = comparableId(right)
  return !!lhs && !!rhs && lhs === rhs
}

function stringValue(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

function booleanValue(value: unknown): boolean | undefined {
  return typeof value === 'boolean' ? value : undefined
}

function numberValue(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function resolveAuthorDetails(rawPost: Post & { username?: string; avatar_url?: string; email?: string }): {
  username: string
  avatarUrl?: string
  email?: string
} {
  const usernameFromPost = typeof rawPost.username === 'string' ? rawPost.username.trim() : ''
  if (usernameFromPost) {
    return {
      username: usernameFromPost,
      avatarUrl: rawPost.avatar_url,
      email: rawPost.email,
    }
  }

  const authStore = useAuthStore()
  if (idsMatch(rawPost.user_id, authStore.user?.id)) {
    return {
      username: authStore.user?.display_name || authStore.user?.username || 'Unknown',
      avatarUrl: rawPost.avatar_url || authStore.user?.avatar_url,
      email: rawPost.email || authStore.user?.email,
    }
  }

  const teamStore = useTeamStore()
  const teamMember = teamStore.members.find((member) => idsMatch(member.user_id, rawPost.user_id))
  if (teamMember) {
    return {
      username: teamMember.display_name || teamMember.username || 'Unknown',
      avatarUrl: rawPost.avatar_url || teamMember.avatar_url,
      email: rawPost.email,
    }
  }

  return {
    username: 'Unknown',
    avatarUrl: rawPost.avatar_url,
    email: rawPost.email,
  }
}

function normalizeReactionUsers(users: unknown): string[] {
  if (!Array.isArray(users)) {
    return []
  }

  return Array.from(
    new Set(
      users
        .map((user) => normalizeEntityId(user) ?? user?.toString?.())
        .filter((user): user is string => typeof user === 'string' && user.length > 0)
    )
  )
}

function mergeReaction(target: MessageReaction, incoming: MessageReaction): MessageReaction {
  const users = Array.from(new Set([...target.users, ...incoming.users]))
  return {
    emoji: target.emoji,
    apiKey: target.apiKey || incoming.apiKey,
    users,
    count: users.length > 0 ? users.length : target.count + incoming.count,
  }
}

function normalizeReaction(rawReaction: unknown): MessageReaction | null {
  const r = rawReaction as Record<string, unknown>
  if (!r || typeof r !== 'object' || typeof r.emoji !== 'string') {
    return null
  }

  const emoji = getReactionEmojiKey(r.emoji)
  const users = normalizeReactionUsers(r.users)
  const numericCount = Number(r.count)

  return {
    emoji,
    apiKey: getPreferredEmojiName(r.emoji),
    users,
    count: users.length > 0
      ? users.length
      : (Number.isFinite(numericCount) && numericCount > 0 ? numericCount : 1),
  }
}

function normalizeReactions(rawReactions: unknown): MessageReaction[] {
  if (!Array.isArray(rawReactions)) {
    return []
  }

  const reactionsByEmoji = new Map<string, MessageReaction>()

  for (const rawReaction of rawReactions) {
    const reaction = normalizeReaction(rawReaction)
    if (!reaction) {
      continue
    }

    const existing = reactionsByEmoji.get(reaction.emoji)
    reactionsByEmoji.set(
      reaction.emoji,
      existing ? mergeReaction(existing, reaction) : reaction
    )
  }

  return Array.from(reactionsByEmoji.values())
}

export function postToMessage(post: Post): Message {
  const rawPost = post as Post & {
    root_id?: string
    create_at?: string | number
    update_at?: string | number
    edit_at?: string | number
    edited_at?: string | number
    pending_post_id?: string
    last_reply_at?: string | number | null
  }
  const rootId = (rawPost.root_post_id ?? rawPost.root_id) || undefined
  const author = resolveAuthorDetails(rawPost)

  return {
    id: rawPost.id,
    channelId: rawPost.channel_id,
    userId: rawPost.user_id,
    username: author.username,
    avatarUrl: author.avatarUrl,
    email: author.email,
    content: rawPost.message,
    timestamp: toIsoTimestamp(rawPost.created_at ?? rawPost.create_at),
    reactions: normalizeReactions(rawPost.reactions),
    rootId,
    threadCount: rawPost.reply_count || 0,
    lastReplyAt: toOptionalIsoTimestamp(rawPost.last_reply_at),
    files: rawPost.files || [],
    isPinned: Boolean(rawPost.is_pinned),
    isSaved: rawPost.is_saved || false,
    status: 'delivered',
    clientMsgId: rawPost.client_msg_id ?? rawPost.pending_post_id,
    props: rawPost.props,
    seq: rawPost.seq ?? 0,
    editedAt: toOptionalIsoTimestamp(rawPost.edited_at ?? rawPost.edit_at),
  }
}

export const useMessageStore = defineStore('messageStore', () => {
  // Messages grouped by channel
  const messagesByChannel = ref<Record<string, Message[]>>({})
  const repliesByThread = ref<Record<string, Message[]>>({})
  const hasMoreOlderByChannel = ref<Record<string, boolean>>({})
  const loading = ref(false)
  const isLoadingOlder = ref(false)
  const error = ref<string | null>(null)

  // Cache computed refs per channelId so Vue can cache reactivity across callers
  const messagesCache = new Map<string, ComputedRef<Message[]>>()

  function getMessages(channelId: string) {
    if (!messagesCache.has(channelId)) {
      messagesCache.set(
        channelId,
        computed(() => messagesByChannel.value[channelId] || [])
      )
    }
    return messagesCache.get(channelId)!
  }

  const hasMoreOlder = computed(() => (channelId: string) => hasMoreOlderByChannel.value[channelId] ?? true)

  function visitLoadedMessages(visitor: (message: Message, collection: Message[]) => void) {
    for (const messages of Object.values(messagesByChannel.value)) {
      if (!messages) {
        continue
      }

      for (const message of messages) {
        visitor(message, messages)
      }
    }

    for (const replies of Object.values(repliesByThread.value)) {
      if (!replies) {
        continue
      }

      for (const reply of replies) {
        visitor(reply, replies)
      }
    }
  }

  async function fetchMessages(channelId: string) {
    loading.value = true
    error.value = null
    try {
      const unreadStore = useUnreadStore()
      const response = await postsApi.list(channelId, { limit: DEFAULT_MESSAGE_LIMIT })
      const messages = response.data.messages
        .filter(p => !p.root_post_id)
        .map(postToMessage)
        .reverse()
      messagesByChannel.value[channelId] = messages

      // Update read state in unread store
      if (response.data.read_state) {
        unreadStore.setReadState(channelId, response.data.read_state)
      }

      // If we got fewer than 50, we probably reached the end
      hasMoreOlderByChannel.value[channelId] = response.data.messages.length >= DEFAULT_MESSAGE_LIMIT
    } catch (e: unknown) {
      console.error(`Failed to fetch messages for channel ${channelId}:`, e);
      error.value = getApiErrorMessage(e) || getErrorMessage(e) || 'Failed to fetch messages'
    } finally {
      loading.value = false
    }
  }

  async function fetchOlderMessages(channelId: string) {
    if (loading.value || !hasMoreOlder.value(channelId)) return

    const currentMessages = messagesByChannel.value[channelId] || []
    if (currentMessages.length === 0) {
      await fetchMessages(channelId)
      return
    }

    // Use the ID of the OLDEST message as the cursor
    const before = currentMessages[0]!.id

    loading.value = true
    isLoadingOlder.value = true
    try {
      const response = await postsApi.list(channelId, { before, limit: DEFAULT_MESSAGE_LIMIT })
      const olderMessages = response.data.messages
        .filter(p => !p.root_post_id)
        .map(postToMessage)
        .reverse()

      if (olderMessages.length > 0) {
        messagesByChannel.value[channelId] = [...olderMessages, ...currentMessages]
      }

      hasMoreOlderByChannel.value[channelId] = response.data.messages.length >= DEFAULT_MESSAGE_LIMIT
    } catch (e: unknown) {
      console.error('Failed to fetch older messages:', e)
    } finally {
      loading.value = false
      isLoadingOlder.value = false
    }
  }

  async function fetchThread(rootId: string) {
    loading.value = true
    error.value = null
    try {
      const response = await postsApi.getThread(rootId)
      const replies = response.data.order
        .filter(id => id !== rootId)
        .map(id => response.data.posts[id])
        .filter((post): post is Post => post !== undefined)
        .map(postToMessage)
      repliesByThread.value[rootId] = replies
    } catch (e: unknown) {
      console.error(`Failed to fetch thread ${rootId}:`, e);
      error.value = getApiErrorMessage(e) || getErrorMessage(e) || 'Failed to fetch thread'
    } finally {
      loading.value = false
    }
  }

  // Optimized for WebSocket usage
  function addOptimisticMessage(message: Message) {
    if (message.rootId) {
      if (!repliesByThread.value[message.rootId]) {
        repliesByThread.value[message.rootId] = []
      }
      repliesByThread.value[message.rootId]?.push(message)
    } else {
      if (!messagesByChannel.value[message.channelId]) {
        messagesByChannel.value[message.channelId] = []
      }
      messagesByChannel.value[message.channelId]?.push(message)
    }
  }

  function updateOptimisticMessage(clientMsgId: string, serverMsg: Message) {
    const channelId = serverMsg.channelId
    const rootId = serverMsg.rootId

    if (rootId) {
      const threadReplies = repliesByThread.value[rootId]
      if (threadReplies) {
        const index = threadReplies.findIndex(m => m.clientMsgId === clientMsgId || m.id === clientMsgId)
        if (index !== -1) {
          threadReplies[index] = serverMsg
        } else {
          threadReplies.push(serverMsg)
        }
      }
    } else {
      const channelMessages = messagesByChannel.value[channelId]
      if (channelMessages) {
        const index = channelMessages.findIndex(m => m.clientMsgId === clientMsgId || m.id === clientMsgId)
        if (index !== -1) {
          channelMessages[index] = serverMsg
        } else {
          channelMessages.push(serverMsg)
        }
      }
    }
  }

  function handleNewMessage(post: Post) {
    if (!post) {
      return
    }

    const message = postToMessage(post)

    if (message.rootId) {
      // Handle reply
      if (!repliesByThread.value[message.rootId]) {
        repliesByThread.value[message.rootId] = []
      }
      const threadReplies = repliesByThread.value[message.rootId]
      if (threadReplies) {
        const index = threadReplies.findIndex(m => m.id === message.id || (m.clientMsgId && m.clientMsgId === message.clientMsgId))
        if (index !== -1) {
          threadReplies[index] = message
        } else {
          threadReplies.push(message)
        }
      }

      // Important: If it was accidentally added to the main channel feed (e.g. by a bug in optimistic logic), remove it
      const channelMessages = messagesByChannel.value[message.channelId]
      if (channelMessages) {
        const idx = channelMessages.findIndex(m => m.id === message.id || (m.clientMsgId && m.clientMsgId === message.clientMsgId))
        if (idx !== -1) {
          channelMessages.splice(idx, 1)
        }
      }
    } else {
      // Handle root message
      if (!messagesByChannel.value[message.channelId]) {
        messagesByChannel.value[message.channelId] = []
      }
      const channelMessages = messagesByChannel.value[message.channelId]
      if (channelMessages) {
        const index = channelMessages.findIndex(m => m.id === message.id || (m.clientMsgId && m.clientMsgId === message.clientMsgId))
        if (index !== -1) {
          channelMessages[index] = message
        } else {
          channelMessages.push(message)
        }
      }
    }

    // Handle notifications
    const channelStore = useChannelStore()
    const authStore = useAuthStore()

    // Unread message counts are authoritative from websocket unread events.
    // Keep only local mention hinting for now.
    if (channelStore.currentChannelId !== message.channelId) {
      const unreadStore = useUnreadStore()

      // Check for mention
      const currentUser = authStore.user
      if (currentUser && message.content.includes(`@${currentUser.username}`)) {
        unreadStore.channelMentions[message.channelId] = (unreadStore.channelMentions[message.channelId] || 0) + 1
      }
    }
  }

  function clearMessages(channelId?: string) {
    if (channelId) {
      delete messagesByChannel.value[channelId]
      messagesCache.delete(channelId)
    } else {
      messagesByChannel.value = {}
      messagesCache.clear()
    }
  }

  function resetSessionState() {
    messagesByChannel.value = {}
    repliesByThread.value = {}
    hasMoreOlderByChannel.value = {}
    loading.value = false
    isLoadingOlder.value = false
    error.value = null
    messagesCache.clear()
  }

  async function pinMessage(messageId: string, channelId: string) {
    try {
      await postsApi.pin(messageId)
      // Update local state
      const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
      if (message) {
        message.isPinned = true
      }
    } catch (e: unknown) {
      error.value = 'Failed to pin message'
      throw e
    }
  }

  async function unpinMessage(messageId: string, channelId: string) {
    try {
      await postsApi.unpin(messageId)
      // Update local state
      const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
      if (message) {
        message.isPinned = false
      }
    } catch (e: unknown) {
      error.value = 'Failed to unpin message'
      throw e
    }
  }

  async function saveMessage(messageId: string, channelId: string) {
    try {
      await postsApi.save(messageId)
      // Update local state
      const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
      if (message) {
        message.isSaved = true
      }
    } catch (e: unknown) {
      error.value = 'Failed to save message'
      throw e
    }
  }

  async function unsaveMessage(messageId: string, channelId: string) {
    try {
      await postsApi.unsave(messageId)
      // Update local state
      const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
      if (message) {
        message.isSaved = false
      }
    } catch (e: unknown) {
      error.value = 'Failed to unsave message'
      throw e
    }
  }

  async function searchMessages(channelId: string, query: string) {
    loading.value = true
    error.value = null
    try {
      const response = await postsApi.list(channelId, { q: query })
      return response.data.messages.map(postToMessage)
    } catch (e: unknown) {
      error.value = 'Failed to search messages'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchPinnedMessages(channelId: string) {
    loading.value = true
    error.value = null
    try {
      const response = await postsApi.list(channelId, { is_pinned: true })
      return response.data.messages.map(postToMessage)
    } catch (e: unknown) {
      error.value = 'Failed to fetch pinned messages'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchSavedMessages() {
    loading.value = true
    try {
      const response = await postsApi.getSaved()
      return response.data.map(postToMessage)
    } catch (e: unknown) {
      error.value = 'Failed to fetch saved messages'
      throw e
    } finally {
      loading.value = false
    }
  }

  function handleMessageUpdate(data: Record<string, unknown>) {
    const id = stringValue(data.id)
    if (!id) return

    // 1. Update in main channels
    for (const cid in messagesByChannel.value) {
      const messages = messagesByChannel.value[cid]
      if (!messages) continue

      const index = messages.findIndex(m => m.id === id)
      if (index !== -1) {
        const msg = messages[index]
        if (!msg) continue

        const message = stringValue(data.message)
        const isPinned = booleanValue(data.is_pinned)
        const replyCount = numberValue(data.reply_count)
        const replyCountInc = numberValue(data.reply_count_inc)
        if (message !== undefined) msg.content = message
        if (isPinned !== undefined) msg.isPinned = isPinned
        if (replyCount !== undefined) msg.threadCount = replyCount
        if (data.edited_at !== undefined || data.edit_at !== undefined) {
          msg.editedAt = toOptionalIsoTimestamp(data.edited_at ?? data.edit_at)
        }
        if (replyCountInc) {
          msg.threadCount = (msg.threadCount || 0) + replyCountInc
        }
      }
    }

    // 2. Update in cached threads
    for (const rootId in repliesByThread.value) {
      const replies = repliesByThread.value[rootId]
      if (!replies) continue

      const index = replies.findIndex(m => m.id === id)
      if (index !== -1) {
        const msg = replies[index]
        if (!msg) continue

        const message = stringValue(data.message)
        const isPinned = booleanValue(data.is_pinned)
        if (message !== undefined) msg.content = message
        if (isPinned !== undefined) msg.isPinned = isPinned
        if (data.edited_at !== undefined || data.edit_at !== undefined) {
          msg.editedAt = toOptionalIsoTimestamp(data.edited_at ?? data.edit_at)
        }
      }
    }
  }

  function handleMessageDelete(messageId: string) {
    // 1. Remove from main channels
    for (const cid in messagesByChannel.value) {
      const messages = messagesByChannel.value[cid]
      if (messages) {
        const index = messages.findIndex(m => m.id === messageId)
        if (index !== -1) {
          messages.splice(index, 1)
        }
      }
    }

    // 2. Remove from cached threads
    for (const rootId in repliesByThread.value) {
      const replies = repliesByThread.value[rootId]
      if (replies) {
        const index = replies.findIndex(m => m.id === messageId)
        if (index !== -1) {
          replies.splice(index, 1)
        }
      }
    }
  }

  function handleReactionAdded(data: Record<string, unknown>) {
    const postId = stringValue(data.post_id)
    const userId = stringValue(data.user_id)
    if (!postId || !userId) return

    const emojiName = stringValue(data.emoji_name) ?? ''
    const reactionKey = getReactionEmojiKey(emojiName)
    const apiKey = getPreferredEmojiName(emojiName)

    visitLoadedMessages((message) => {
      if (message.id !== postId) {
        return
      }

      const existingReaction = message.reactions.find((reaction) => reaction.emoji === reactionKey)
      if (existingReaction) {
        if (!existingReaction.users.includes(userId)) {
          existingReaction.users.push(userId)
        }
        existingReaction.count = existingReaction.users.length || existingReaction.count + 1
        existingReaction.apiKey = existingReaction.apiKey || apiKey
      } else {
        message.reactions.push({
          emoji: reactionKey,
          apiKey,
          count: 1,
          users: [userId],
        })
      }
    })
  }

  function handleReactionRemoved(data: Record<string, unknown>) {
    const postId = stringValue(data.post_id)
    const userId = stringValue(data.user_id)
    if (!postId || !userId) return

    const emojiName = stringValue(data.emoji_name) ?? ''
    const reactionKey = getReactionEmojiKey(emojiName)

    visitLoadedMessages((message) => {
      if (message.id !== postId) {
        return
      }

      const index = message.reactions.findIndex((reaction) => reaction.emoji === reactionKey)
      if (index === -1) {
        return
      }

      const reaction = message.reactions[index]
      if (!reaction) {
        return
      }

      const userIndex = reaction.users.indexOf(userId)
      if (userIndex !== -1) {
        reaction.users.splice(userIndex, 1)
      }

      reaction.count = reaction.users.length > 0 ? reaction.users.length : reaction.count - 1
      if (reaction.count <= 0) {
        message.reactions.splice(index, 1)
      }
    })
  }

  // Feature store compatibility actions
  function setMessages(channelId: string, messages: Message[]) {
    messagesByChannel.value[channelId] = messages
  }

  function prependMessages(channelId: string, messages: Message[]) {
    const existing = messagesByChannel.value[channelId] || []
    messagesByChannel.value[channelId] = [...messages, ...existing]
  }

  function addMessage(channelId: string, message: Message) {
    const existing = messagesByChannel.value[channelId] || []
    messagesByChannel.value[channelId] = [...existing, message]
  }

  function patchMessage(messageId: string, updates: Partial<Message>) {
    for (const messages of Object.values(messagesByChannel.value)) {
      const message = messages.find(m => m.id === messageId)
      if (message) {
        Object.assign(message, updates)
        return
      }
    }
  }

  function removeMessage(channelId: string, messageId: string) {
    const messages = messagesByChannel.value[channelId]
    if (messages) {
      const index = messages.findIndex(m => m.id === messageId)
      if (index !== -1) {
        messages.splice(index, 1)
      }
    }
  }

  function replaceOptimisticMessage(channelId: string, clientId: string, message: Message) {
    const messages = messagesByChannel.value[channelId]
    if (messages) {
      const index = messages.findIndex(m => m.clientMsgId === clientId)
      if (index !== -1) {
        messages[index] = message
      }
    }
  }

  function markMessageFailed(channelId: string, clientId: string) {
    const messages = messagesByChannel.value[channelId]
    if (messages) {
      const message = messages.find(m => m.clientMsgId === clientId)
      if (message) {
        message.status = 'failed'
      }
    }
  }

  function setThreadReplies(rootId: string, replies: Message[]) {
    repliesByThread.value[rootId] = replies
  }

  function addThreadReply(rootId: string, reply: Message) {
    const existing = repliesByThread.value[rootId] || []
    repliesByThread.value[rootId] = [...existing, reply]
  }

  function replaceOptimisticThreadReply(rootId: string, clientId: string, message: Message) {
    const replies = repliesByThread.value[rootId]
    if (replies) {
      const index = replies.findIndex(m => m.clientMsgId === clientId)
      if (index !== -1) {
        replies[index] = message
      }
    }
  }

  function setThreadLoading(rootId: string, isLoading: boolean) {
    // No-op: legacy store doesn't have this, but feature service may call it
    void rootId
    void isLoading
  }

  function addReaction(messageId: string, emoji: string, userId: string) {
    for (const messages of Object.values(messagesByChannel.value)) {
      const message = messages.find(m => m.id === messageId)
      if (message) {
        const reaction = message.reactions.find(r => r.emoji === emoji)
        if (reaction) {
          if (!reaction.users.includes(userId)) {
            reaction.users.push(userId)
            reaction.count++
          }
        } else {
          message.reactions.push({ emoji, count: 1, users: [userId] })
        }
        return
      }
    }
  }

  function removeReaction(messageId: string, emoji: string, userId: string) {
    for (const messages of Object.values(messagesByChannel.value)) {
      const message = messages.find(m => m.id === messageId)
      if (message) {
        const index = message.reactions.findIndex(r => r.emoji === emoji)
        if (index !== -1) {
          const reaction = message.reactions[index]
          if (reaction) {
            const userIndex = reaction.users.indexOf(userId)
            if (userIndex !== -1) {
              reaction.users.splice(userIndex, 1)
              reaction.count--
              if (reaction.count <= 0) {
                message.reactions.splice(index, 1)
              }
            }
          }
        }
        return
      }
    }
  }

  function addOptimisticReaction(messageId: string, emoji: string, userId: string) {
    addReaction(messageId, emoji, userId)
  }

  function setLoading(value: boolean) {
    loading.value = value
  }

  function setLoadingOlder(value: boolean) {
    isLoadingOlder.value = value
  }

  function setHasMoreOlder(channelId: string, value: boolean) {
    hasMoreOlderByChannel.value[channelId] = value
  }

  function setError(err: string | null) {
    error.value = err
  }

  function clearError() {
    error.value = null
  }

  function clearChannel(channelId: string) {
    delete messagesByChannel.value[channelId]
    delete hasMoreOlderByChannel.value[channelId]
    messagesCache.delete(channelId)
  }

  function updateMessage(message: Message) {
    patchMessage(message.id, message)
  }

  function findMessageByClientId(channelId: string, clientId: string) {
    return messagesByChannel.value[channelId]?.find(m => m.clientMsgId === clientId)
  }

  function getMessageById(channelId: string, messageId: string) {
    return messagesByChannel.value[channelId]?.find(m => m.id === messageId)
  }

  return {
    // State
    messagesByChannel,
    repliesByThread,
    loading,
    isLoadingOlder,
    error,

    // Getters
    getMessages,
    hasMoreOlder,

    // Legacy actions
    fetchMessages,
    fetchThread,
    addOptimisticMessage,
    updateOptimisticMessage,
    handleNewMessage,
    handleMessageUpdate,
    handleMessageDelete,
    handleReactionAdded,
    handleReactionRemoved,
    clearMessages,
    resetSessionState,
    pinMessage,
    unpinMessage,
    saveMessage,
    unsaveMessage,
    fetchSavedMessages,
    fetchPinnedMessages,
    searchMessages,
    fetchOlderMessages,

    // Feature actions
    setMessages,
    prependMessages,
    addMessage,
    patchMessage,
    removeMessage,
    replaceOptimisticMessage,
    markMessageFailed,
    setThreadReplies,
    addThreadReply,
    replaceOptimisticThreadReply,
    setThreadLoading,
    addReaction,
    removeReaction,
    addOptimisticReaction,
    setLoading,
    setLoadingOlder,
    setHasMoreOlder,
    setError,
    clearError,
    clearChannel,
    updateMessage,
    findMessageByClientId,
    getMessageById
  }
})
