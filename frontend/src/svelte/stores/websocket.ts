import type { Unsubscriber } from 'svelte/store'
import { authStore } from './auth'

let socket: WebSocket | null = null
let unsubscribeAuth: Unsubscriber | null = null

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

function closeSocket(): void {
    const current = socket
    socket = null

    if (current && current.readyState !== WebSocket.CLOSED) {
        current.close()
    }
}

function connect(token: string): void {
    if (socket && (socket.readyState === WebSocket.CONNECTING || socket.readyState === WebSocket.OPEN)) {
        return
    }

    const nextSocket = new WebSocket(websocketUrl(), [token])
    socket = nextSocket

    nextSocket.onclose = (event) => {
        if (socket === nextSocket) {
            socket = null
        }

        if (isAuthExpiryCloseEvent(event)) {
            void authStore.logout('expired')
        }
    }
}

export function initAuthExpiryWebSocket(): () => void {
    if (unsubscribeAuth) {
        return unsubscribeAuth
    }

    unsubscribeAuth = authStore.subscribe((state) => {
        if (state.token) {
            connect(state.token)
            return
        }

        closeSocket()
    })

    return () => {
        unsubscribeAuth?.()
        unsubscribeAuth = null
        closeSocket()
    }
}
