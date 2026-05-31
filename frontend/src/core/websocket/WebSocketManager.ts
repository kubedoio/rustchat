// WebSocket Manager — UNFINISHED REFACTORING, NOT WIRED INTO THE APP
//
// The active WebSocket implementation is in frontend/src/composables/useWebSocket.ts.
// This file is part of an in-progress refactoring that is not yet connected.
// Do not modify this file expecting runtime changes until it is integrated.

import { log } from '@/utils/log'
import { ref, computed, markRaw } from 'vue'

export type ConnectionState = 'connecting' | 'open' | 'closed' | 'error'

export interface WebSocketEvent {
  event: string
  data: unknown
  broadcast: {
    channel_id: string
    user_id: string
  }
  seq?: number
}

export type EventHandler = (event: WebSocketEvent) => void

class WebSocketManager {
  private ws: WebSocket | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private handlers = new Map<string, Set<EventHandler>>()
  private pendingMessages: string[] = []

  // Public state (reactive)
  state = ref<ConnectionState>('closed')
  connectionId = ref<string>('')
  lastSeq = ref<number>(0)

  readonly isConnected = computed(() => this.state.value === 'open')

  constructor() {
    // Bind methods
    this.connect = this.connect.bind(this)
    this.disconnect = this.disconnect.bind(this)
    this.send = this.send.bind(this)
  }

  // Connect to WebSocket
  connect(url: string, token: string) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return
    }

    this.state.value = 'connecting'

    // On reconnect, append connection_id and last_seq for server resumption.
    let connectUrl = url
    if (this.connectionId.value) {
      const sep = url.includes('?') ? '&' : '?'
      connectUrl += `${sep}connection_id=${encodeURIComponent(this.connectionId.value)}`
      if (this.lastSeq.value > 0) {
        connectUrl += `&sequence_number=${this.lastSeq.value}`
      }
    }

    // Use websocket subprotocol for auth token transport.
    // Query-token transport is rejected by the backend.
    this.ws = new WebSocket(connectUrl, [token])

    this.ws.onopen = () => {
      this.state.value = 'open'
      this.flushPendingMessages()
      this.startHeartbeat()
    }

    this.ws.onmessage = event => {
      this.handleMessage(event.data)
    }

    this.ws.onclose = () => {
      this.state.value = 'closed'
      this.scheduleReconnect(url, token)
    }

    this.ws.onerror = () => {
      this.state.value = 'error'
    }
  }

  disconnect() {
    this.stopHeartbeat()
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    this.ws?.close()
    this.ws = null
    this.resetConnectionState()
  }

  // Register a handler for a specific event type
  on(event: string, handler: EventHandler): () => void {
    if (!this.handlers.has(event)) {
      this.handlers.set(event, new Set())
    }
    this.handlers.get(event)!.add(handler)

    return () => {
      this.handlers.get(event)?.delete(handler)
    }
  }

  // Send a message
  send(message: object): boolean {
    const json = JSON.stringify(message)

    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(json)
      return true
    } else {
      this.pendingMessages.push(json)
      return false
    }
  }

  private handleMessage(data: string) {
    try {
      const event: WebSocketEvent = JSON.parse(data)

      // Parse connection_id from hello event data
      if (event.event === 'hello' && event.data) {
        const helloData = event.data as Record<string, unknown>
        if (typeof helloData.connection_id === 'string') {
          this.connectionId.value = helloData.connection_id
        }
      }

      // Track last sequence number for resumption
      if (typeof event.seq === 'number' && event.seq > this.lastSeq.value) {
        this.lastSeq.value = event.seq
      }

      // Dispatch to handlers
      const handlers = this.handlers.get(event.event)
      if (handlers) {
        handlers.forEach(handler => {
          try {
            handler(event)
          } catch (err) {
            log.error(`Handler error for ${event.event}:`, err)
          }
        })
      }

      // Also dispatch to wildcard handlers
      const wildcards = this.handlers.get('*')
      if (wildcards) {
        wildcards.forEach(handler => {
          try {
            handler(event)
          } catch (err) {
            log.error(`Wildcard handler error:`, err)
          }
        })
      }
    } catch (err) {
      log.error('Failed to parse WebSocket message:', err)
    }
  }

  private flushPendingMessages() {
    while (this.pendingMessages.length > 0 && this.ws?.readyState === WebSocket.OPEN) {
      const message = this.pendingMessages.shift()
      if (message) {
        this.ws.send(message)
      }
    }
  }

  private scheduleReconnect(url: string, token: string) {
    this.reconnectTimer = setTimeout(() => {
      this.connect(url, token)
    }, 5000)
  }

  private resetConnectionState() {
    this.connectionId.value = ''
    this.lastSeq.value = 0
  }

  private heartbeatTimer: ReturnType<typeof setInterval> | null = null

  private startHeartbeat() {
    this.stopHeartbeat()
    this.heartbeatTimer = setInterval(() => {
      this.send({ action: 'ping' })
    }, 30000)
  }

  private stopHeartbeat() {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer)
      this.heartbeatTimer = null
    }
  }
}

// Singleton instance
export const wsManager = markRaw(new WebSocketManager())

// Vue composable for components that need WebSocket access
export function useWebSocket() {
  return {
    state: wsManager.state,
    isConnected: wsManager.isConnected,
    connectionId: wsManager.connectionId,
    lastSeq: wsManager.lastSeq,
    connect: wsManager.connect,
    disconnect: wsManager.disconnect,
    send: wsManager.send,
    on: wsManager.on.bind(wsManager),
  }
}
