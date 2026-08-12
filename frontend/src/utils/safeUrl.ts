const SAFE_HTTP_PROTOCOLS = new Set(['http:', 'https:'])

export function normalizeSafeHttpUrl(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null
  }

  const trimmed = value.trim()
  if (!trimmed) {
    return null
  }

  try {
    const base = typeof window !== 'undefined' ? window.location.origin : 'https://localhost'
    const url = new URL(trimmed, base)
    if (!SAFE_HTTP_PROTOCOLS.has(url.protocol)) {
      return null
    }
    return url.toString()
  } catch {
    return null
  }
}
