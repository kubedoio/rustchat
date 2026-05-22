export function parseStatusExpiryMs(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value > 1_000_000_000_000 ? value : value * 1000
  }

  if (typeof value === 'string' && value.length > 0) {
    const numeric = Number(value)
    if (Number.isFinite(numeric) && numeric > 0) {
      return numeric > 1_000_000_000_000 ? numeric : numeric * 1000
    }

    const parsed = Date.parse(value)
    if (Number.isFinite(parsed)) {
      return parsed
    }
  }

  return null
}

export function scheduleStatusExpiry(
  timers: Map<string, ReturnType<typeof setTimeout>>,
  key: string,
  expiresAt: unknown,
  onExpire: () => void
) {
  const existing = timers.get(key)
  if (existing) {
    clearTimeout(existing)
    timers.delete(key)
  }

  const expiresAtMs = parseStatusExpiryMs(expiresAt)
  if (!expiresAtMs) {
    return null
  }

  const remainingMs = expiresAtMs - Date.now()
  if (remainingMs <= 0) {
    onExpire()
    return expiresAtMs
  }

  timers.set(
    key,
    setTimeout(() => {
      onExpire()
      timers.delete(key)
    }, remainingMs)
  )

  return expiresAtMs
}

export function clearStatusExpiryTimer(
  timers: Map<string, ReturnType<typeof setTimeout>>,
  key: string
) {
  const timer = timers.get(key)
  if (!timer) {
    return
  }

  clearTimeout(timer)
  timers.delete(key)
}
