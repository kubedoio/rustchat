import { test, expect } from '@playwright/test'

/**
 * WebSocket Disconnection UX E2E Tests
 * 
 * Tests the progressive disconnection UI states:
 * 1. Reconnecting (< 5s): Yellow banner, auto-retry
 * 2. Disconnected (5-30s): Orange banner, countdown, manual retry
 * 3. Failed (> 30s): Full-screen modal, reconnect/refresh actions
 */

test.describe('WebSocket Connection States', () => {
  test.beforeEach(async ({ page }) => {
    // Login and navigate to a channel
    await page.goto('/login')
    await page.fill('[data-testid="login-username"]', 'testuser')
    await page.fill('[data-testid="login-password"]', 'testpass')
    await page.click('[data-testid="login-submit"]')
    await page.waitForURL('**/channels/**')
    
    // Wait for WebSocket to connect
    await page.waitForSelector('[data-testid="connection-indicator"][data-status="connected"]')
  })

  test('shows reconnecting banner on brief disconnect', async ({ page }) => {
    // Simulate WebSocket disconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    // Should show yellow banner
    const banner = page.locator('[data-testid="connection-status-bar"]')
    await expect(banner).toBeVisible()
    await expect(banner).toHaveAttribute('data-status', 'reconnecting')
    await expect(banner).toContainText('Reconnecting')

    // Content should be dimmed
    const content = page.locator('[data-testid="main-content"]')
    await expect(content).toHaveClass(/opacity-80/)

    // Header indicator should be yellow
    const indicator = page.locator('[data-testid="connection-indicator"]')
    await expect(indicator).toHaveClass(/bg-amber-500/)

    // Composer should be disabled
    const sendButton = page.locator('[data-testid="send-button"]')
    await expect(sendButton).toBeDisabled()
    await expect(sendButton).toHaveAttribute('title', 'Reconnecting...')

    // Restore connection
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketOpen()
    })

    // Should return to normal
    await expect(banner).not.toBeVisible()
    await expect(content).not.toHaveClass(/opacity-80/)
    await expect(indicator).toHaveClass(/bg-green-500/)
  })

  test('shows disconnected state with countdown', async ({ page }) => {
    // Simulate disconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    // Wait for state transition (5 seconds)
    await page.waitForTimeout(5500)

    // Should show orange banner with countdown
    const banner = page.locator('[data-testid="connection-status-bar"]')
    await expect(banner).toHaveAttribute('data-status', 'disconnected')
    await expect(banner).toContainText('Connection lost')
    await expect(banner).toContainText('Retrying in')

    // Retry button should be visible
    const retryButton = page.locator('[data-testid="retry-connection-button"]')
    await expect(retryButton).toBeVisible()

    // Content more dimmed
    const content = page.locator('[data-testid="main-content"]')
    await expect(content).toHaveClass(/opacity-60/)

    // Header indicator should be orange
    const indicator = page.locator('[data-testid="connection-indicator"]')
    await expect(indicator).toHaveClass(/bg-orange-500/)
  })

  test('manual retry button triggers immediate reconnect', async ({ page }) => {
    // Simulate disconnect and wait for extended state
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })
    await page.waitForTimeout(5500)

    // Click retry button
    await page.click('[data-testid="retry-connection-button"]')

    // Should immediately show reconnecting
    const banner = page.locator('[data-testid="connection-status-bar"]')
    await expect(banner).toHaveAttribute('data-status', 'reconnecting')
    await expect(banner).toContainText('Reconnecting')
  })

  test('shows critical disconnection modal after timeout', async ({ page }) => {
    // Simulate disconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    // Wait for critical state (30 seconds)
    await page.waitForTimeout(30500)

    // Modal should appear
    const modal = page.locator('[data-testid="connection-lost-modal"]')
    await expect(modal).toBeVisible()
    await expect(modal).toContainText('Disconnected')
    await expect(modal).toContainText('Your conversation may be out of date')

    // Backdrop blur should be applied
    const content = page.locator('[data-testid="main-content"]')
    await expect(content).toHaveClass(/blur/)

    // Reconnect and Refresh buttons should be visible
    await expect(page.locator('[data-testid="modal-reconnect-button"]')).toBeVisible()
    await expect(page.locator('[data-testid="modal-refresh-button"]')).toBeVisible()

    // Header indicator should be red
    const indicator = page.locator('[data-testid="connection-indicator"]')
    await expect(indicator).toHaveClass(/bg-red-500/)
  })

  test('reconnect button in modal restores connection', async ({ page }) => {
    // Simulate disconnect and reach critical state
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })
    await page.waitForTimeout(30500)

    // Click reconnect
    await page.click('[data-testid="modal-reconnect-button"]')

    // Modal should close
    const modal = page.locator('[data-testid="connection-lost-modal"]')
    await expect(modal).not.toBeVisible()

    // Connection should restore
    await page.waitForSelector('[data-testid="connection-indicator"][data-status="connected"]')
    const indicator = page.locator('[data-testid="connection-indicator"]')
    await expect(indicator).toHaveClass(/bg-green-500/)
  })

  test('refresh button in modal reloads page', async ({ page }) => {
    // Simulate disconnect and reach critical state
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })
    await page.waitForTimeout(30500)

    // Set up dialog handler to dismiss any confirmation dialogs
    page.on('dialog', async dialog => {
      await dialog.accept()
    })

    // Click refresh - this will reload the page
    await page.click('[data-testid="modal-refresh-button"]')

    // Should navigate to login or loading state
    await expect(page).toHaveURL(/\/login|loading/)
  })

  test('composer is disabled during all disconnect states', async ({ page }) => {
    // Type a message
    const input = page.locator('[data-testid="message-input"]')
    await input.fill('Test message')

    // Simulate disconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    // Send button should be disabled in reconnecting state
    let sendButton = page.locator('[data-testid="send-button"]')
    await expect(sendButton).toBeDisabled()

    // Wait for disconnected state
    await page.waitForTimeout(5500)
    await expect(sendButton).toBeDisabled()

    // Wait for failed state
    await page.waitForTimeout(25000)
    await expect(sendButton).toBeDisabled()

    // Restore connection
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketOpen()
    })

    // Send button should be enabled
    await expect(sendButton).toBeEnabled()

    // Message should still be in input
    await expect(input).toHaveValue('Test message')
  })

  test('tooltip shows correct disabled reason', async ({ page }) => {
    // Simulate disconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    const sendButton = page.locator('[data-testid="send-button"]')
    await expect(sendButton).toHaveAttribute('title', 'Reconnecting...')
  })

  test('connection indicator updates through all states', async ({ page }) => {
    const indicator = page.locator('[data-testid="connection-indicator"]')

    // Start connected (green)
    await expect(indicator).toHaveAttribute('data-status', 'connected')
    await expect(indicator).toHaveClass(/bg-green-500/)

    // Disconnect (yellow)
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })
    await expect(indicator).toHaveAttribute('data-status', 'reconnecting')
    await expect(indicator).toHaveClass(/bg-amber-500/)

    // Extended (orange)
    await page.waitForTimeout(5500)
    await expect(indicator).toHaveAttribute('data-status', 'disconnected')
    await expect(indicator).toHaveClass(/bg-orange-500/)

    // Failed (red)
    await page.waitForTimeout(25000)
    await expect(indicator).toHaveAttribute('data-status', 'failed')
    await expect(indicator).toHaveClass(/bg-red-500/)
  })
})

test.describe('WebSocket Reconnection Sync', () => {
  test('syncs missed messages after reconnect', async ({ page }) => {
    // Login and load channel
    await page.goto('/login')
    await page.fill('[data-testid="login-username"]', 'testuser')
    await page.fill('[data-testid="login-password"]', 'testpass')
    await page.click('[data-testid="login-submit"]')
    await page.waitForURL('**/channels/**')

    const channelId = await page.evaluate(() => {
      return (window as any).testHelpers.getCurrentChannelId()
    })

    // Disconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    // Send a message from another client (simulated)
    await page.evaluate((channel) => {
      return (window as any).testHelpers.sendMessageAsOtherUser(channel, 'Missed message')
    }, channelId)

    // Reconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketOpen()
    })

    // Wait for sync
    await page.waitForTimeout(1000)

    // Missed message should appear
    await expect(page.locator('.message-content')).toContainText('Missed message')
  })

  test('fetches unread counts after reconnect', async ({ page }) => {
    // Note the current unread count
    const initialBadge = await page.locator('[data-testid="unread-badge"]').textContent()

    // Disconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    // Simulate unread messages arriving while disconnected
    await page.evaluate(() => {
      (window as any).testHelpers.simulateUnreadCounts({ channel_id: 'test', unread_count: 5 })
    })

    // Reconnect
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketOpen()
    })

    // Unread count should update
    await page.waitForTimeout(500)
    const newBadge = await page.locator('[data-testid="unread-badge"]').textContent()
    expect(newBadge).not.toEqual(initialBadge)
  })
})

test.describe('Accessibility', () => {
  test('status bar has correct ARIA attributes', async ({ page }) => {
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })

    const banner = page.locator('[data-testid="connection-status-bar"]')
    await expect(banner).toHaveAttribute('role', 'status')
    await expect(banner).toHaveAttribute('aria-live', 'polite')
  })

  test('modal has correct ARIA attributes', async ({ page }) => {
    await page.evaluate(() => {
      (window as any).testHelpers.simulateWebSocketClose()
    })
    await page.waitForTimeout(30500)

    const modal = page.locator('[data-testid="connection-lost-modal"]')
    await expect(modal).toHaveAttribute('role', 'dialog')
    await expect(modal).toHaveAttribute('aria-modal', 'true')
  })

  test('status indicator has aria-label', async ({ page }) => {
    const indicator = page.locator('[data-testid="connection-indicator"]')
    await expect(indicator).toHaveAttribute('aria-label', /Connection: (connected|reconnecting|disconnected|failed)/)
  })
})
