import type { Unsubscriber } from 'svelte/store'
import { get, writable } from 'svelte/store'
import { authStore } from './auth'
import { chatStore } from './chat'

export type ConnectionStatus = 'connecting' | 'connected' | 'reconnecting' | 'disconnected' | 'failed'

let socket: WebSocket | null = null
let unsubscribeAuth: Unsubscriber | null = null
let currentToken: string | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let statusTimer: ReturnType<typeof setTimeout> | null = null
let failedTimer: ReturnType<typeof setTimeout> | null = null
let hasBeenConnected = false
const eventHandlers = new Map<string, Set<(data: any) => void>>()

export const connectionStatus = writable<ConnectionStatus>('connecting')

export function onWebSocketEvent(event: string, handler: (data: any) => void): () => void {
    if (!eventHandlers.has(event)) {
        eventHandlers.set(event, new Set())
    }
    eventHandlers.get(event)!.add(handler)
    return () => {
        eventHandlers.get(event)?.delete(handler)
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

function websocketUrl(): string {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    return `${protocol}//${window.location.host}/api/v4/websocket`
}

function clearTimers(): void {
    if (reconnectTimer) {
        clearTimeout(reconnectTimer)
        reconnectTimer = null
    }
    if (statusTimer) {
        clearTimeout(statusTimer)
        statusTimer = null
    }
    if (failedTimer) {
        clearTimeout(failedTimer)
        failedTimer = null
    }
}

function closeSocket(): void {
    const current = socket
    socket = null

    if (current && current.readyState !== WebSocket.CLOSED) {
        current.close()
    }
}

function scheduleReconnect(): void {
    clearTimers()

    // Immediate reconnect attempt after 3 seconds
    reconnectTimer = setTimeout(() => {
        if (currentToken) {
            connect(currentToken)
        }
    }, 3000)

    // After 5s without reconnection → disconnected
    statusTimer = setTimeout(() => {
        connectionStatus.set('disconnected')
    }, 5000)

    // After 30s without reconnection → failed
    failedTimer = setTimeout(() => {
        connectionStatus.set('failed')
    }, 30000)
}

function handleMessage(data: string): void {
    try {
        const event = JSON.parse(data) as { event: string; data: string; broadcast?: { channel_id: string; user_id: string } }
        const payload = event.data ? JSON.parse(event.data) : {}

        switch (event.event) {
            case 'post_received': {
                const post = payload as { id: string; channel_id: string; user_id: string; message: string; created_at: string; files?: unknown[]; client_msg_id?: string }
                if (post.channel_id) {
                    chatStore.addMessage(post.channel_id, {
                        id: post.id,
                        channel_id: post.channel_id,
                        user_id: post.user_id,
                        message: post.message,
                        created_at: post.created_at,
                        files: (payload.files as unknown[] ?? []).map((f: unknown) => {
                            const file = f as Record<string, unknown>
                            return {
                                id: String(file.id ?? ''),
                                name: file.name ? String(file.name) : undefined,
                                url: file.url ? String(file.url) : undefined,
                                size: typeof file.size === 'number' ? file.size : undefined,
                                mime_type: file.mime_type ? String(file.mime_type) : undefined,
                                mimeType: file.mimeType ? String(file.mimeType) : undefined,
                                width: typeof file.width === 'number' ? file.width : undefined,
                                height: typeof file.height === 'number' ? file.height : undefined,
                            }
                        }),
                        client_msg_id: post.client_msg_id,
                    })
                }
                break
            }
            case 'post_updated': {
                const post = payload as { id: string; channel_id: string; message?: string; created_at?: string }
                if (post.channel_id) {
                    chatStore.updateMessage(post.channel_id, {
                        id: post.id,
                        message: post.message,
                        created_at: post.created_at,
                    })
                }
                break
            }
            case 'post_deleted': {
                const post = payload as { id: string; channel_id: string }
                if (post.channel_id) {
                    chatStore.deleteMessage(post.channel_id, post.id)
                }
                break
            }
            case 'status_change': {
                const statusPayload = payload as { user_id: string; status: string }
                if (statusPayload.user_id) {
                    chatStore.updateMemberPresence(statusPayload.user_id, statusPayload.status)
                }
                break
            }
            case 'unread_counts_updated': {
                const unreadData = payload as { channel_id: string; team_id: string; unread_count: number }
                if (unreadData.channel_id) {
                    chatStore.update((state) => ({
                        ...state,
                        unreadCounts: {
                            ...state.unreadCounts,
                            [unreadData.channel_id]: unreadData.unread_count,
                        },
                    }))
                }
                break
            }
            default: {
                // Dispatch to registered custom handlers
                const handlers = eventHandlers.get(event.event)
                if (handlers) {
                    for (const handler of handlers) {
                        handler(payload)
                    }
                }
                break
            }
        }
    } catch {
        // Ignore malformed messages
    }
}

export function connect(token: string): void {
    if (socket && (socket.readyState === WebSocket.CONNECTING || socket.readyState === WebSocket.OPEN)) {
        return
    }

    currentToken = token
    connectionStatus.set('connecting')
    clearTimers()

    const nextSocket = new WebSocket(websocketUrl(), [token])
    socket = nextSocket

    nextSocket.onopen = () => {
        if (socket === nextSocket) {
            const wasReconnecting = get(connectionStatus) === 'reconnecting'
            connectionStatus.set('connected')
            clearTimers()

            if (wasReconnecting && hasBeenConnected) {
                const currentChannelId = get(chatStore).currentChannelId
                if (currentChannelId) {
                    void chatStore.fetchMessages(currentChannelId)
                }
                void chatStore.fetchUnreadCounts()
            }
            hasBeenConnected = true
        }
    }

    nextSocket.onmessage = (event) => {
        handleMessage(event.data)
    }

    nextSocket.onclose = (event) => {
        if (socket === nextSocket) {
            socket = null
        }

        if (isAuthExpiryCloseEvent(event)) {
            void authStore.logout('expired')
            return
        }

        connectionStatus.set('reconnecting')
        scheduleReconnect()
    }

    nextSocket.onerror = () => {
        // Error is usually followed by close, which handles reconnection
    }
}

export function retryConnection(): void {
    clearTimers()
    connectionStatus.set('reconnecting')
    if (currentToken) {
        connect(currentToken)
    }
}

export function registerWebSocketHandlers(): () => void {
    if (unsubscribeAuth) {
        return unsubscribeAuth
    }

    unsubscribeAuth = authStore.subscribe((state) => {
        if (state.token) {
            connect(state.token)
            return
        }

        closeSocket()
        clearTimers()
        connectionStatus.set('connecting')
    })

    return () => {
        unsubscribeAuth?.()
        unsubscribeAuth = null
        closeSocket()
        clearTimers()
        connectionStatus.set('connecting')
    }
}

// Expose test helpers for E2E tests
;(window as any).testHelpers = {
    simulateWebSocketClose: () => {
        socket?.close()
    },
    simulateWebSocketOpen: () => {
        if (currentToken) {
            connect(currentToken)
        }
    },
    getCurrentChannelId: () => get(chatStore).currentChannelId,
    sendMessageAsOtherUser: (channelId: string, message: string) => {
        handleMessage(
            JSON.stringify({
                event: 'post_received',
                data: JSON.stringify({
                    id: `test-${Date.now()}`,
                    channel_id: channelId,
                    user_id: 'other-user',
                    message,
                    created_at: new Date().toISOString(),
                    files: [],
                }),
                broadcast: { channel_id: channelId, user_id: 'other-user' },
            }),
        )
    },
    simulateUnreadCounts: (counts: { channel_id: string; unread_count: number }) => {
        handleMessage(
            JSON.stringify({
                event: 'unread_counts_updated',
                data: JSON.stringify(counts),
            }),
        )
    },
}
