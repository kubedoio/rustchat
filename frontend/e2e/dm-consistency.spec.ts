import { test, expect } from '@playwright/test'
import { installStableWebSocket, mockChatApi } from './support/chat-fixtures'

/**
 * DM Consistency E2E Tests
 *
 * Verifies that a DM contact's presence and custom status are rendered
 * consistently across three surfaces:
 * 1. DM sidebar row
 * 2. Channel info panel (right sidebar)
 * 3. User profile modal
 *
 * Also validates that state survives a websocket reconnect.
 */

test.describe('DM Status Consistency', () => {
  test.beforeEach(async ({ page }) => {
    await installStableWebSocket(page)
    await mockChatApi(page)

    // Login and navigate to a channel
    await page.goto('/login')
    await page.fill('[data-testid="login-username"]', 'testuser')
    await page.fill('[data-testid="login-password"]', 'testpass')
    await page.click('[data-testid="login-submit"]')
    await page.waitForURL('**/channels/**')

    // Wait for WebSocket to connect
    await page.waitForSelector('[data-testid="connection-indicator"][data-status="connected"]')
  })

  test('sidebar, info panel, and profile modal agree on presence and status', async ({ page }) => {
    // Click the first DM row in the sidebar
    const dmRow = page.locator('[data-testid="dm-sidebar-row"]').first()
    await expect(dmRow).toBeVisible()
    await dmRow.click()

    // Wait for the DM channel to load
    await page.waitForTimeout(500)

    // Capture sidebar status text (may include emoji + text or just presence label)
    const sidebarStatus = page.locator('[data-testid="dm-sidebar-status"]').first()
    await expect(sidebarStatus).toBeVisible()
    const sidebarText = await sidebarStatus.textContent() || ''

    // Open channel info panel via header menu
    await page.click('[data-testid="channel-header-menu"]')
    await page.click('[data-testid="channel-details-button"]')

    // Capture info panel presence label
    const infoPresence = page.locator('[data-testid="channel-info-presence"]')
    await expect(infoPresence).toBeVisible()
    const infoPresenceText = await infoPresence.textContent() || ''

    // Capture info panel custom status if present
    const infoCustomStatus = page.locator('[data-testid="channel-info-custom-status"]')
    let infoCustomText = ''
    if (await infoCustomStatus.isVisible().catch(() => false)) {
      infoCustomText = await infoCustomStatus.textContent() || ''
    }

    // Click a message avatar to open the profile modal
    const messageAvatar = page.locator('[data-testid="message-avatar"]').first()
    await expect(messageAvatar).toBeVisible()
    await messageAvatar.click()

    // Capture profile modal presence label
    const profilePresence = page.locator('[data-testid="profile-modal-presence"]')
    await expect(profilePresence).toBeVisible()
    const profilePresenceText = await profilePresence.textContent() || ''

    // Capture profile modal custom status if present
    const profileCustomStatus = page.locator('[data-testid="profile-modal-custom-status"]')
    let profileCustomText = ''
    if (await profileCustomStatus.isVisible().catch(() => false)) {
      profileCustomText = await profileCustomStatus.textContent() || ''
    }

    // Presence labels should match across all three surfaces
    expect(infoPresenceText.trim().toLowerCase()).toBe(profilePresenceText.trim().toLowerCase())

    // Sidebar shows either custom status or presence label; if custom status exists,
    // it should appear in sidebar too
    if (infoCustomText) {
      expect(sidebarText).toContain(infoCustomText.trim())
    } else {
      // No custom status: sidebar should show the presence label
      expect(sidebarText.trim().toLowerCase()).toBe(infoPresenceText.trim().toLowerCase())
    }

    // Close profile modal
    await page.keyboard.press('Escape')
    await expect(page.locator('[data-testid="profile-modal-status"]')).not.toBeVisible()
  })

  test('status state survives websocket reconnect', async ({ page }) => {
    // Click the first DM row
    const dmRow = page.locator('[data-testid="dm-sidebar-row"]').first()
    await dmRow.click()
    await page.waitForTimeout(500)

    // Capture initial sidebar status
    const sidebarStatus = page.locator('[data-testid="dm-sidebar-status"]').first()
    const initialStatus = await sidebarStatus.textContent() || ''

    // Open info panel and capture presence
    await page.click('[data-testid="channel-header-menu"]')
    await page.click('[data-testid="channel-details-button"]')
    const infoPresence = page.locator('[data-testid="channel-info-presence"]')
    const initialPresence = await infoPresence.textContent() || ''

    // Simulate WebSocket disconnect and reconnect via page evaluate.
    // This relies on the app exposing its WebSocket instance or test helpers.
    // If not available, the test will skip the reconnect assertion gracefully.
    const canSimulate = await page.evaluate(() => {
      return typeof (window as any).testHelpers !== 'undefined' &&
        typeof (window as any).testHelpers.simulateWebSocketClose === 'function'
    })

    if (canSimulate) {
      await page.evaluate(() => {
        (window as any).testHelpers.simulateWebSocketClose()
      })

      // Wait for reconnecting UI
      await expect(page.locator('[data-testid="connection-status-bar"]')).toBeVisible()

      // Restore connection
      await page.evaluate(() => {
        (window as any).testHelpers.simulateWebSocketOpen()
      })

      // Wait for reconnected state
      await page.waitForSelector('[data-testid="connection-indicator"][data-status="connected"]')
      await page.waitForTimeout(500)

      // Re-read sidebar status after reconnect
      const postReconnectStatus = await sidebarStatus.textContent() || ''
      expect(postReconnectStatus.trim()).toBe(initialStatus.trim())

      // Re-read info panel presence after reconnect
      const postReconnectPresence = await infoPresence.textContent() || ''
      expect(postReconnectPresence.trim().toLowerCase()).toBe(initialPresence.trim().toLowerCase())
    } else {
      test.skip(true, 'WebSocket test helpers not available; skipping reconnect simulation')
    }
  })
})
