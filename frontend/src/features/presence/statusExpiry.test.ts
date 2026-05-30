import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { clearStatusExpiryTimer, parseStatusExpiryMs, scheduleStatusExpiry } from './statusExpiry'

describe('statusExpiry', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-03-29T12:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('parses unix seconds, unix milliseconds, and ISO timestamps', () => {
    expect(parseStatusExpiryMs(1_775_185_200)).toBe(1_775_185_200_000)
    expect(parseStatusExpiryMs(1_775_185_200_123)).toBe(1_775_185_200_123)
    expect(parseStatusExpiryMs('2026-03-29T12:05:00Z')).toBe(Date.parse('2026-03-29T12:05:00Z'))
  })

  it('replaces an existing timer and expires once at the latest deadline', () => {
    const timers = new Map<string, ReturnType<typeof setTimeout>>()
    const onExpire = vi.fn()

    scheduleStatusExpiry(timers, 'user-1', '2026-03-29T12:01:00Z', onExpire)
    scheduleStatusExpiry(timers, 'user-1', '2026-03-29T12:02:00Z', onExpire)

    vi.advanceTimersByTime(60_000)
    expect(onExpire).not.toHaveBeenCalled()

    vi.advanceTimersByTime(60_000)
    expect(onExpire).toHaveBeenCalledTimes(1)
    expect(timers.has('user-1')).toBe(false)
  })

  it('expires immediately when the timestamp is already in the past', () => {
    const timers = new Map<string, ReturnType<typeof setTimeout>>()
    const onExpire = vi.fn()

    const expiresAt = scheduleStatusExpiry(timers, 'user-1', '2026-03-29T11:59:59Z', onExpire)

    expect(expiresAt).toBe(Date.parse('2026-03-29T11:59:59Z'))
    expect(onExpire).toHaveBeenCalledTimes(1)
    expect(timers.size).toBe(0)
  })

  it('clears a scheduled timer by key', () => {
    const timers = new Map<string, ReturnType<typeof setTimeout>>()
    const onExpire = vi.fn()

    scheduleStatusExpiry(timers, 'user-1', '2026-03-29T12:01:00Z', onExpire)
    clearStatusExpiryTimer(timers, 'user-1')

    vi.advanceTimersByTime(60_000)
    expect(onExpire).not.toHaveBeenCalled()
    expect(timers.size).toBe(0)
  })
})
