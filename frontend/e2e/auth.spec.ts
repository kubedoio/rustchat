import { test, expect } from '@playwright/test'

const TEST_USER = {
  id: '11111111-1111-1111-1111-111111111111',
  username: 'testuser',
  email: 'test@example.com',
  role: 'member',
}

function createJwtWithExp(expSeconds: number): string {
  const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64url')
  const payload = Buffer.from(
    JSON.stringify({
      sub: TEST_USER.id,
      email: TEST_USER.email,
      role: TEST_USER.role,
      iat: expSeconds - 60,
      exp: expSeconds,
    })
  ).toString('base64url')
  return `${header}.${payload}.signature`
}

async function mockAuthenticatedSessionApi(page: import('@playwright/test').Page) {
  const handleApiRoute = async (
    route: import('@playwright/test').Route,
    request: import('@playwright/test').Request
  ) => {
    const url = new URL(request.url())
    const path = url.pathname

    if (path === '/api/v1/site/info') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          site_name: 'RustChat',
          logo_url: null,
          enable_sso: false,
          require_sso: false,
        }),
      })
      return
    }

    if (path === '/api/v1/oauth2/providers') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      })
      return
    }

    if (path === '/api/v1/auth/me') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(TEST_USER),
      })
      return
    }

    if (path === '/api/v1/theme/current') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ theme: 'light' }),
      })
      return
    }

    if (path === '/api/v1/unreads/overview') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ channels: [], teams: [] }),
      })
      return
    }

    if (path === '/api/v4/plugins/com.mattermost.calls/config') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          ICEServersConfigs: [],
          NeedsTURNCredentials: false,
        }),
      })
      return
    }

    if (path === '/api/v4/plugins/com.mattermost.calls/channels') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      })
      return
    }

    if (path.startsWith('/api/v1/teams')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      })
      return
    }

    if (path.startsWith('/api/v1/channels')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      })
      return
    }

    if (path.startsWith('/api/v1/posts')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ posts: {}, order: [], next_post_id: '' }),
      })
      return
    }

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({}),
    })
  }

  await page.route('**/api/v1/**', handleApiRoute)
  await page.route('**/api/v4/**', handleApiRoute)
}

async function seedAuthenticatedSession(page: import('@playwright/test').Page, token: string) {
  await page.goto('/login')
  await page.evaluate(tokenValue => {
    localStorage.setItem('auth_token', tokenValue)
    document.cookie = `MMAUTHTOKEN=${tokenValue}; path=/; SameSite=Strict`
  }, token)
}

test('should allow a user to sign in', async ({ page }) => {
  // Mock site info
  await page.route('**/api/v1/site/info', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        site_name: 'RustChat',
        logo_url: null,
      }),
    })
  })

  // Mock login API
  await page.route('**/api/v1/auth/login', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        token: 'fake-jwt-token',
        user: {
          id: '123',
          username: 'testuser',
          email: 'test@example.com',
          role: 'member',
        },
      }),
    })
  })

  // Mock me API
  await page.route('**/api/v1/auth/me', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        id: '123',
        username: 'testuser',
        email: 'test@example.com',
        role: 'member',
      }),
    })
  })

  // Mock OAuth2 providers
  await page.route('**/api/v1/oauth2/providers', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    })
  })

  await page.goto('/login')

  // Fill in credentials
  await page.fill('#email', 'test@example.com')
  await page.fill('#password', 'password')

  // Click login button
  await page.click('button[type="submit"]')

  // Expect redirect to dashboard (/)
  await expect(page).toHaveURL('/')
})

test('logs out and redirects to /login when JWT expires during active session', async ({
  page,
}) => {
  const expSeconds = Math.floor(Date.now() / 1000) + 2
  const shortLivedToken = createJwtWithExp(expSeconds)

  await mockAuthenticatedSessionApi(page)
  await seedAuthenticatedSession(page, shortLivedToken)

  await page.goto('/')
  await expect(page).toHaveURL('/login', { timeout: 12000 })
  await expect(page.locator('#email')).toBeVisible()

  const storedToken = await page.evaluate(() => localStorage.getItem('auth_token'))
  expect(storedToken ?? '').toBe('')

  const cookieState = await page.evaluate(() => document.cookie)
  expect(cookieState).not.toContain('MMAUTHTOKEN=')
})

test('logs out on websocket auth-expiry close and does not reconnect', async ({ page }) => {
  const expSeconds = Math.floor(Date.now() / 1000) + 3600
  const longLivedToken = createJwtWithExp(expSeconds)

  await mockAuthenticatedSessionApi(page)
  await page.addInitScript(() => {
    ;(window as any).__mockWsCreateCount = 0

    class MockWebSocket {
      static CONNECTING = 0
      static OPEN = 1
      static CLOSING = 2
      static CLOSED = 3

      public readyState = MockWebSocket.CONNECTING
      public onopen: ((event?: any) => void) | null = null
      public onmessage: ((event?: any) => void) | null = null
      public onclose: ((event?: any) => void) | null = null
      public onerror: ((event?: any) => void) | null = null

      constructor(_url: string, _protocols?: string | string[]) {
        ;(window as any).__mockWsCreateCount += 1

        setTimeout(() => {
          this.readyState = MockWebSocket.OPEN
          this.onopen?.({ type: 'open' })
          setTimeout(() => {
            this.readyState = MockWebSocket.CLOSED
            this.onclose?.({
              code: 1008,
              reason: 'Authentication token expired',
            })
          }, 100)
        }, 0)
      }

      send(_data: string) {}

      close() {
        if (this.readyState === MockWebSocket.CLOSED) {
          return
        }
        this.readyState = MockWebSocket.CLOSED
        this.onclose?.({ code: 1000, reason: 'client close' })
      }
    }

    ;(window as any).WebSocket = MockWebSocket as any
  })

  await seedAuthenticatedSession(page, longLivedToken)

  await page.goto('/')
  await expect(page).toHaveURL('/')

  await expect(page).toHaveURL('/login', { timeout: 12000 })

  const storedToken = await page.evaluate(() => localStorage.getItem('auth_token'))
  expect(storedToken ?? '').toBe('')

  await page.waitForTimeout(1000)
  const wsCreateCountAfterLoginSettles = await page.evaluate(
    () => (window as any).__mockWsCreateCount ?? 0
  )
  await page.waitForTimeout(2000)
  const wsCreateCountAfterCooldown = await page.evaluate(
    () => (window as any).__mockWsCreateCount ?? 0
  )
  expect(wsCreateCountAfterCooldown).toBe(wsCreateCountAfterLoginSettles)
})
