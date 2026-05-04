# SPEC: WebSocket Disconnection UX (2026-04-08)

## Problem Statement

When users leave the RustChat window open for extended periods, the WebSocket connection can be lost due to:
- Network interruptions (WiFi disconnect, laptop sleep/wake)
- Server restarts or deployments
- Browser throttling background tabs
- Proxy/firewall timeouts

Previously, the UI continued to display potentially stale content without clear indication, leading to:
- Users acting on outdated information (deleted/edited messages not reflected)
- Failed message sends with no clear feedback
- Confusion about why new messages weren't appearing

## Goals

1. **Transparency**: User always knows the connection state
2. **Progressive urgency**: Visual severity increases with disconnection time
3. **Clear actions**: User knows exactly what to do to reconnect
4. **Safety**: Prevent actions on potentially stale data

## Non-Goals

1. No automatic refresh or data clearing without user consent
2. No queueing of messages typed during disconnection (future enhancement)
3. No mobile-specific handling (uses same WebSocket infrastructure)

## User Experience States

### State 1: Reconnecting (< 5 seconds)

**Trigger:** WebSocket closes, immediate auto-reconnect initiated

**Visual Design:**
- Yellow/amber banner at top: "Reconnecting..."
- Content opacity: 80%
- Header indicator: 🟡 Yellow pulsing dot
- Composer: Disabled with tooltip "Reconnecting..."

**Behavior:**
- Exponential backoff retry (1s, 1.5s, 2.25s, ... max 10s)
- User can read existing content
- No action required from user

### State 2: Disconnected (5-30 seconds)

**Trigger:** Reconnection attempts continue to fail

**Visual Design:**
- Orange banner at top: "Connection lost. Retrying in Xs..."
- Countdown timer showing next attempt
- Content opacity: 60%
- Header indicator: 🟠 Orange pulsing dot
- Composer: Disabled with tooltip

**Behavior:**
- "Retry now" button for immediate reconnect attempt
- Continued automatic retry every 10 seconds
- Content still visible but clearly marked as potentially stale

### State 3: Failed (> 30 seconds or max retries)

**Trigger:** 30 seconds elapsed or 10 reconnection attempts exhausted

**Visual Design:**
- Full-screen modal overlay with backdrop blur
- Title: "🔌 Disconnected"
- Message: "You've been disconnected from the server. Your conversation may be out of date."
- Background: Previous content visible but heavily blurred

**Actions:**
- **Reconnect**: Attempts WebSocket reconnection, syncs missed messages
- **Refresh page**: Full page reload

**Behavior:**
- All interactive elements disabled
- User must take explicit action to continue
- On reconnect: automatic sync of missed messages

---

## Technical Architecture

### State Machine

```
CONNECTED 
    ↓ (onClose)
RECONNECTING (< 5s)
    ↓ (timeout 5s)
DISCONNECTED (5-30s)
    ↓ (timeout 30s or max retries)
FAILED (> 30s)
    ↓ (user action)
RECONNECTING → CONNECTED
```

### Frontend Implementation

**New Reactive State** (`useWebSocket` composable):
```typescript
interface ConnectionState {
  status: 'connected' | 'reconnecting' | 'disconnected' | 'failed'
  disconnectedAt: Date | null
  reconnectAttempt: number
  nextRetryIn: number  // countdown seconds
}
```

**State Transition Logic:**
```typescript
function updateConnectionStatus() {
  if (connected.value) {
    status.value = 'connected'
    return
  }
  
  const elapsed = Date.now() - disconnectedAt.value.getTime()
  
  if (elapsed >= 30000 || attempts >= 10) {
    status.value = 'failed'
  } else if (elapsed >= 5000) {
    status.value = 'disconnected'
  } else {
    status.value = 'reconnecting'
  }
}
```

**Retry Configuration:**
```typescript
const MAX_RECONNECT_ATTEMPTS = 10
const RECONNECT_DELAY_BASE = 1000  // 1 second
const RECONNECT_DELAY_MAX = 10000  // 10 seconds
const BACKOFF_MULTIPLIER = 1.5
```

### Components

| Component | Location | Purpose |
|-----------|----------|---------|
| `ConnectionStatusBar` | `frontend/src/components/ui/` | Banner for States 1 & 2 |
| `ConnectionLostModal` | `frontend/src/components/ui/` | Full-screen modal for State 3 |
| `GlobalHeader` (modified) | `frontend/src/components/layout/` | Status dot indicator |
| `MessageComposer` (modified) | `frontend/src/components/composer/` | Disable send on disconnect |
| `App.vue` (modified) | `frontend/src/` | Global integration |

### Content Dimming

Applied via CSS classes on the main content container:
```css
.opacity-80 { opacity: 0.8; }  /* reconnecting */
.opacity-60 { opacity: 0.6; }  /* disconnected */
```

---

## API & Contracts

### WebSocket Events (No Change)

The WebSocket protocol remains unchanged. This feature is purely client-side UX.

| Event | Direction | Purpose |
|-------|-----------|---------|
| `ping` | Client → Server | Keepalive (30s interval) |
| `pong` | Server → Client | Keepalive response |
| `reconnect` | Client → Server | Request state sync after reconnect |
| `initial_load` | Server → Client | Full state snapshot after reconnect |

### WebSocket Connection States

| State | Description | User Impact |
|-------|-------------|-------------|
| `CONNECTING` | Initial connection attempt | Loading spinner |
| `OPEN` | Connected and operational | Normal operation |
| `CLOSING` | Graceful shutdown | Brief, transitions quickly |
| `CLOSED` | Connection closed | Triggers reconnection logic |

---

## Testing Strategy

### E2E Tests (Playwright)

**Test Case 1: Brief Disconnection**
```typescript
test('shows reconnecting banner and restores connection', async ({ page }) => {
  await page.goto('/channels/general')
  await expect(page.locator('[data-testid="connection-status"]')).toHaveText('connected')
  
  // Simulate disconnect
  await page.evaluate(() => window.simulateDisconnect())
  await expect(page.locator('.connection-banner')).toContainText('Reconnecting')
  await expect(page.locator('[data-testid="content-wrapper"]')).toHaveClass(/opacity-80/)
  
  // Restore connection
  await page.evaluate(() => window.simulateReconnect())
  await expect(page.locator('.connection-banner')).not.toBeVisible()
})
```

**Test Case 2: Extended Disconnection**
```typescript
test('shows countdown and retry button', async ({ page }) => {
  await page.goto('/channels/general')
  await page.evaluate(() => window.simulateDisconnect())
  
  // Wait for state transition
  await page.waitForTimeout(5000)
  await expect(page.locator('.connection-banner')).toContainText('Retrying in')
  await expect(page.locator('[data-testid="retry-button"]')).toBeVisible()
  
  // Click retry
  await page.click('[data-testid="retry-button"]')
  await expect(page.locator('.connection-banner')).toContainText('Reconnecting')
})
```

**Test Case 3: Critical Disconnection**
```typescript
test('shows full-screen modal after timeout', async ({ page }) => {
  await page.goto('/channels/general')
  await page.evaluate(() => window.simulateDisconnect())
  
  await page.waitForTimeout(30000)
  await expect(page.locator('[data-testid="connection-lost-modal"]')).toBeVisible()
  await expect(page.locator('[data-testid="content-wrapper"]')).toHaveClass(/blur/)
  
  // Click reconnect
  await page.click('[data-testid="reconnect-button"]')
  await expect(page.locator('[data-testid="connection-lost-modal"]')).not.toBeVisible()
})
```

**Test Case 4: Composer Disabled**
```typescript
test('disables composer during disconnect', async ({ page }) => {
  await page.goto('/channels/general')
  await page.fill('[data-testid="message-input"]', 'Hello')
  
  await page.evaluate(() => window.simulateDisconnect())
  await expect(page.locator('[data-testid="send-button"]')).toBeDisabled()
  await expect(page.locator('[data-testid="send-button"]')).toHaveAttribute('title', 'Reconnecting...')
})
```

### Manual Testing Checklist

- [ ] Brief disconnect (< 5s): Yellow banner, 80% opacity, auto-reconnect
- [ ] Extended disconnect (5-30s): Orange banner, countdown, retry button
- [ ] Critical disconnect (> 30s): Full-screen modal, blurred background
- [ ] Composer: Disabled with tooltip on all disconnect states
- [ ] Header indicator: Color changes with state
- [ ] Reconnect: Syncs missed messages automatically
- [ ] Refresh: Full page reload works

---

## Accessibility

### Screen Readers

- Banner: `role="status"`, `aria-live="polite"`
- Modal: `role="dialog"`, `aria-modal="true"`, focus trap
- Status dot: `aria-label="Connection: {state}"`

### Keyboard Navigation

- Banner: No interaction (informational only)
- Modal: Tab cycles between Reconnect/Refresh buttons
- Escape: Does not dismiss (user must take action)

### Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  .animate-pulse { animation: none; }
  .transition-opacity { transition: none; }
}
```

---

## Metrics & Monitoring

### Client-Side Events (Future)

```typescript
// Analytics events
track('websocket_disconnected', { duration, attempts })
track('websocket_reconnected', { method: 'auto' | 'manual' | 'refresh' })
track('websocket_failed', { max_attempts_reached: boolean })
```

### Health Indicators

- Average reconnection time
- Percentage of users reaching "failed" state
- Message send failure rate during disconnections

---

## Future Enhancements

1. **Message Queueing**: Queue messages typed during disconnection, send on reconnect
2. **Smart Reconnect**: Detect network type changes (WiFi → cellular)
3. **Offline Mode**: Allow reading cached content when explicitly offline
4. **Notification on Disconnect**: Browser notification if disconnected while in background

---

## Implementation Status

| Component | Status | Commit |
|-----------|--------|--------|
| useWebSocket state tracking | ✅ Complete | `feat(websocket): add connection state tracking` |
| ConnectionStatusBar | ✅ Complete | `feat(ui): add ConnectionStatusBar component` |
| ConnectionLostModal | ✅ Complete | `feat(ui): add ConnectionLostModal component` |
| MessageComposer integration | ✅ Complete | `feat(composer): disable sending when disconnected` |
| App.vue integration | ✅ Complete | `feat(app): integrate connection status components` |
| GlobalHeader indicator | ✅ Complete | `feat(header): add connection status indicator dot` |
| Documentation | ✅ Complete | This spec |
| E2E Tests | ⏳ Pending | See test cases above |

---

## References

- Design doc: `docs/plans/2026-04-08-websocket-disconnection-ux-design.md`
- Implementation plan: `docs/plans/2026-04-08-websocket-disconnection-ux-implementation.md`
- Architecture: `docs/architecture/architecture-overview.md` (WebSocket Hub section)
- Testing model: `docs/development/testing.md`
