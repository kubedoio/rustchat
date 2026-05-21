const MM_ID_ALPHABET = 'ybndrfg8ejkmcpqxot1uwisza345h769'
const MM_ID_RE = /^[a-z0-9]{26}$/i
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

export function decodeMattermostId(id: string): string | null {
  if (!MM_ID_RE.test(id)) {
    return null
  }

  let buffer = 0
  let bits = 0
  const bytes: number[] = []
  const lower = id.toLowerCase()

  for (let i = 0; i < lower.length; i++) {
    const value = MM_ID_ALPHABET.indexOf(lower[i]!)
    if (value < 0) {
      return null
    }
    buffer = (buffer << 5) | value
    bits += 5

    while (bits >= 8) {
      bytes.push((buffer >> (bits - 8)) & 0xff)
      bits -= 8
    }
  }

  if (bytes.length < 16) {
    return null
  }

  const hex = bytes
    .slice(0, 16)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('')
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`
}

export function normalizeEntityId(value: unknown): string | undefined {
  if (typeof value !== 'string' || value.length === 0) {
    return undefined
  }

  if (UUID_RE.test(value)) {
    return value.toLowerCase()
  }

  const decoded = decodeMattermostId(value)
  return decoded ?? value
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Object.prototype.toString.call(value) === '[object Object]'
}

function shouldNormalizeSingleIdKey(key: string): boolean {
  return key === 'id' || key.endsWith('_id')
}

function shouldNormalizeIdArrayKey(key: string): boolean {
  return key.endsWith('_ids')
}

export function normalizeIdsDeep<T>(value: T): T {
  if (Array.isArray(value)) {
    return value.map(item => normalizeIdsDeep(item)) as T
  }

  if (!isPlainObject(value)) {
    return value
  }

  const next: Record<string, unknown> = {}
  for (const [key, rawVal] of Object.entries(value)) {
    if (typeof rawVal === 'string' && shouldNormalizeSingleIdKey(key)) {
      next[key] = normalizeEntityId(rawVal) ?? rawVal
      continue
    }

    if (Array.isArray(rawVal) && shouldNormalizeIdArrayKey(key)) {
      next[key] = rawVal.map(item =>
        typeof item === 'string' ? (normalizeEntityId(item) ?? item) : item
      )
      continue
    }

    next[key] = normalizeIdsDeep(rawVal)
  }

  return next as T
}

export function shouldNormalizeHttpPayload(value: unknown): boolean {
  if (value == null) {
    return false
  }

  if (typeof FormData !== 'undefined' && value instanceof FormData) {
    return false
  }
  if (typeof URLSearchParams !== 'undefined' && value instanceof URLSearchParams) {
    return false
  }
  if (typeof Blob !== 'undefined' && value instanceof Blob) {
    return false
  }

  return isPlainObject(value) || Array.isArray(value)
}
