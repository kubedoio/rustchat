import { ref } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useMessageStore } from '../stores/messages'
import { usePresenceStore } from '../stores/presence'
import { useUnreadStore } from '../stores/unreads'
import { useChannelStore } from '../stores/channels'
import type { Post } from '../api/posts'

// Server -> Client
export interface WsEnvelope {
    type: 'event' | 'response' | 'error' | 'ack'
    event: string
    seq?: number
    channel_id?: string
    data: any
}

// Client -> Server
export interface ClientEnvelope {
    type: 'command'
    event: string
    data: any
    channel_id?: string
    client_msg_id?: string
    seq?: number
}

// Singleton state
const ws = ref<WebSocket | null>(null)
const connected = ref(false)
const reconnectAttempts = ref(0)
const maxReconnectAttempts = 10
const subscriptions = ref<Set<string>>(new Set())
const listeners = ref<Record<string, Set<(data: any) => void>>>({})

export function useWebSocket() {
    const authStore = useAuthStore()
    const messageStore = useMessageStore()
    const presenceStore = usePresenceStore()
    const unreadStore = useUnreadStore()
    const channelStore = useChannelStore()


    function connect() {
        if (!authStore.token) {
            console.log('No auth token, skipping WebSocket connection')
            return
        }

        if (ws.value?.readyState === WebSocket.OPEN) return;

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
        const host = window.location.host
        const url = `${protocol}//${host}/api/v1/ws?token=${authStore.token}`

        try {
            const socket = new WebSocket(url)
            ws.value = socket

            socket.onopen = () => {
                console.log('WebSocket connected')
                connected.value = true
                reconnectAttempts.value = 0

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
            }

            socket.onclose = (event) => {
                console.log('WebSocket disconnected', event.code, event.reason)
                connected.value = false
                ws.value = null

                // Attempt to reconnect
                if (reconnectAttempts.value < maxReconnectAttempts) {
                    reconnectAttempts.value++
                    // Exponential backoff with jitter
                    const baseDelay = Math.min(1000 * Math.pow(1.5, reconnectAttempts.value), 30000)
                    const jitter = Math.random() * 1000
                    const delay = baseDelay + jitter

                    console.log(`Reconnecting in ${Math.round(delay)}ms...`)
                    setTimeout(() => {
                        if (!connected.value) connect()
                    }, delay)
                }
            }

            socket.onerror = (error) => {
                console.error('WebSocket error:', error)
            }

            socket.onmessage = (event) => {
                try {
                    const envelope: WsEnvelope = JSON.parse(event.data)
                    handleMessage(envelope)
                } catch (e) {
                    console.error('Failed to parse WebSocket message:', e)
                }
            }
        } catch (e) {
            console.error('Failed to create WebSocket:', e)
        }
    }

    function handleMessage(envelope: WsEnvelope) {
        // console.log('WS Received:', envelope.event, envelope.data)

        switch (envelope.event) {
            case 'hello':
                console.log('WebSocket hello received', envelope.data)
                break

            case 'message_created':
            case 'post_created': // Fallback
            case 'thread_reply_created': {
                const post = envelope.data as Post
                // If it's a thread reply, logic might slightly differ (handled by store)
                messageStore.handleNewMessage(post)

                // Notifications handling (counters are handled by unread_counts_updated)
                if (post.channel_id !== channelStore.currentChannelId && post.user_id !== authStore.user?.id) {
                    const mentionsUser = post.message?.includes(`@${authStore.user?.username}`) || false

                    if (mentionsUser) {
                        const channel = channelStore.channels.find(c => c.id === post.channel_id)
                        const title = channel ? `#${channel.name}` : 'New Mention'

                        if (Notification.permission === 'granted') {
                            new Notification(title, { body: post.message })
                        } else if (Notification.permission !== 'denied') {
                            Notification.requestPermission().then(p => {
                                if (p === 'granted') {
                                    new Notification(title, { body: post.message })
                                }
                            })
                        }
                    }
                }
                break
            }

            case 'message_updated':
            case 'post_edited': // Fallback
            case 'thread_reply_updated':
                if (envelope.data.id) {
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
                if (envelope.data && (envelope.data.post_id || envelope.data.id)) {
                    messageStore.handleMessageDelete(envelope.data.post_id || envelope.data.id)
                }
                break

            case 'reaction_added':
                messageStore.handleReactionAdded(envelope.data)
                break

            case 'reaction_removed':
                messageStore.handleReactionRemoved(envelope.data)
                break

            case 'user_typing':
                if (envelope.data) {
                    presenceStore.addTypingUser(
                        envelope.data.user_id,
                        envelope.data.display_name || envelope.data.username || 'Someone',
                        envelope.channel_id || envelope.data.channel_id,
                        envelope.data.thread_root_id
                    )
                }
                break

            case 'user_typing_stop':
                if (envelope.data) {
                    presenceStore.removeTypingUser(
                        envelope.data.user_id,
                        envelope.channel_id || envelope.data.channel_id,
                        envelope.data.thread_root_id
                    )
                }
                break

            case 'user_presence':
                if (envelope.data) {
                    presenceStore.updatePresenceFromEvent(
                        envelope.data.user_id,
                        envelope.data.status || 'online'
                    )
                }
                break

            case 'channel_created': {
                if (envelope.data) {
                    channelStore.addChannel(envelope.data)
                }
                break
            }

            case 'unread_counts_updated': {
                if (envelope.data) {
                    unreadStore.handleUnreadUpdate(envelope.data)
                }
                break
            }

            case 'error':
                console.error('WS Error:', envelope.data)
                break
        }

        // Notify listeners
        const eventListeners = listeners.value[envelope.event]
        if (eventListeners) {
            eventListeners.forEach(cb => cb(envelope.data))
        }
    }

    function disconnect() {
        if (ws.value) {
            ws.value.close()
            ws.value = null
        }
        connected.value = false
        subscriptions.value.clear()
    }

    function send(envelope: ClientEnvelope) {
        if (ws.value && connected.value) {
            ws.value.send(JSON.stringify(envelope))
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
        send({
            type: 'command',
            event: 'typing_start',
            channel_id: channelId,
            data: { thread_root_id: threadRootId }
        })
    }

    function sendStopTyping(channelId: string, threadRootId?: string) {
        send({
            type: 'command',
            event: 'typing_stop',
            channel_id: channelId,
            data: { thread_root_id: threadRootId }
        })
    }

    async function sendMessage(channelId: string, content: string, rootId?: string) {
        const clientMsgId = crypto.randomUUID()
        const authStore = useAuthStore()

        // Create temp message
        const tempMsg: any = {
            id: clientMsgId, // uses same ID for now, server will respond with real ID which replaces it, OR we keep Client ID until confirmed
            // Actually our store uses `id` as primary key.
            // If we use UUIDv4 for local, server generates UUIDv4.
            // When server update comes, we swap 'id' to server ID.
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
            status: 'sending', // Display as grayed out or spinner
            clientMsgId,
            rootId
        }

        messageStore.addOptimisticMessage(tempMsg)

        send({
            type: 'command',
            event: 'send_message',
            channel_id: channelId,
            client_msg_id: clientMsgId,
            data: {
                message: content,
                root_post_id: rootId
            }
        })
    }

    function sendPresence(status: string) {
        send({
            type: 'command',
            event: 'presence',
            data: { status }
        })
    }

    function onEvent(event: string, callback: (data: any) => void) {
        if (!listeners.value[event]) {
            listeners.value[event] = new Set()
        }
        listeners.value[event].add(callback)
    }

    function offEvent(event: string, callback: (data: any) => void) {
        if (listeners.value[event]) {
            listeners.value[event].delete(callback)
        }
    }

    return {
        connected,
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
