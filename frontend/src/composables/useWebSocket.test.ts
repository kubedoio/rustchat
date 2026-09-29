// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const FIXED_CLIENT_MSG_ID = 'fixed-client-msg-id'
const JITTER_FACTOR = 0.5
const JITTER_MS = JITTER_FACTOR * 1000

// Module-level mocks shared by every fresh import of useWebSocket (vi.resetModules
// re-runs the module, but the hoisted mock objects persist within this file).
const mocks = vi.hoisted(() => {
  const authStore = {
    token: 'test-token',
    user: { id: 'user-1', username: 'alice', avatar_url: '/alice.png' },
    logout: vi.fn(),
    syncUserStatusSnapshot: vi.fn(),
  }
  const messageStore = {
    fetchMessages: vi.fn(),
    handleNewMessage: vi.fn(),
    handleMessageUpdate: vi.fn(),
    handleMessageDelete: vi.fn(),
    handleReactionAdded: vi.fn(),
    handleReactionRemoved: vi.fn(),
    handleAgentStreamChunk: vi.fn(),
    handleAgentStreamComplete: vi.fn(),
    handleAgentStreamError: vi.fn(),
    handleAgentError: vi.fn(),
    addOptimisticMessage: vi.fn(),
    updateOptimisticMessage: vi.fn(),
    messagesByChannel: {} as Record<string, Array<Record<string, unknown>>>,
  }
  const presenceStore = {
    addTypingUser: vi.fn(),
    removeTypingUser: vi.fn(),
    updatePresenceFromEvent: vi.fn(),
  }
  const unreadStore = {
    handleUnreadUpdate: vi.fn(),
    applyPostUnread: vi.fn(),
    channelUnreads: {} as Record<string, number>,
    channelMentions: {} as Record<string, number>,
  }
  const channelStore = {
    addChannel: vi.fn(),
    currentChannelId: null as string | null,
    channels: [] as Array<{ id: string; name: string }>,
  }
  const toast = { success: vi.fn(), error: vi.fn(), info: vi.fn(), register: vi.fn() }
  return {
    authStore,
    messageStore,
    presenceStore,
    unreadStore,
    channelStore,
    toast,
    postsApiCreate: vi.fn(),
    postToMessage: vi.fn(),
    randomUUID: vi.fn(),
  }
})

vi.mock('@/utils/log', () => ({
  log: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}))
vi.mock('../features/auth/stores/authStore', () => ({
  useAuthStore: () => mocks.authStore,
}))
vi.mock('@/features/messages/stores/messageStore', () => ({
  useMessageStore: () => mocks.messageStore,
  postToMessage: (post: unknown) => mocks.postToMessage(post),
}))
vi.mock('../features/presence', () => ({
  usePresenceStore: () => mocks.presenceStore,
}))
vi.mock('@/features/unreads/stores/unreadStore', () => ({
  useUnreadStore: () => mocks.unreadStore,
}))
vi.mock('@/features/channels/stores/channelStore', () => ({
  useChannelStore: () => mocks.channelStore,
}))
vi.mock('./useToast', () => ({
  useToast: () => mocks.toast,
}))
vi.mock('../api/posts', () => ({
  postsApi: { create: mocks.postsApiCreate },
}))
vi.mock('./useUserSummary', () => ({
  applyUserStatusSnapshot: vi.fn(),
}))

class FakeWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3
  static instances: FakeWebSocket[] = []

  url: string
  protocols: string[] | undefined
  sent: string[] = []
  onopen: (() => void) | null = null
  onclose: ((ev: { code: number; reason: string }) => void) | null = null
  onerror: ((ev: unknown) => void) | null = null
  onmessage: ((ev: { data: string }) => void) | null = null
  readyState = FakeWebSocket.CONNECTING

  constructor(url: string, protocols?: string[]) {
    this.url = url
    this.protocols = protocols
    FakeWebSocket.instances.push(this)
  }

  send(data: string) {
    this.sent.push(data)
  }

  close() {
    this.readyState = FakeWebSocket.CLOSED
  }

  // Test helpers simulating what the browser would deliver.
  open() {
    this.readyState = FakeWebSocket.OPEN
    this.onopen?.()
  }

  emitClose(code = 1000, reason = '') {
    this.readyState = FakeWebSocket.CLOSED
    this.onclose?.({ code, reason })
  }

  emitServerMessage(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) })
  }
}

class FakeNotification {
  static instances: Array<{ title: string; options: unknown }> = []
  static permission: 'granted' | 'denied' | 'default' = 'granted'
  static requestPermission = vi.fn(async () => 'granted' as 'granted' | 'denied' | 'default')

  title: string
  options: unknown

  constructor(title: string, options?: unknown) {
    this.title = title
    this.options = options
    FakeNotification.instances.push({ title, options })
  }
}

function wsBaseUrl() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${protocol}//${window.location.host}/api/v4/websocket`
}

// Mirrors the backoff formula in useWebSocket.ts: `attempt` is the value of
// reconnectAttempts AFTER the increment performed when the timer is scheduled,
// and the cap is hardcoded to 30000 in the source (not RECONNECT_DELAY_MAX_MS).
function reconnectDelay(attempt: number) {
  return Math.min(1000 * Math.pow(1.5, attempt), 30000) + JITTER_MS
}

async function useFreshWebSocket() {
  const mod = await import('./useWebSocket')
  return mod.useWebSocket()
}

function lastSocket() {
  return FakeWebSocket.instances[FakeWebSocket.instances.length - 1]!
}

function openSocket(sock: FakeWebSocket) {
  sock.open()
}

function sentFrames(sock: FakeWebSocket) {
  return sock.sent.map(frame => JSON.parse(frame))
}

describe('useWebSocket', () => {
  beforeEach(() => {
    // The composable keeps singleton state at module level; every test needs a
    // fresh module so refs, wsConnectionId and wsLastSeq do not leak between tests.
    vi.resetModules()
    vi.useFakeTimers()
    vi.clearAllMocks()
    vi.restoreAllMocks()
    // Deterministic jitter: Math.random() -> 0.5, so jitter is always 500ms.
    vi.spyOn(Math, 'random').mockReturnValue(JITTER_FACTOR)

    FakeWebSocket.instances = []
    FakeNotification.instances = []
    FakeNotification.permission = 'granted'

    vi.stubGlobal('WebSocket', FakeWebSocket)
    vi.stubGlobal('Notification', FakeNotification)
    vi.stubGlobal('crypto', { randomUUID: mocks.randomUUID })

    mocks.authStore.token = 'test-token'
    mocks.authStore.user = { id: 'user-1', username: 'alice', avatar_url: '/alice.png' }
    mocks.authStore.logout.mockResolvedValue(undefined)
    mocks.channelStore.currentChannelId = null
    mocks.channelStore.channels = []
    mocks.messageStore.messagesByChannel = {}
    mocks.messageStore.addOptimisticMessage.mockImplementation((msg: Record<string, unknown>) => {
      const channelId = msg.channelId as string
      const list = (mocks.messageStore.messagesByChannel[channelId] ??= [])
      list.push(msg)
    })
    mocks.postToMessage.mockImplementation((post: Record<string, unknown>) => ({
      id: post.id,
      channelId: post.channel_id,
      content: post.message,
      status: 'delivered',
    }))
    mocks.randomUUID.mockReturnValue(FIXED_CLIENT_MSG_ID)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('does not create a WebSocket when there is no auth token', async () => {
    mocks.authStore.token = ''
    const ws = await useFreshWebSocket()

    ws.connect()

    expect(FakeWebSocket.instances).toHaveLength(0)
  })

  it('connects with the auth token as subprotocol and reports connected state', async () => {
    const ws = await useFreshWebSocket()

    ws.connect()

    expect(FakeWebSocket.instances).toHaveLength(1)
    const sock = FakeWebSocket.instances[0]!
    expect(sock.url).toBe(wsBaseUrl())
    expect(sock.protocols).toEqual(['test-token'])
    expect(ws.connected.value).toBe(false)

    openSocket(sock)

    expect(ws.connected.value).toBe(true)
    expect(ws.connectionStatus.value).toBe('connected')
    expect(ws.disconnectedAt.value).toBeNull()
    expect(ws.connectionError.value).toBeNull()

    // A second connect while the socket is OPEN must not open another socket.
    ws.connect()
    expect(FakeWebSocket.instances).toHaveLength(1)
  })

  it('surfaces connection errors as toasts', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!

    sock.onerror?.(new Error('refused'))

    expect(mocks.toast.error).toHaveBeenCalledWith(
      'Real-time connection error',
      'The connection to the server was refused. Please check your network.'
    )
  })

  it('reconnects after a normal close with exponential backoff and resumes the session', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    // Capture the connection id from hello and track the highest envelope seq.
    sock.emitServerMessage({ type: 'event', event: 'hello', data: { connection_id: 'conn-abc' } })
    sock.emitServerMessage({ type: 'event', event: 'ack', seq: 7, data: null })
    sock.emitServerMessage({ type: 'event', event: 'ack', seq: 3, data: null })

    sock.emitClose(1000, '')
    expect(ws.connected.value).toBe(false)
    expect(ws.reconnectAttempt.value).toBe(1)
    expect(ws.connectionStatus.value).toBe('reconnecting')
    expect(ws.disconnectedAt.value).not.toBeNull()

    // Attempt 1 backoff: min(1000 * 1.5^1, 30000) + 500ms jitter = 2000ms.
    vi.advanceTimersByTime(reconnectDelay(1) - 1)
    expect(FakeWebSocket.instances).toHaveLength(1)
    vi.advanceTimersByTime(1)
    expect(FakeWebSocket.instances).toHaveLength(2)

    const resumed = FakeWebSocket.instances[1]!
    expect(resumed.url).toBe(`${wsBaseUrl()}?connection_id=conn-abc&sequence_number=7`)
    expect(resumed.protocols).toEqual(['test-token'])
  })

  it('increases the backoff for the second attempt', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    openSocket(FakeWebSocket.instances[0]!)

    FakeWebSocket.instances[0]!.emitClose(1000, '')
    vi.advanceTimersByTime(reconnectDelay(1))
    expect(FakeWebSocket.instances).toHaveLength(2)

    FakeWebSocket.instances[1]!.emitClose(1000, '')

    // Attempt 2 backoff: min(1000 * 1.5^2, 30000) + 500ms jitter = 2750ms.
    vi.advanceTimersByTime(reconnectDelay(2) - 1)
    expect(FakeWebSocket.instances).toHaveLength(2)
    vi.advanceTimersByTime(1)
    expect(FakeWebSocket.instances).toHaveLength(3)
  })

  it('caps the backoff at 30s and stops reconnecting after MAX_RECONNECT_ATTEMPTS', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    openSocket(FakeWebSocket.instances[0]!)

    // Attempts 1..8 each create one follow-up socket (2..9).
    for (let attempt = 1; attempt <= 8; attempt++) {
      lastSocket().emitClose(1000, '')
      vi.advanceTimersByTime(reconnectDelay(attempt))
      expect(FakeWebSocket.instances).toHaveLength(attempt + 1)
    }

    // Attempt 9: 1000 * 1.5^9 exceeds the hardcoded 30s cap, so the delay is
    // 30000 + 500ms jitter = 30500ms (not the uncapped 38443ms).
    lastSocket().emitClose(1000, '')
    vi.advanceTimersByTime(reconnectDelay(9) - 1)
    expect(FakeWebSocket.instances).toHaveLength(9)
    vi.advanceTimersByTime(1)
    expect(FakeWebSocket.instances).toHaveLength(10)
    expect(ws.reconnectAttempt.value).toBe(9)
    expect(ws.connectionStatus.value).toBe('reconnecting')

    // Attempt 10 still passes the `reconnectAttempts < MAX_RECONNECT_ATTEMPTS`
    // guard, so one more retry is scheduled. Note that the status already reads
    // 'failed' here (reconnectAttempt >= MAX) even though a retry is pending.
    lastSocket().emitClose(1000, '')
    expect(ws.reconnectAttempt.value).toBe(10)
    expect(ws.connectionStatus.value).toBe('failed')
    vi.advanceTimersByTime(reconnectDelay(10))
    expect(FakeWebSocket.instances).toHaveLength(11)

    // Attempt 11: reconnectAttempts (10) is no longer < MAX_RECONNECT_ATTEMPTS
    // (10), so no timer is scheduled and no further sockets are created.
    lastSocket().emitClose(1000, '')
    vi.advanceTimersByTime(120_000)
    expect(FakeWebSocket.instances).toHaveLength(11)
    expect(ws.reconnectAttempt.value).toBe(11)
    expect(ws.connectionStatus.value).toBe('failed')
  })

  it('runs a per-second countdown while waiting for the next retry', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    sock.emitClose(1000, '')

    // startCountdown uses min(1000 * 1.5^1, RECONNECT_DELAY_MAX_MS) / 1000 = 1.5.
    expect(ws.nextRetryIn.value).toBe(1.5)
    vi.advanceTimersByTime(1000)
    expect(ws.nextRetryIn.value).toBe(0.5)

    // The interval decrements whole seconds, so a fractional start overshoots
    // slightly below zero before the countdown stops (current behavior).
    vi.advanceTimersByTime(1000)
    expect(ws.nextRetryIn.value).toBe(-0.5)
    vi.advanceTimersByTime(10_000)
    expect(ws.nextRetryIn.value).toBe(-0.5)
  })

  it('logs the user out and never reconnects when the close event signals token expiry', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    sock.emitClose(1008, 'Invalid session token')

    expect(mocks.authStore.logout).toHaveBeenCalledTimes(1)
    expect(mocks.authStore.logout).toHaveBeenCalledWith('expired')
    vi.advanceTimersByTime(120_000)
    expect(FakeWebSocket.instances).toHaveLength(1)
    expect(ws.reconnectAttempt.value).toBe(0)
    // NOTE: connectionStatus stays 'reconnecting' here because the auth-expiry
    // branch returns before any status update (current behavior).
    expect(ws.connectionStatus.value).toBe('reconnecting')
  })

  it('treats an "authentication token expired" close reason as auth expiry without code 1008', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    sock.emitClose(1006, 'authentication token expired')

    expect(mocks.authStore.logout).toHaveBeenCalledWith('expired')
    vi.advanceTimersByTime(120_000)
    expect(FakeWebSocket.instances).toHaveLength(1)
  })

  it('does not schedule a reconnect when the auth token is already gone at close time', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    mocks.authStore.token = ''
    sock.emitClose(1000, '')

    expect(mocks.authStore.logout).not.toHaveBeenCalled()
    vi.advanceTimersByTime(120_000)
    expect(FakeWebSocket.instances).toHaveLength(1)
    expect(ws.reconnectAttempt.value).toBe(0)
  })

  it('resends channel subscriptions on open and dedupes repeated subscribe calls', async () => {
    const ws = await useFreshWebSocket()
    ws.subscribe('channel-1')
    ws.subscribe('channel-2')
    ws.subscribe('channel-1') // duplicate, must not produce a second command

    ws.connect()
    const sock = FakeWebSocket.instances[0]!

    // Nothing is sent before the socket is open.
    expect(sock.sent).toHaveLength(0)

    openSocket(sock)

    const frames = sentFrames(sock)
    expect(frames).toHaveLength(2)
    expect(frames).toContainEqual({
      type: 'command',
      event: 'subscribe_channel',
      channel_id: 'channel-1',
      data: {},
    })
    expect(frames).toContainEqual({
      type: 'command',
      event: 'subscribe_channel',
      channel_id: 'channel-2',
      data: {},
    })

    ws.unsubscribe('channel-2')
    expect(sentFrames(sock)).toContainEqual({
      type: 'command',
      event: 'unsubscribe_channel',
      channel_id: 'channel-2',
      data: {},
    })
  })

  it('resubscribes and requests a reconnect snapshot after a dropped connection is restored', async () => {
    const ws = await useFreshWebSocket()
    ws.subscribe('channel-1')
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    sock.emitClose(1000, '')
    vi.advanceTimersByTime(reconnectDelay(1))
    expect(FakeWebSocket.instances).toHaveLength(2)

    const resumed = FakeWebSocket.instances[1]!
    openSocket(resumed)

    const frames = sentFrames(resumed)
    expect(frames).toContainEqual({
      type: 'command',
      event: 'subscribe_channel',
      channel_id: 'channel-1',
      data: {},
    })
    expect(frames).toContainEqual({ action: 'reconnect', seq: 1, data: {} })
    expect(ws.reconnectAttempt.value).toBe(0)
    expect(ws.connectionStatus.value).toBe('connected')
  })

  it('refetches messages for the current channel when the socket opens', async () => {
    mocks.channelStore.currentChannelId = 'channel-9'
    const ws = await useFreshWebSocket()

    ws.connect()
    openSocket(FakeWebSocket.instances[0]!)

    expect(mocks.messageStore.fetchMessages).toHaveBeenCalledWith('channel-9')
  })

  it('dispatches normalized event payloads to onEvent listeners and offEvent removes them', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    const listener = vi.fn()
    ws.onEvent('posted', listener)

    const payload = {
      post: { id: 'post-1', channel_id: 'channel-9', user_id: 'user-1', message: 'hi' },
    }
    sock.emitServerMessage({ type: 'event', event: 'posted', data: payload })

    expect(listener).toHaveBeenCalledTimes(1)
    expect(listener).toHaveBeenCalledWith(payload)
    // Own messages in this test must not trigger a desktop notification.
    expect(FakeNotification.instances).toHaveLength(0)

    ws.offEvent('posted', listener)
    sock.emitServerMessage({ type: 'event', event: 'posted', data: payload })
    expect(listener).toHaveBeenCalledTimes(1)
  })

  it('ignores malformed JSON frames without crashing or notifying listeners', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    const listener = vi.fn()
    ws.onEvent('posted', listener)

    expect(() => sock.onmessage?.({ data: '{"event": ' })).not.toThrow()
    expect(listener).not.toHaveBeenCalled()
    expect(ws.connected.value).toBe(true)
  })

  it('shows a desktop notification for new messages from other users in other channels', async () => {
    mocks.channelStore.currentChannelId = 'channel-1'
    mocks.channelStore.channels = [{ id: 'channel-2', name: 'general' }]
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    sock.emitServerMessage({
      type: 'event',
      event: 'posted',
      channel_id: 'channel-2',
      data: {
        post: {
          id: 'post-9',
          channel_id: 'channel-2',
          user_id: 'user-2',
          username: 'bob',
          message: 'hey there',
        },
      },
    })

    expect(mocks.messageStore.handleNewMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'post-9',
        channel_id: 'channel-2',
        user_id: 'user-2',
        message: 'hey there',
      })
    )
    expect(FakeNotification.instances).toHaveLength(1)
    expect(FakeNotification.instances[0]!.title).toBe('#general - bob')
    expect(FakeNotification.instances[0]!.options).toEqual({
      body: 'hey there',
      icon: '/favicon.ico',
    })
  })

  it('sends messages optimistically and reconciles them with the REST response', async () => {
    const ws = await useFreshWebSocket()
    mocks.postsApiCreate.mockResolvedValue({
      data: {
        id: 'server-post-1',
        channel_id: 'channel-1',
        user_id: 'user-1',
        message: 'hello',
        seq: 5,
      },
    })

    await ws.sendMessage('channel-1', 'hello')

    expect(mocks.randomUUID).toHaveBeenCalledTimes(1)
    expect(mocks.postsApiCreate).toHaveBeenCalledWith({
      channel_id: 'channel-1',
      message: 'hello',
      root_post_id: undefined,
      file_ids: [],
      client_msg_id: FIXED_CLIENT_MSG_ID,
    })
    expect(mocks.messageStore.addOptimisticMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        id: FIXED_CLIENT_MSG_ID,
        clientMsgId: FIXED_CLIENT_MSG_ID,
        channelId: 'channel-1',
        userId: 'user-1',
        username: 'alice',
        content: 'hello',
        status: 'sending',
      })
    )
    expect(mocks.postToMessage).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'server-post-1', message: 'hello' })
    )
    expect(mocks.messageStore.updateOptimisticMessage).toHaveBeenCalledWith(FIXED_CLIENT_MSG_ID, {
      id: 'server-post-1',
      channelId: 'channel-1',
      content: 'hello',
      status: 'delivered',
    })
  })

  it('marks the optimistic message as failed when the REST call rejects', async () => {
    const ws = await useFreshWebSocket()
    mocks.postsApiCreate.mockRejectedValue(new Error('network down'))

    await ws.sendMessage('channel-1', 'oops')

    expect(mocks.messageStore.updateOptimisticMessage).not.toHaveBeenCalled()
    const stored = mocks.messageStore.messagesByChannel['channel-1']
    expect(stored).toHaveLength(1)
    expect(stored![0]).toMatchObject({ id: FIXED_CLIENT_MSG_ID, status: 'failed' })
  })

  it('disconnects, resets the session, and does not resume the old connection', async () => {
    const ws = await useFreshWebSocket()
    ws.subscribe('channel-1')
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)
    sock.emitServerMessage({ type: 'event', event: 'hello', data: { connection_id: 'conn-abc' } })
    sock.emitServerMessage({ type: 'event', event: 'ack', seq: 4, data: null })

    ws.disconnect()

    expect(sock.readyState).toBe(FakeWebSocket.CLOSED)
    expect(ws.connected.value).toBe(false)
    // NOTE: connectionStatus becomes 'reconnecting' after an explicit
    // disconnect() because updateConnectionStatus treats a null disconnectedAt
    // as "reconnecting" (current behavior, arguably should be 'disconnected').
    expect(ws.connectionStatus.value).toBe('reconnecting')

    ws.connect()
    expect(FakeWebSocket.instances).toHaveLength(2)
    const second = FakeWebSocket.instances[1]!
    // reconnectAttempts was reset, so the fresh connect carries no resume params.
    expect(second.url).toBe(wsBaseUrl())
    openSocket(second)
    // Subscriptions were cleared by disconnect(), so nothing is re-sent.
    expect(sentFrames(second)).toHaveLength(0)

    // Even after a subsequent drop from the fresh session (reconnectAttempts
    // becomes 1), the URL carries no connection_id because disconnect() reset it.
    second.emitClose(1000, '')
    vi.advanceTimersByTime(reconnectDelay(1))
    expect(FakeWebSocket.instances).toHaveLength(3)
    expect(FakeWebSocket.instances[2]!.url).toBe(wsBaseUrl())
  })

  it('clears a pending reconnect timer on disconnect', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    sock.emitClose(1000, '') // schedules a reconnect in 2000ms
    ws.disconnect()

    vi.advanceTimersByTime(120_000)
    expect(FakeWebSocket.instances).toHaveLength(1)
  })

  it('documents that a browser-delivered close event after disconnect() still triggers a ghost reconnect', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    ws.disconnect()
    // A real browser still delivers a close event after a client-side close().
    // disconnect() does not detach the socket's onclose handler, so the full
    // reconnect path runs again even though the user disconnected on purpose.
    // This documents current (buggy) behavior; fix the source to flip this
    // test. Tracked in #320.
    sock.emitClose(1006, '')

    vi.advanceTimersByTime(reconnectDelay(1))
    expect(FakeWebSocket.instances).toHaveLength(2)
  })

  it('sends typing, stop-typing, and presence commands as actions', async () => {
    const ws = await useFreshWebSocket()
    ws.connect()
    const sock = FakeWebSocket.instances[0]!
    openSocket(sock)

    ws.sendTyping('channel-1', 'root-1')
    ws.sendStopTyping('channel-1', 'root-1')
    ws.sendPresence('online')

    expect(sentFrames(sock)).toEqual([
      { action: 'user_typing', seq: 1, data: { channel_id: 'channel-1', parent_id: 'root-1' } },
      {
        action: 'user_typing_stop',
        seq: 2,
        data: { channel_id: 'channel-1', parent_id: 'root-1' },
      },
      { type: 'command', event: 'presence', data: { status: 'online' } },
    ])
  })
})
