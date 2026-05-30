import { computed, ref, watch, type Ref } from 'vue'

type TabLock = {
  id: string
  ts: number
}

const ACTIVE_TAB_KEY = 'rustchat:active_tab'
const CHANNEL_NAME = 'rustchat:active_tab'
const HEARTBEAT_MS = 2000
const STALE_LOCK_MS = 8000

function safeParseLock(raw: string | null): TabLock | null {
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') {
      return null
    }
    const maybeLock = parsed as { id?: unknown; ts?: unknown }
    if (typeof maybeLock.id !== 'string' || typeof maybeLock.ts !== 'number') {
      return null
    }
    return { id: maybeLock.id, ts: maybeLock.ts }
  } catch {
    return null
  }
}

export function useSingleActiveTab(enabled: Ref<boolean>) {
  const tabId = `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
  const activeTabId = ref<string | null>(null)
  const lockTimestamp = ref(0)

  let started = false
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null
  let livenessTimer: ReturnType<typeof setInterval> | null = null
  let channel: BroadcastChannel | null = null

  const isActiveTab = computed(() => {
    if (!enabled.value) return true
    return activeTabId.value === tabId
  })

  function readLock(): TabLock | null {
    return safeParseLock(localStorage.getItem(ACTIVE_TAB_KEY))
  }

  function writeLock(next: TabLock) {
    localStorage.setItem(ACTIVE_TAB_KEY, JSON.stringify(next))
    activeTabId.value = next.id
    lockTimestamp.value = next.ts
  }

  function clearLock() {
    localStorage.removeItem(ACTIVE_TAB_KEY)
    if (activeTabId.value === tabId) {
      activeTabId.value = null
      lockTimestamp.value = 0
    }
  }

  function isStale(lock: TabLock | null): boolean {
    if (!lock) return true
    return Date.now() - lock.ts > STALE_LOCK_MS
  }

  function syncFromStorage() {
    const lock = readLock()
    activeTabId.value = lock?.id ?? null
    lockTimestamp.value = lock?.ts ?? 0
  }

  function claimActiveTab() {
    if (!enabled.value) return
    const next = { id: tabId, ts: Date.now() }
    writeLock(next)
    channel?.postMessage({ type: 'claim', ...next })
  }

  function heartbeat() {
    if (!enabled.value || !isActiveTab.value) return
    const next = { id: tabId, ts: Date.now() }
    writeLock(next)
    channel?.postMessage({ type: 'heartbeat', ...next })
  }

  function releaseIfOwned() {
    const lock = readLock()
    if (lock?.id === tabId) {
      clearLock()
      channel?.postMessage({ type: 'release', id: tabId, ts: Date.now() })
    }
  }

  function evaluateLock() {
    if (!enabled.value) return
    const lock = readLock()
    if (isActiveTab.value) {
      heartbeat()
      return
    }
    if (document.visibilityState === 'visible' && isStale(lock)) {
      claimActiveTab()
      return
    }
    syncFromStorage()
  }

  function onStorage(event: StorageEvent) {
    if (event.key !== ACTIVE_TAB_KEY) return
    syncFromStorage()
  }

  function onVisibilityChange() {
    if (!enabled.value) return
    if (document.visibilityState !== 'visible') return
    evaluateLock()
  }

  function onChannelMessage(event: MessageEvent) {
    const data = event.data as { id?: string; ts?: number } | null
    if (!data || typeof data.id !== 'string' || typeof data.ts !== 'number') {
      return
    }
    activeTabId.value = data.id
    lockTimestamp.value = data.ts
  }

  function start() {
    if (started) return
    started = true

    syncFromStorage()
    const current = readLock()
    if (document.visibilityState === 'visible' && isStale(current)) {
      claimActiveTab()
    }

    if (typeof BroadcastChannel !== 'undefined') {
      channel = new BroadcastChannel(CHANNEL_NAME)
      channel.addEventListener('message', onChannelMessage)
    }

    window.addEventListener('storage', onStorage)
    window.addEventListener('beforeunload', releaseIfOwned)
    window.addEventListener('pagehide', releaseIfOwned)
    document.addEventListener('visibilitychange', onVisibilityChange)

    heartbeatTimer = setInterval(heartbeat, HEARTBEAT_MS)
    livenessTimer = setInterval(evaluateLock, HEARTBEAT_MS)
  }

  function stop() {
    if (!started) return
    started = false

    if (heartbeatTimer) {
      clearInterval(heartbeatTimer)
      heartbeatTimer = null
    }
    if (livenessTimer) {
      clearInterval(livenessTimer)
      livenessTimer = null
    }

    window.removeEventListener('storage', onStorage)
    window.removeEventListener('beforeunload', releaseIfOwned)
    window.removeEventListener('pagehide', releaseIfOwned)
    document.removeEventListener('visibilitychange', onVisibilityChange)

    if (channel) {
      channel.removeEventListener('message', onChannelMessage)
      channel.close()
      channel = null
    }

    releaseIfOwned()
    activeTabId.value = null
    lockTimestamp.value = 0
  }

  watch(
    enabled,
    isEnabled => {
      if (isEnabled) {
        start()
      } else {
        stop()
      }
    },
    { immediate: true }
  )

  return {
    isActiveTab,
    takeOver: claimActiveTab,
  }
}
