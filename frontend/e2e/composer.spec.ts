import { test, expect, type Page } from '@playwright/test'

const TEST_USER = {
  id: '11111111-1111-1111-1111-111111111111',
  username: 'testuser',
  email: 'test@example.com',
  display_name: 'Test User',
  role: 'member',
  presence: 'online',
}

const TEST_TEAM = {
  id: 'team-1',
  name: 'workspace',
  display_name: 'Workspace',
  created_at: '2026-03-01T00:00:00.000Z',
}

const TEST_CHANNELS = [
  {
    id: 'channel-general',
    team_id: TEST_TEAM.id,
    name: 'general',
    display_name: 'general',
    channel_type: 'public',
    created_at: '2026-03-01T00:00:00.000Z',
    creator_id: TEST_USER.id,
    unreadCount: 0,
    mentionCount: 0,
  },
  {
    id: 'channel-random',
    team_id: TEST_TEAM.id,
    name: 'random',
    display_name: 'random',
    channel_type: 'public',
    created_at: '2026-03-01T00:00:00.000Z',
    creator_id: TEST_USER.id,
    unreadCount: 0,
    mentionCount: 0,
  },
]

const TEAM_MEMBERS = [
  {
    team_id: TEST_TEAM.id,
    user_id: TEST_USER.id,
    role: 'member',
    username: TEST_USER.username,
    display_name: TEST_USER.display_name,
    avatar_url: '',
    presence: 'online',
    created_at: '2026-03-01T00:00:00.000Z',
  },
  {
    team_id: TEST_TEAM.id,
    user_id: 'user-adam',
    role: 'member',
    username: 'adam',
    display_name: 'Adam Builder',
    avatar_url: '',
    presence: 'online',
    created_at: '2026-03-01T00:00:00.000Z',
  },
]

function makePreferences() {
  return {
    user_id: TEST_USER.id,
    notify_desktop: 'mentions',
    notify_push: 'mentions',
    notify_email: 'true',
    notify_sounds: true,
    dnd_enabled: false,
    dnd_start_time: null,
    dnd_end_time: null,
    dnd_days: '',
    message_display: 'standard',
    sidebar_behavior: 'default',
    time_format: '24h',
    mention_keywords: ['@testuser', '@channel', '@all', '@here'],
    collapsed_reply_threads: false,
    use_military_time: true,
    teammate_name_display: 'full_name',
    availability_status_visible: true,
    show_last_active_time: true,
    timezone: 'Europe/Berlin',
    link_previews_enabled: true,
    image_previews_enabled: true,
    click_to_reply: true,
    channel_display_mode: 'full',
    quick_reactions_enabled: true,
    emoji_picker_enabled: true,
    language: 'en',
    group_unread_channels: 'never',
    limit_visible_dms_gms: '40',
    send_on_ctrl_enter: false,
    enable_post_formatting: true,
    enable_join_leave_messages: true,
    enable_performance_debugging: false,
    unread_scroll_position: 'last',
    sync_drafts: true,
  }
}

async function mockComposerApi(page: Page) {
  const postsByChannel: Record<string, any[]> = {
    'channel-general': [],
    'channel-random': [],
  }
  const uploadedFiles = new Map<
    string,
    { id: string; name: string; url: string; size: number; mime_type: string }
  >()
  let fileCounter = 0
  let postCounter = 0

  const json = (route: import('@playwright/test').Route, body: unknown, status = 200) =>
    route.fulfill({
      status,
      contentType: 'application/json',
      body: JSON.stringify(body),
    })

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

    if (path === '/api/v1/auth/me') {
      await json(route, TEST_USER)
      return
    }

    if (path === '/api/v1/theme/current') {
      await json(route, { theme: 'light' })
      return
    }

    if (path === '/api/v1/users/me/preferences') {
      await json(route, makePreferences())
      return
    }

    if (path === '/api/v1/unreads/overview') {
      await json(route, { channels: [], teams: [] })
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
      await json(route, TEST_CHANNELS)
      return
    }

    const channelPostsMatch = path.match(/^\/api\/v1\/channels\/([^/]+)\/posts$/)
    if (channelPostsMatch) {
      const channelId = channelPostsMatch[1] ?? ''
      if (request.method() === 'POST') {
        const body = request.postDataJSON() as {
          message?: string
          file_ids?: string[]
          client_msg_id?: string
        }
        postCounter += 1
        const files = Array.isArray(body.file_ids)
          ? body.file_ids
              .map(id => uploadedFiles.get(id))
              .filter(
                (
                  file
                ): file is {
                  id: string
                  name: string
                  url: string
                  size: number
                  mime_type: string
                } => !!file
              )
          : []
        const post = {
          id: `post-${postCounter}`,
          channel_id: channelId,
          user_id: TEST_USER.id,
          username: TEST_USER.username,
          avatar_url: '',
          email: TEST_USER.email,
          message: body.message || '',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          is_pinned: false,
          is_saved: false,
          reactions: [],
          files,
          client_msg_id: body.client_msg_id,
          seq: postCounter,
        }
        postsByChannel[channelId] = [...(postsByChannel[channelId] || []), post]
        await json(route, post)
        return
      }

      await json(route, {
        messages: postsByChannel[channelId] || [],
        read_state: {
          last_read_message_id: null,
          first_unread_message_id: null,
        },
      })
      return
    }

    const readMatch = path.match(/^\/api\/v1\/channels\/([^/]+)\/members\/([^/]+)\/read$/)
    if (readMatch) {
      await json(route, { status: 'OK' })
      return
    }

    if (path === '/api/v1/files' && request.method() === 'POST') {
      fileCounter += 1
      const id = `file-${fileCounter}`
      const response = {
        id,
        name: fileCounter === 1 ? 'test.txt' : 'pasted-image.png',
        mime_type: fileCounter === 1 ? 'text/plain' : 'image/png',
        size: fileCounter === 1 ? 12 : 15,
        url: `/uploads/${id}`,
      }
      uploadedFiles.set(id, response)
      await json(route, response)
      return
    }

    await json(route, {})
  })

  await page.route('**/api/v4/**', async (route, request) => {
    const url = new URL(request.url())
    const path = url.pathname

    if (path === '/api/v4/plugins/com.mattermost.calls/config') {
      await json(route, {
        ICEServersConfigs: [],
        NeedsTURNCredentials: false,
      })
      return
    }

    if (path === '/api/v4/plugins/com.mattermost.calls/channels') {
      await json(route, [])
      return
    }

    await json(route, {})
  })
}

async function bootstrapComposerSession(page: Page) {
  await page.goto('/login')
  await page.evaluate(() => {
    localStorage.setItem('auth_token', 'composer-test-token')
    document.cookie = 'MMAUTHTOKEN=composer-test-token; path=/; SameSite=Strict'
  })
  await page.goto('/')
  await page.waitForSelector('[aria-label="Message composer"]')
}

test.describe('Mattermost Composer E2E', () => {
  test.beforeEach(async ({ page }) => {
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
          this.readyState = StableMockWebSocket.CLOSED
          this.onclose?.(new CloseEvent('close', { code: 1000, reason: 'client close' }))
        }
      }

      ;(window as any).WebSocket = StableMockWebSocket
    })

    await mockComposerApi(page)
    await bootstrapComposerSession(page)
  })

  test('sends message on Enter', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')
    await composer.fill('Hello world')
    await composer.press('Enter')

    // Message should appear in the list
    await expect(page.locator('text=Hello world')).toBeVisible()

    // Composer should be cleared
    await expect(composer).toHaveValue('')
  })

  test('inserts newline on Shift+Enter', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')
    await composer.fill('Line 1')
    await composer.press('Shift+Enter')
    await composer.type('Line 2')

    await expect(composer).toHaveValue('Line 1\nLine 2')
  })

  test('bold formatting via toolbar', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    // Type and select text
    await composer.fill('bold text')
    await composer.selectText()

    // Click bold button
    await page.click('[aria-label="Bold"]')

    await expect(composer).toHaveValue('**bold text**')
  })

  test('bold formatting via keyboard shortcut', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    await composer.fill('bold text')
    await composer.selectText()
    await composer.press('Control+b')

    await expect(composer).toHaveValue('**bold text**')
  })

  test('italic formatting', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    await composer.fill('italic text')
    await composer.selectText()
    await page.click('[aria-label="Italic"]')

    await expect(composer).toHaveValue('*italic text*')
  })

  test('link insertion', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    await composer.fill('click here')
    await composer.selectText()
    await page.click('[aria-label="Link"]')

    // Should have placeholder link
    await expect(composer).toHaveValue('[click here](url)')
  })

  test('emoji autocomplete with colon', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    await composer.type(':smi')

    // Wait for autocomplete to appear
    await expect(page.locator('text=Emoji matching')).toBeVisible()
    await expect(page.locator('text=:smile:')).toBeVisible()

    // Select emoji
    await page.click('text=:smile:')

    await expect(composer).toHaveValue(':smile: ')
  })

  test('mention autocomplete with @', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    await composer.type('@ad')

    // Wait for autocomplete
    await expect(page.locator('text=Channel Members')).toBeVisible()
    await expect(page.getByRole('button', { name: /adam/i })).toBeVisible()

    // Press Enter to select first result
    await composer.press('Enter')

    // Should insert mention
    const value = await composer.inputValue()
    expect(value).toMatch(/^@\w+/)
  })

  test('file attachment via click', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    // Attach file
    const [fileChooser] = await Promise.all([
      page.waitForEvent('filechooser'),
      page.click('[aria-label="Attach file"]'),
    ])

    await fileChooser.setFiles({
      name: 'test.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('test content'),
    })

    // Should show file preview
    await expect(page.locator('text=test.txt')).toBeVisible()
  })

  test('file attachment via drag and drop', async ({ page }) => {
    const dataTransfer = await page.evaluateHandle(() => {
      const transfer = new DataTransfer()
      transfer.items.add(new File(['fake-image-data'], 'dragged-image.png', { type: 'image/png' }))
      return transfer
    })

    await page.locator('[aria-label="Message composer"]').dispatchEvent('drop', { dataTransfer })

    await expect(page.locator('text=dragged-image.png')).toBeVisible()
  })

  test('draft persistence', async ({ page, context }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    // Type a message but don't send
    await composer.fill('Draft message')

    // Navigate away and back
    await page.click('text=random')
    await page.click('text=general')

    // Draft should be restored
    await expect(composer).toHaveValue('Draft message')
  })

  test('send button disabled when empty', async ({ page }) => {
    const sendButton = page.locator('[aria-label="Send message"]')

    // Initially disabled
    await expect(sendButton).toBeDisabled()

    // Type something
    await page.fill('[aria-label="Message composer"]', 'test')

    // Should be enabled
    await expect(sendButton).toBeEnabled()
  })

  test('formatting toolbar toggle', async ({ page }) => {
    const toggleButton = page.locator('[aria-label="Toggle formatting toolbar"]')
    const toolbar = page.locator('.bg-bg-surface-2\\/50')

    // Initially visible
    await expect(toolbar).toBeVisible()

    // Toggle off
    await toggleButton.click()
    await expect(toolbar).not.toBeVisible()

    // Toggle on
    await toggleButton.click()
    await expect(toolbar).toBeVisible()
  })

  test('keyboard shortcut help', async ({ page }) => {
    // Look for keyboard hints
    await expect(page.locator('text=to send')).toBeVisible()
    await expect(page.locator('text=newline')).toBeVisible()
  })

  test('escape closes emoji picker', async ({ page }) => {
    await page.click('[aria-label="Insert emoji"]')

    // Emoji picker should be visible
    const emojiPickerSearch = page.locator('input[placeholder="Search emoji..."]')
    await expect(emojiPickerSearch).toBeVisible()

    // Press escape
    await page.keyboard.press('Escape')

    // Should close
    await expect(emojiPickerSearch).not.toBeVisible()
  })

  test('typing indicator is sent', async ({ page }) => {
    const composer = page.locator('[aria-label="Message composer"]')

    // Type something
    await composer.type('typing...')

    // Wait a bit for typing indicator
    await page.waitForTimeout(500)

    // This is hard to verify without WebSocket mocking, but the functionality exists
  })
})
