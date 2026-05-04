import type { Page, Route } from '@playwright/test'

export const TEST_USER = {
  id: '11111111-1111-1111-1111-111111111111',
  username: 'testuser',
  email: 'test@example.com',
  display_name: 'Test User',
  role: 'member',
  presence: 'online',
}

export const TEST_TEAM = {
  id: 'team-1',
  name: 'workspace',
  display_name: 'Workspace',
  created_at: '2026-03-01T00:00:00.000Z',
}

export const ADAM_USER = {
  id: 'user-adam',
  username: 'adam',
  email: 'adam@example.com',
  display_name: 'Adam Builder',
  first_name: 'Adam',
  last_name: 'Builder',
  role: 'member',
  presence: 'online',
  status_text: 'Building carefully',
  status_emoji: ':hammer:',
}

const CHANNELS = [
  {
    id: 'test',
    team_id: TEST_TEAM.id,
    name: 'general',
    display_name: 'general',
    channel_type: 'public',
    unreadCount: 1,
    mentionCount: 0,
  },
  {
    id: 'channel-random',
    team_id: TEST_TEAM.id,
    name: 'random',
    display_name: 'random',
    channel_type: 'public',
    unreadCount: 0,
    mentionCount: 0,
  },
  {
    id: 'dm-adam',
    team_id: TEST_TEAM.id,
    name: 'dm_testuser_adam',
    display_name: 'Adam Builder',
    channel_type: 'direct',
    unreadCount: 0,
    mentionCount: 0,
  },
]

const TEAM_MEMBERS = [
  {
    team_id: TEST_TEAM.id,
    user_id: ADAM_USER.id,
    role: 'member',
    username: ADAM_USER.username,
    display_name: ADAM_USER.display_name,
    avatar_url: '',
    presence: ADAM_USER.presence,
    created_at: '2026-03-01T00:00:00.000Z',
  },
  {
    team_id: TEST_TEAM.id,
    user_id: TEST_USER.id,
    role: 'member',
    username: TEST_USER.username,
    display_name: TEST_USER.display_name,
    avatar_url: '',
    presence: TEST_USER.presence,
    created_at: '2026-03-01T00:00:00.000Z',
  },
]

const postsByChannel: Record<string, unknown[]> = {
  test: [
    {
      id: 'post-general-1',
      channel_id: 'test',
      user_id: ADAM_USER.id,
      username: ADAM_USER.username,
      avatar_url: '',
      message: 'Welcome to the websocket test channel',
      created_at: '2026-03-01T00:00:00.000Z',
      files: [],
      reactions: [],
    },
  ],
  'dm-adam': [
    {
      id: 'post-dm-1',
      channel_id: 'dm-adam',
      user_id: ADAM_USER.id,
      username: ADAM_USER.username,
      avatar_url: '',
      message: 'Direct message fixture',
      created_at: '2026-03-01T00:00:00.000Z',
      files: [],
      reactions: [],
    },
  ],
  'channel-random': [],
}

function json(route: Route, body: unknown, status = 200) {
  return route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify(body),
  })
}

export async function mockChatApi(page: Page) {
  await page.route('**/api/v1/**', async (route, request) => {
    const url = new URL(request.url())
    const path = url.pathname

    if (path === '/api/v1/site/info') {
      await json(route, {
        site_name: 'RustChat',
        logo_url: null,
        enable_sso: false,
        require_sso: false,
      })
      return
    }

    if (path === '/api/v1/oauth2/providers') {
      await json(route, [])
      return
    }

    if (path === '/api/v1/auth/login' && request.method() === 'POST') {
      await json(route, { token: 'chat-fixture-token', user: TEST_USER })
      return
    }

    if (path === '/api/v1/auth/me') {
      await json(route, TEST_USER)
      return
    }

    if (path === '/api/v1/theme/current') {
      await json(route, { theme: 'light' })
      return
    }

    if (path === '/api/v1/users/me/preferences') {
      await json(route, {})
      return
    }

    if (path === '/api/v1/unreads/overview') {
      await json(route, {
        channels: [{ channel_id: 'test', unread_count: 1 }],
        teams: [{ team_id: TEST_TEAM.id, unread_count: 1 }],
      })
      return
    }

    if (path === '/api/v1/teams') {
      await json(route, [TEST_TEAM])
      return
    }

    if (path === `/api/v1/teams/${TEST_TEAM.id}/members`) {
      await json(route, TEAM_MEMBERS)
      return
    }

    if (path === '/api/v1/channels') {
      await json(route, CHANNELS)
      return
    }

    const channelPostsMatch = path.match(/^\/api\/v1\/channels\/([^/]+)\/posts$/)
    if (channelPostsMatch) {
      const channelId = channelPostsMatch[1] ?? ''
      await json(route, {
        messages: postsByChannel[channelId] ?? [],
        read_state: {
          last_read_message_id: null,
          first_unread_message_id: null,
        },
      })
      return
    }

    const userMatch = path.match(/^\/api\/v1\/users\/([^/]+)$/)
    if (userMatch) {
      await json(route, userMatch[1] === ADAM_USER.id ? ADAM_USER : TEST_USER)
      return
    }

    await json(route, {})
  })

  await page.route('**/api/v4/**', async (route) => {
    await json(route, {})
  })
}

export async function installStableWebSocket(page: Page) {
  await page.addInitScript(() => {
    class StableMockWebSocket {
      static CONNECTING = 0
      static OPEN = 1
      static CLOSING = 2
      static CLOSED = 3

      readyState = StableMockWebSocket.CONNECTING
      onopen: ((event?: Event) => void) | null = null
      onmessage: ((event?: MessageEvent) => void) | null = null
      onclose: ((event?: CloseEvent) => void) | null = null
      onerror: ((event?: Event) => void) | null = null

      constructor(_url: string, _protocols?: string | string[]) {
        setTimeout(() => {
          this.readyState = StableMockWebSocket.OPEN
          this.onopen?.(new Event('open'))
        }, 0)
      }

      send(_data: string) {}

      close() {
        if (this.readyState === StableMockWebSocket.CLOSED) {
          return
        }

        this.readyState = StableMockWebSocket.CLOSED
        this.onclose?.(new CloseEvent('close', { code: 1000, reason: 'client close' }))
      }
    }

    ;(window as any).WebSocket = StableMockWebSocket
  })
}
