import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { ref } from 'vue'

const resetMessages = vi.fn()
const clearChannels = vi.fn()
const clearUnreads = vi.fn()
const clearPresence = vi.fn()
const clearTeams = vi.fn()
const clearPreferences = vi.fn()
const clearUserSummaryCache = vi.fn()
const closeVideoCall = vi.fn()
const closeRhs = vi.fn()
const closeSettings = vi.fn()
const closeLhs = vi.fn()
const resetCalls = vi.fn()
const syncTheme = vi.fn()
const replaceLocation = vi.fn()

vi.mock('@vueuse/core', () => ({
  useSessionStorage: (_key: string, initialValue: string) => ref(initialValue),
}))

vi.mock('../../../composables/useUserSummary', () => ({
  clearUserSummaryCache,
}))

vi.mock('../../presence', () => ({
  usePresenceStore: () => ({
    clear: clearPresence,
  }),
}))

vi.mock('@/features/messages/stores/messageStore', () => ({
  useMessageStore: () => ({
    resetSessionState: resetMessages,
  }),
}))

vi.mock('@/features/channels/stores/channelStore', () => ({
  useChannelStore: () => ({
    clearChannels,
  }),
}))

vi.mock('@/features/unreads/stores/unreadStore', () => ({
  useUnreadStore: () => ({
    clearAllState: clearUnreads,
  }),
}))

vi.mock('@/features/teams/stores/teamStore', () => ({
  useTeamStore: () => ({
    clear: clearTeams,
  }),
}))

vi.mock('@/features/channels/stores/channelPreferencesStore', () => ({
  useChannelPreferencesStore: () => ({
    clearState: clearPreferences,
  }),
}))

vi.mock('../../ui/stores/uiStore', () => ({
  useUIStore: () => ({
    closeVideoCall,
    closeRhs,
    closeSettings,
    closeLhs,
  }),
}))

vi.mock('@/stores/calls', () => ({
  useCallsStore: () => ({
    resetSessionState: resetCalls,
  }),
}))

vi.mock('../../theme/stores/themeStore', () => ({
  useThemeStore: () => ({
    syncFromServer: syncTheme,
  }),
}))

vi.mock('../../../api/client', () => ({
  default: {
    post: vi.fn(),
    get: vi.fn(),
    put: vi.fn(),
  },
}))

describe('auth logout session cleanup', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    ;(globalThis as any).localStorage = {
      setItem: vi.fn(),
    }
    ;(globalThis as any).document = {
      cookie: '',
    }
    ;(globalThis as any).window = {
      location: {
        pathname: '/',
        replace: replaceLocation,
      },
    }
  })

  it('clears the active presence path and user summary cache on logout', async () => {
    const { useAuthStore } = await import('./authStore')
    const store = useAuthStore()

    store.token = 'token-value'
    await store.logout()

    expect(clearPresence).toHaveBeenCalledTimes(1)
    expect(clearUserSummaryCache).toHaveBeenCalledTimes(1)
    expect(resetMessages).toHaveBeenCalledTimes(1)
    expect(clearChannels).toHaveBeenCalledTimes(1)
    expect(clearUnreads).toHaveBeenCalledTimes(1)
    expect(clearTeams).toHaveBeenCalledTimes(1)
    expect(clearPreferences).toHaveBeenCalledTimes(1)
    expect(resetCalls).toHaveBeenCalledTimes(1)
    expect(replaceLocation).toHaveBeenCalledWith('/login')
  })
})
