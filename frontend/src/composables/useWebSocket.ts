import { log } from '@/utils/log';
import { ref } from 'vue'
import { useAuthStore } from '../features/auth/stores/authStore'
import { useMessageStore } from '@/features/messages/stores/messageStore'
import { postToMessage } from '@/features/messages/stores/messageStore'
import type { Message } from '@/features/messages/stores/messageStore'
import { usePresenceStore } from '../features/presence'
import type { PresenceStatus } from '../core/entities/User'
import { useUnreadStore } from '@/features/unreads/stores/unreadStore'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { useToast } from './useToast'
import { postsApi, type ChannelUnreadAt, type Post } from '../api/posts'
import type { Channel } from '../api/channels'
import { normalizeEntityId, normalizeIdsDeep } from '../utils/idCompat'
import { applyUserStatusSnapshot } from './useUserSummary'
import {
  MAX_RECONNECT_ATTEMPTS,
  WS_DISCONNECTED_TIMEOUT,
  WS_FAILED_TIMEOUT,
  RECONNECT_DELAY_BASE_MS,
  RECONNECT_DELAY_MAX_MS,
} from '../constants'


// Type for WebSocket listener callbacks
type WsListener = (data: unknown) => void

// Server -> Client
export interface WsEnvelope {
    type: 'event' | 'response' | 'error' | 'ack'
    event: string
    seq?: number
    channel_id?: string
    broadcast?: {
        channel_id?: string
        team_id?: string
        user_id?: string
    }
    data: unknown
}

// Client -> Server
export interface ClientEnvelope {
    type: 'command'
    event: string
    data: unknown
    channel_id?: string
    client_msg_id?: string
    seq?: number
}

// Singleton state
const ws = ref<WebSocket | null>(null)
const connected = ref(false)
const reconnectAttempts = ref(0)
const maxReconnectAttempts = MAX_RECONNECT_ATTEMPTS
const subscriptions = ref<Set<string>>(new Set())
const listeners = ref<Record<string, Set<WsListener>>>({})
let actionSeq = 1
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

// Connection state management
const connectionStatus = ref<'connected' | 'reconnecting' | 'disconnected' | 'failed'>('connected')
const disconnectedAt = ref<Date | null>(null)
const reconnectAttempt = ref(0)
const nextRetryIn = ref(0)
const connectionError = ref<string | null>(null)

// Constants
const STATE_TRANSITION_MS = {
  TO_DISCONNECTED: WS_DISCONNECTED_TIMEOUT,
  TO_FAILED: WS_FAILED_TIMEOUT,
}

// Countdown timer
let countdownTimer: ReturnType<typeof setInterval> | null = null

function startCountdown() {
  stopCountdown()
  nextRetryIn.value = Math.min(
    RECONNECT_DELAY_BASE_MS * Math.pow(1.5, reconnectAttempt.value),
    RECONNECT_DELAY_MAX_MS
  ) / 1000

  countdownTimer = setInterval(() => {
    if (nextRetryIn.value > 0) {
      nextRetryIn.value--
    } else {
      stopCountdown()
    }
  }, 1000)
}

function stopCountdown() {
  if (countdownTimer) {
    clearInterval(countdownTimer)
    countdownTimer = null
  }
}

function updateConnectionStatus() {
  if (connected.value) {
    connectionStatus.value = 'connected'
    disconnectedAt.value = null
    connectionError.value = null
    stopCountdown()
    return
  }

  if (!disconnectedAt.value) {
    connectionStatus.value = 'reconnecting'
    return
  }

  const disconnectedMs = Date.now() - disconnectedAt.value.getTime()

  if (disconnectedMs >= STATE_TRANSITION_MS.TO_FAILED || reconnectAttempt.value >= MAX_RECONNECT_ATTEMPTS) {
    connectionStatus.value = 'failed'
    stopCountdown()
  } else if (disconnectedMs >= STATE_TRANSITION_MS.TO_DISCONNECTED) {
    connectionStatus.value = 'disconnected'
  } else {
    connectionStatus.value = 'reconnecting'
  }
}

function clearReconnectTimer() {
    if (reconnectTimer) {
        clearTimeout(reconnectTimer)
        reconnectTimer = null
    }
}

function isAuthExpiryCloseEvent(event: CloseEvent): boolean {
    const reason = (event.reason || '').toLowerCase()
    return (
        (event.code === 1008 && reason.includes('token')) ||
        reason.includes('authentication token expired') ||
        reason.includes('token expired')
    )
}

function normalizeWsTimestamp(value: unknown, fallback: string): string {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return new Date(value).toISOString()
    }
    if (typeof value === 'string' && value.length > 0) {
        return value
    }
    return fallback
}

function extractWsPostPayload(data: unknown): Record<string, unknown> | null {
    if (!data || typeof data !== 'object') {
        return null
    }

    if ('post' in data) {
        const wrappedPost = (data as Record<string, unknown>).post
        if (typeof wrappedPost === 'string') {
            try {
                const parsed = JSON.parse(wrappedPost)
                return parsed && typeof parsed === 'object' ? parsed as Record<string, unknown> : null
            } catch {
                return null
            }
        }
        if (wrappedPost && typeof wrappedPost === 'object') {
            return wrappedPost as Record<string, unknown>
        }
        return null
    }

    return data as Record<string, unknown>
}

function normalizeWsPost(data: unknown, envelopeChannelId?: string): Post | null {
    const rawPost = extractWsPostPayload(data)
    if (!rawPost || typeof rawPost.id !== 'string') {
        return null
    }
    const normalizedRawPost = normalizeIdsDeep(rawPost) as Record<string, unknown>

    const fallbackTimestamp = new Date().toISOString()
    const createdAt = normalizeWsTimestamp(normalizedRawPost.created_at ?? normalizedRawPost.create_at, fallbackTimestamp)
    const updatedAt = normalizeWsTimestamp(normalizedRawPost.updated_at ?? normalizedRawPost.update_at, createdAt)
    const postId = normalizeEntityId(normalizedRawPost.id) ?? (normalizedRawPost.id as string)
    const channelId = normalizeEntityId(normalizedRawPost.channel_id)
        ?? normalizeEntityId(envelopeChannelId)
        ?? (normalizedRawPost.channel_id as string | undefined)
        ?? envelopeChannelId
    const userId = normalizeEntityId(normalizedRawPost.user_id) ?? (normalizedRawPost.user_id as string | undefined)
    const rootPostId = normalizeEntityId(normalizedRawPost.root_post_id ?? normalizedRawPost.root_id)
        ?? (normalizedRawPost.root_post_id as string | undefined)
        ?? (normalizedRawPost.root_id as string | undefined)

    return {
        ...normalizedRawPost,
        id: postId,
        channel_id: channelId,
        user_id: userId,
        message: typeof normalizedRawPost.message === 'string' ? normalizedRawPost.message : '',
        root_post_id: rootPostId,
        root_id: rootPostId,
        created_at: createdAt,
        updated_at: updatedAt,
        client_msg_id: (normalizedRawPost.client_msg_id ?? normalizedRawPost.pending_post_id) as string | undefined,
        file_ids: Array.isArray(normalizedRawPost.file_ids)
            ? normalizedRawPost.file_ids.map((id: unknown) => normalizeEntityId(id) ?? id)
            : [],
        is_pinned: typeof normalizedRawPost.is_pinned === 'boolean' ? normalizedRawPost.is_pinned : false,
        seq: (normalizedRawPost.seq as number | undefined) ?? 0,
    } as unknown as Post
}

function normalizeWsEnvelope(envelope: WsEnvelope): WsEnvelope {
    const broadcastChannelId = normalizeEntityId(envelope.broadcast?.channel_id) ?? envelope.broadcast?.channel_id
    const channelId = normalizeEntityId(envelope.channel_id) ?? envelope.channel_id ?? broadcastChannelId

    return {
        ...envelope,
        channel_id: channelId,
        data: normalizeIdsDeep(envelope.data),
        broadcast: envelope.broadcast
            ? {
                ...envelope.broadcast,
                channel_id: broadcastChannelId,
                team_id: normalizeEntityId(envelope.broadcast.team_id) ?? envelope.broadcast.team_id,
                user_id: normalizeEntityId(envelope.broadcast.user_id) ?? envelope.broadcast.user_id,
            }
            : envelope.broadcast,
    }
}

function normalizeWsReactionPayload(data: unknown): Record<string, unknown> | null {
    if (!data || typeof data !== 'object') {
        return null
    }

    let rawReaction: unknown = data
    if ('reaction' in data) {
        rawReaction = (data as Record<string, unknown>).reaction
    }

    if (typeof rawReaction === 'string') {
        try {
            rawReaction = JSON.parse(rawReaction)
        } catch {
            return null
        }
    }

    if (!rawReaction || typeof rawReaction !== 'object') {
        return null
    }

    const normalized = normalizeIdsDeep(rawReaction) as Record<string, unknown>
    const emojiName = typeof normalized.emoji_name === 'string'
        ? normalized.emoji_name
        : typeof normalized.emoji === 'string'
            ? normalized.emoji
            : undefined

    if (!emojiName) {
        return null
    }

    return {
        ...normalized,
        post_id: normalizeEntityId(normalized.post_id) ?? normalized.post_id,
        user_id: normalizeEntityId(normalized.user_id) ?? normalized.user_id,
        emoji_name: emojiName,
    }
}

function normalizeWsChannelType(value: unknown): Channel['channel_type'] {
    const raw = String(value || '').toLowerCase()
    if (raw === 'o' || raw === 'public') return 'public'
    if (raw === 'p' || raw === 'private') return 'private'
    if (raw === 'd' || raw === 'direct') return 'direct'
    if (raw === 'g' || raw === 'group') return 'group'
    return 'public'
}

function normalizeWsChannelPayload(data: unknown): Channel | null {
    if (!data || typeof data !== 'object') {
        return null
    }

    const dataObj = data as Record<string, unknown>
    const id = normalizeEntityId(dataObj.id) ?? (dataObj.id as string | undefined)
    const teamId = normalizeEntityId(dataObj.team_id) ?? (dataObj.team_id as string | undefined)
    if (typeof id !== 'string' || !id || typeof teamId !== 'string' || !teamId) {
        return null
    }

    const displayName = typeof dataObj.display_name === 'string' && dataObj.display_name
        ? dataObj.display_name
        : (typeof dataObj.name === 'string' ? dataObj.name : '')
    const createdAtRaw = dataObj.created_at ?? dataObj.create_at

    return {
        id,
        team_id: teamId,
        name: typeof dataObj.name === 'string' ? dataObj.name : '',
        display_name: displayName,
        channel_type: normalizeWsChannelType(dataObj.channel_type ?? dataObj.type),
        header: typeof dataObj.header === 'string' ? dataObj.header : '',
        purpose: typeof dataObj.purpose === 'string' ? dataObj.purpose : '',
        created_at: normalizeWsTimestamp(createdAtRaw, new Date().toISOString()),
        creator_id: normalizeEntityId(dataObj.creator_id) ?? (dataObj.creator_id as string | undefined) ?? '',
        unreadCount: typeof dataObj.unreadCount === 'number' ? dataObj.unreadCount : 0,
        mentionCount: typeof dataObj.mentionCount === 'number' ? dataObj.mentionCount : 0,
    }
}

function normalizeWsPresence(value: unknown): PresenceStatus {
    const raw = String(value || '').toLowerCase()
    if (raw === 'online' || raw === 'away' || raw === 'dnd' || raw === 'offline') {
        return raw
    }
    return 'offline'
}

// Raw status data from WebSocket
interface RawStatusData {
    user_id?: unknown
    status?: unknown
    text?: unknown
    emoji?: unknown
    expires_at?: unknown
}

// Raw unread data from WebSocket
interface RawUnreadData {
    channel_id?: unknown
    msg_count?: unknown
    unread_count?: unknown
    mention_count?: unknown
}

export function useWebSocket() {
    const authStore = useAuthStore()
    const messageStore = useMessageStore()
    const presenceStore = usePresenceStore()
    const unreadStore = useUnreadStore()
    const channelStore = useChannelStore()
    const toast = useToast()

    function applyInitialLoadSnapshot(data: unknown) {
        if (!data || typeof data !== 'object') {
            return
        }

        const dataObj = data as Record<string, unknown>

        if (Array.isArray(dataObj.channels)) {
            dataObj.channels.forEach((rawChannel: unknown) => {
                const channel = normalizeWsChannelPayload(rawChannel)
                if (channel) {
                    channelStore.addChannel(channel)
                }
            })
        }

        if (Array.isArray(dataObj.channel_unreads)) {
            dataObj.channel_unreads.forEach((rawUnread: unknown) => {
                const unreadData = rawUnread as RawUnreadData
                const channelId = normalizeEntityId(unreadData.channel_id) ?? (unreadData.channel_id as string | undefined)
                if (typeof channelId !== 'string' || !channelId) {
                    return
                }

                const unreadCount = Number(unreadData.msg_count ?? unreadData.unread_count ?? 0)
                const mentionCount = Number(unreadData.mention_count ?? 0)
                unreadStore.channelUnreads[channelId] = Number.isFinite(unreadCount) ? unreadCount : 0
                unreadStore.channelMentions[channelId] = Number.isFinite(mentionCount) ? mentionCount : 0
            })
        }

        if (Array.isArray(dataObj.statuses)) {
            dataObj.statuses.forEach((rawStatus: unknown) => {
                const statusData = rawStatus as RawStatusData
                const userId = normalizeEntityId(statusData.user_id) ?? (statusData.user_id as string | undefined)
                const status = normalizeWsPresence(statusData.status)
                if (typeof userId === 'string' && userId) {
                    presenceStore.updatePresenceFromEvent(userId, status)
                    applyUserStatusSnapshot({
                        userId,
                        presence: status as PresenceStatus,
                        statusText: (statusData.text ?? null) as string | null,
                        statusEmoji: (statusData.emoji ?? null) as string | null,
                        statusExpiresAt: (statusData.expires_at ?? null) as string | number | null,
                    })

                    if (userId === authStore.user?.id) {
                        authStore.syncUserStatusSnapshot({
                            status: status as PresenceStatus,
                            text: (statusData.text ?? null) as string | null,
                            emoji: (statusData.emoji ?? null) as string | null,
                            expiresAt: (statusData.expires_at ?? null) as string | number | null,
                        })
                    }
                }
            })
        }
    }


    function connect() {
        if (!authStore.token) {
            log.debug('No auth token, skipping WebSocket connection')
            return
        }

        if (ws.value?.readyState === WebSocket.OPEN) return;

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
        const host = window.location.host
        // Align with Mattermost mobile websocket endpoint semantics.
        const url = `${protocol}//${host}/api/v4/websocket`

        try {
            // Browsers can't set Authorization headers for WebSocket handshakes.
            // Pass bearer token via Sec-WebSocket-Protocol.
            const socket = new WebSocket(url, [authStore.token])
            ws.value = socket

            socket.onopen = () => {
                const openedAfterReconnect = reconnectAttempts.value > 0
                log.debug('WebSocket connected')
                connected.value = true
                reconnectAttempts.value = 0
                reconnectAttempt.value = 0
                clearReconnectTimer()
                updateConnectionStatus()
                stopCountdown()

                // Resubscribe to channels
                subscriptions.value.forEach(cid => {
                    send({
                        type: 'command',
                        event: 'subscribe_channel',
                        channel_id: cid,
                        data: {}
                    })
                })

                // Trigger resync for current channel if needed
                if (channelStore.currentChannelId) {
                    // Implement resync logic here or notify store
                    // For now, simpler to just refetch messages if connection was lost for a while
                    // or rely on 'after' cursor fetch.
                    messageStore.fetchMessages(channelStore.currentChannelId)
                }

                if (openedAfterReconnect) {
                    // Non-reliable WS clients must explicitly request a reconnect snapshot.
                    sendAction('reconnect', {})
                }
            }

            socket.onclose = (event) => {
                log.debug('WebSocket disconnected', event.code, event.reason)
                connected.value = false
                ws.value = null
                disconnectedAt.value = new Date()
                reconnectAttempt.value++
                updateConnectionStatus()
                startCountdown()

                if (isAuthExpiryCloseEvent(event)) {
                    clearReconnectTimer()
                    reconnectAttempts.value = 0
                    reconnectAttempt.value = 0
                    stopCountdown()
                    void authStore.logout('expired')
                    return
                }

                if (!authStore.token) {
                    clearReconnectTimer()
                    reconnectAttempts.value = 0
                    reconnectAttempt.value = 0
                    stopCountdown()
                    return
                }

                // Attempt to reconnect
                if (reconnectAttempts.value < maxReconnectAttempts) {
                    reconnectAttempts.value++
                    // Exponential backoff with jitter
                    const baseDelay = Math.min(1000 * Math.pow(1.5, reconnectAttempts.value), 30000)
                    const jitter = Math.random() * 1000
                    const delay = baseDelay + jitter

                    log.debug(`Reconnecting in ${Math.round(delay)}ms...`)
                    clearReconnectTimer()
                    reconnectTimer = setTimeout(() => {
                        if (!connected.value) connect()
                    }, delay)
                }
            }

            socket.onerror = (error) => {
                log.error('WebSocket connection failed:', error)
                toast.error('Real-time connection error', 'The connection to the server was refused. Please check your network.')
            }

            socket.onmessage = (event) => {
                try {
                    const rawEnvelope: WsEnvelope = JSON.parse(event.data)
                    const envelope = normalizeWsEnvelope(rawEnvelope)
                    handleMessage(envelope)
                } catch (e) {
                    log.error('Failed to parse WebSocket message:', e)
                }
            }
        } catch (e) {
            log.error('Failed to create WebSocket:', e)
        }
    }

    function handleMessage(envelope: WsEnvelope) {
        // log.debug('WS Received:', envelope.event, envelope.data)

        switch (envelope.event) {
            case 'hello':
                log.debug('WebSocket hello received', envelope.data)
                break

            case 'initial_load':
                applyInitialLoadSnapshot(envelope.data)
                break

            case 'posted':
            case 'message_created':
            case 'post_created': // Fallback
            case 'message_posted':
            // Mattermost standard
            case 'thread_reply_created': {
                const post = normalizeWsPost(envelope.data, envelope.channel_id)
                if (!post) {
                    break
                }
                // If it's a thread reply, logic might slightly differ (handled by store)
                messageStore.handleNewMessage(post)

                // Notifications handling (counters are handled by unread_counts_updated)
                if (post.channel_id !== channelStore.currentChannelId && post.user_id !== authStore.user?.id) {
                    const channel = channelStore.channels.find(c => c.id === post.channel_id)
                    const channelName = channel?.name || 'Unknown Channel'
                    const senderName = post.username || 'Someone'
                    const title = `#${channelName} - ${senderName}`

                    if (Notification.permission === 'granted') {
                        new Notification(title, { body: post.message, icon: '/favicon.ico' })
                    } else if (Notification.permission !== 'denied') {
                        Notification.requestPermission().then(p => {
                            if (p === 'granted') {
                                new Notification(title, { body: post.message, icon: '/favicon.ico' })
                            }
                        })
                    }
                }
                break
            }

            case 'message_updated':
            case 'post_edited': // Fallback
            case 'thread_reply_updated':
                if (envelope.data && typeof envelope.data === 'object' && 'id' in envelope.data) {
                    // Partial update or full post?
                    // Backend sends { id, reply_count_inc, last_reply_at } for thread updates
                    // Or full post for edits.
                    // Store needs to handle both.
                    messageStore.handleMessageUpdate(envelope.data)
                }
                break

            case 'message_deleted':
            case 'post_deleted': // Fallback
            case 'thread_reply_deleted':
                if (envelope.data && typeof envelope.data === 'object') {
                    const dataObj = envelope.data as Record<string, unknown>
                    const postId = dataObj.post_id ?? dataObj.id
                    if (postId) {
                        messageStore.handleMessageDelete(String(postId))
                    }
                }
                break

            case 'reaction_added':
                {
                    const reaction = normalizeWsReactionPayload(envelope.data)
                    if (reaction) {
                        messageStore.handleReactionAdded(reaction)
                    }
                }
                break

            case 'reaction_removed':
                {
                    const reaction = normalizeWsReactionPayload(envelope.data)
                    if (reaction) {
                        messageStore.handleReactionRemoved(reaction)
                    }
                }
                break

            case 'user_typing':
            case 'typing': // Compatibility with some mobile clients
                if (envelope.data && typeof envelope.data === 'object') {
                    const dataObj = envelope.data as Record<string, unknown>
                    const typingChannelId = envelope.channel_id || envelope.broadcast?.channel_id || dataObj.channel_id
                    if (!typingChannelId) {
                        break
                    }
                    presenceStore.addTypingUser(
                        String(dataObj.user_id || ''),
                        String(dataObj.display_name || dataObj.username || 'Someone'),
                        String(typingChannelId),
                        dataObj.thread_root_id as string | undefined
                    )
                }
                break

            case 'user_typing_stop':
            case 'stop_typing':
                if (envelope.data && typeof envelope.data === 'object') {
                    const dataObj = envelope.data as Record<string, unknown>
                    const typingChannelId = envelope.channel_id || envelope.broadcast?.channel_id || dataObj.channel_id
                    if (!typingChannelId) {
                        break
                    }
                    presenceStore.removeTypingUser(
                        String(dataObj.user_id || ''),
                        String(typingChannelId),
                        dataObj.thread_root_id as string | undefined
                    )
                }
                break

            case 'user_presence':
                if (envelope.data && typeof envelope.data === 'object') {
                    const dataObj = envelope.data as Record<string, unknown>
                    presenceStore.updatePresenceFromEvent(
                        String(dataObj.user_id || ''),
                        String(dataObj.status || 'online') as PresenceStatus
                    )
                }
                break

            case 'status_change':
                if (envelope.data && typeof envelope.data === 'object') {
                    const dataObj = envelope.data as Record<string, unknown>
                    const userId = String(dataObj.user_id || '')
                    const status = String(dataObj.status || 'online')
                    presenceStore.updatePresenceFromEvent(userId, status as PresenceStatus)
                    applyUserStatusSnapshot({
                        userId,
                        presence: status as PresenceStatus,
                        statusText: (dataObj.text ?? null) as string | null,
                        statusEmoji: (dataObj.emoji ?? null) as string | null,
                        statusExpiresAt: (dataObj.expires_at ?? null) as string | number | null,
                    })

                    if (userId === authStore.user?.id) {
                        authStore.syncUserStatusSnapshot({
                            status: status as PresenceStatus,
                            text: (dataObj.text ?? null) as string | null,
                            emoji: (dataObj.emoji ?? null) as string | null,
                            expiresAt: (dataObj.expires_at ?? null) as string | number | null,
                        })
                    }
                }
                break

            case 'channel_created': {
                if (envelope.data) {
                    const channel = normalizeWsChannelPayload(envelope.data)
                    if (channel) {
                        channelStore.addChannel(channel)
                    }
                }
                break
            }

            case 'unread_counts_updated': {
                if (envelope.data) {
                    unreadStore.handleUnreadUpdate(envelope.data as { channel_id: string; team_id: string; unread_count: number })
                }
                break
            }

            case 'post_unread': {
                if (envelope.data) {
                    const payload = normalizeIdsDeep(envelope.data) as ChannelUnreadAt
                    unreadStore.applyPostUnread(payload)
                }
                break
            }

            case 'error':
                log.error('WS Error:', envelope.data)
                break
        }

        // Notify listeners
        const eventListeners = listeners.value[envelope.event]
        if (eventListeners) {
            eventListeners.forEach(cb => cb(envelope.data))
        }
    }

    function disconnect() {
        clearReconnectTimer()
        stopCountdown()
        if (ws.value) {
            ws.value.close()
            ws.value = null
        }
        connected.value = false
        reconnectAttempts.value = 0
        reconnectAttempt.value = 0
        disconnectedAt.value = null
        subscriptions.value.clear()
        updateConnectionStatus()
    }

    function send(envelope: ClientEnvelope) {
        if (ws.value && connected.value) {
            ws.value.send(JSON.stringify(envelope))
        }
    }

    function sendAction(action: string, data: Record<string, unknown>) {
        if (ws.value && connected.value) {
            ws.value.send(JSON.stringify({
                action,
                seq: actionSeq++,
                data,
            }))
        }
    }

    function subscribe(channelId: string) {
        if (!subscriptions.value.has(channelId)) {
            subscriptions.value.add(channelId)
            send({
                type: 'command',
                event: 'subscribe_channel',
                channel_id: channelId,
                data: {}
            })
        }
    }

    function unsubscribe(channelId: string) {
        if (subscriptions.value.has(channelId)) {
            subscriptions.value.delete(channelId)
            send({
                type: 'command',
                event: 'unsubscribe_channel',
                channel_id: channelId,
                data: {}
            })
        }
    }

    function sendTyping(channelId: string, threadRootId?: string) {
        // Match Mattermost web/mobile typing command format.
        sendAction('user_typing', {
            channel_id: channelId,
            parent_id: threadRootId,
        })
    }

    function sendStopTyping(channelId: string, threadRootId?: string) {
        sendAction('user_typing_stop', {
            channel_id: channelId,
            parent_id: threadRootId,
        })
    }

    async function sendMessage(channelId: string, content: string, rootId?: string, fileIds: string[] = []) {
        const clientMsgId = crypto.randomUUID()
        const authStore = useAuthStore()
        const messageStore = useMessageStore()

        // Create temp message for optimistic UI
        const tempMsg: Message = {
            id: clientMsgId,
            channelId,
            userId: authStore.user?.id || '',
            username: authStore.user?.username || 'Me',
            avatarUrl: authStore.user?.avatar_url,
            content,
            timestamp: new Date().toISOString(),
            reactions: [],
            files: [],
            isPinned: false,
            isSaved: false,
            status: 'sending',
            clientMsgId,
            rootId,
            seq: 0  // Will be assigned by server
        }

        messageStore.addOptimisticMessage(tempMsg)

        try {
            const { data: post } = await postsApi.create({
                channel_id: channelId,
                message: content,
                root_post_id: rootId,
                file_ids: fileIds,
                client_msg_id: clientMsgId
            })

            // Convert server Post to frontend Message using store helper
            const finalMsg = postToMessage(post)
            messageStore.updateOptimisticMessage(clientMsgId, finalMsg)
        } catch (error) {
            log.error('Failed to send message via REST:', error)
            // Ideally we'd have a store method to mark as failed
            const msg = (messageStore.messagesByChannel[channelId] || []).find(m => m.id === clientMsgId)
            if (msg) msg.status = 'failed'
        }
    }

    function sendPresence(status: string) {
        send({
            type: 'command',
            event: 'presence',
            data: { status }
        })
    }

    function onEvent(event: string, callback: WsListener) {
        if (!listeners.value[event]) {
            listeners.value[event] = new Set()
        }
        listeners.value[event].add(callback)
    }

    function offEvent(event: string, callback: WsListener) {
        if (listeners.value[event]) {
            listeners.value[event].delete(callback)
        }
    }

    return {
        connected,
        connectionStatus,
        disconnectedAt,
        reconnectAttempt,
        nextRetryIn,
        connectionError,
        connect,
        disconnect,
        subscribe,
        unsubscribe,
        sendTyping,
        sendStopTyping,
        sendMessage,
        sendPresence,
        onEvent,
        offEvent,
    }
}
