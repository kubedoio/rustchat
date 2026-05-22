import { expect, test, type Page } from '@playwright/test'

const TEST_EMAIL = 'settings-parity@example.com'

function makeUser() {
  return {
    id: '11111111-1111-1111-1111-111111111111',
    org_id: null,
    username: 'settingsparity',
    email: TEST_EMAIL,
    display_name: 'Settings Parity User',
    avatar_url: null,
    first_name: 'Settings',
    last_name: 'Parity',
    nickname: null,
    position: null,
    is_bot: false,
    role: 'member',
    presence: 'online',
    status_text: null,
    status_emoji: null,
    status_expires_at: null,
    custom_status: null,
    notify_props: {},
    timezone: 'UTC',
    email_verified: true,
    created_at: '2026-03-01T00:00:00.000Z',
  }
}

function makePreferences() {
  return {
    user_id: '11111111-1111-1111-1111-111111111111',
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
    mention_keywords: ['@settingsparity', '@channel', '@all', '@here'],
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
    send_on_ctrl_enter: true,
    enable_post_formatting: true,
    enable_join_leave_messages: true,
    enable_performance_debugging: false,
    unread_scroll_position: 'last',
    sync_drafts: true,
  }
}

async function mockApi(page: Page) {
  const user = makeUser()
  let preferences = makePreferences()

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

  await page.route('**/api/v1/oauth2/providers', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    })
  })

  await page.route('**/api/v1/auth/login', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        token: 'settings-parity-token',
        token_type: 'Bearer',
        expires_in: 86400,
        user,
      }),
    })
  })

  await page.route('**/api/v1/auth/me', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(user),
    })
  })

  await page.route('**/api/v1/theme/current', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        theme: 'light',
      }),
    })
  })

  await page.route('**/api/v1/users/me/preferences', async (route, request) => {
    if (request.method() === 'PUT') {
      const incoming = request.postDataJSON() as Record<string, unknown>
      preferences = {
        ...preferences,
        ...incoming,
      }
    }

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(preferences),
    })
  })

  await page.route('**/api/v4/notifications/test', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ status: 'OK' }),
    })
  })

  await page.route('**/api/v1/teams**', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    })
  })

  await page.route('**/api/v1/channels**', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    })
  })

  await page.route('**/api/v1/posts**', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ posts: {}, order: [], next_post_id: '' }),
    })
  })

  await page.route('**/api/v1/unreads/overview', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ channels: [], teams: [] }),
    })
  })
}

async function bootstrapAuthenticatedSession(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem('auth_token', 'settings-parity-token')
    document.cookie = 'MMAUTHTOKEN=settings-parity-token; path=/; SameSite=Strict'
  })

  await page.goto('/')
  await page.waitForLoadState('networkidle')
}

async function openSettingsFromUserMenu(page: Page) {
  const avatarTrigger = page.getByTestId('user-menu-trigger')
  await expect(avatarTrigger).toBeVisible({ timeout: 15000 })
  await avatarTrigger.click()

  const profileButton = page.getByRole('button', { name: 'Profile' })
  await expect(profileButton).toBeVisible({ timeout: 10000 })
  await profileButton.click()

  const settingsModal = page.locator('div[role="dialog"] > div.relative').first()
  await expect(settingsModal).toBeVisible({ timeout: 10000 })

  await page.addStyleTag({
    content:
      '*,:before,:after{animation:none!important;transition:none!important;caret-color:transparent!important;}',
  })

  return settingsModal
}

async function assertTabScreenshot(
  page: Page,
  modal: import('@playwright/test').Locator,
  tabName: string,
  filename: string
) {
  await page.getByRole('button', { name: tabName }).first().click()
  await page.waitForTimeout(200)

  await expect(modal).toHaveScreenshot(filename, {
    animations: 'disabled',
    caret: 'hide',
    maxDiffPixelRatio: 0.015,
  })
}

test('capture settings parity surfaces', async ({ page }) => {
  await page.setViewportSize({ width: 1512, height: 982 })
  await mockApi(page)
  await bootstrapAuthenticatedSession(page)

  const modal = await openSettingsFromUserMenu(page)

  await assertTabScreenshot(page, modal, 'Notifications', 'settings-notifications.png')
  await assertTabScreenshot(page, modal, 'Display', 'settings-display.png')
  await assertTabScreenshot(page, modal, 'Sidebar', 'settings-sidebar.png')
  await assertTabScreenshot(page, modal, 'Advanced', 'settings-advanced.png')
  await assertTabScreenshot(page, modal, 'Calls', 'settings-calls.png')
})
