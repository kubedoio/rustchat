/**
 * Query parameter serialization
 * Matches Axios behavior for query string formatting
 */

/**
 * Serialize query parameters to URL-encoded string
 * Matches Axios behavior:
 * - Arrays → repeated keys (tags=a&tags=b)
 * - Objects → JSON stringified
 * - Null/undefined → omitted
 * - Dates → ISO string
 * - Booleans → "true"/"false"
 */
export function serializeQueryParams(params: Record<string, unknown>): string {
  const pairs: string[] = []

  for (const [key, value] of Object.entries(params)) {
    // Skip null and undefined
    if (value === null || value === undefined) {
      continue
    }

    const encodedKey = encodeURIComponent(key)

    if (Array.isArray(value)) {
      // Arrays → repeated keys
      for (const item of value) {
        if (item !== null && item !== undefined) {
          pairs.push(`${encodedKey}=${encodeURIComponent(String(item))}`)
        }
      }
    } else if (value instanceof Date) {
      // Dates → ISO string
      pairs.push(`${encodedKey}=${encodeURIComponent(value.toISOString())}`)
    } else if (typeof value === 'object') {
      // Objects → JSON stringified
      pairs.push(`${encodedKey}=${encodeURIComponent(JSON.stringify(value))}`)
    } else if (typeof value === 'boolean') {
      // Booleans → "true"/"false"
      pairs.push(`${encodedKey}=${encodeURIComponent(String(value))}`)
    } else {
      // Primitives
      pairs.push(`${encodedKey}=${encodeURIComponent(String(value))}`)
    }
  }

  return pairs.join('&')
}

/**
 * Build full URL with base, path, and query string
 */
export function buildURL(
  baseURL: string | undefined,
  url: string,
  params?: Record<string, unknown>
): string {
  // Join baseURL and url
  let fullURL: string
  if (baseURL) {
    // Remove trailing slash from base, remove leading slash from url
    const base = baseURL.replace(/\/$/, '')
    const path = url.replace(/^\//, '')
    fullURL = `${base}/${path}`
  } else {
    fullURL = url
  }

  // Add query string if params provided
  if (params && Object.keys(params).length > 0) {
    const queryString = serializeQueryParams(params)
    if (queryString) {
      const separator = fullURL.includes('?') ? '&' : '?'
      fullURL = `${fullURL}${separator}${queryString}`
    }
  }

  return fullURL
}
