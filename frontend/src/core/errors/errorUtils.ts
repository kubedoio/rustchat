/**
 * Error utility functions for safely extracting information from unknown errors.
 * Use these instead of casting catch variables to `any`.
 */

/**
 * Safely extract an API error message from an unknown error.
 * Checks for Axios-style `response.data.message` and nested `response.data.error.message`.
 */
export function getApiErrorMessage(error: unknown): string | undefined {
  if (!error || typeof error !== 'object') return undefined

  const e = error as Record<string, unknown>
  const response = e.response as Record<string, unknown> | undefined
  const data = response?.data as Record<string, unknown> | undefined

  if (typeof data?.message === 'string') return data.message

  if (typeof data?.error === 'string') return data.error

  if (typeof data?.error === 'object' && data.error !== null) {
    const errObj = data.error as Record<string, unknown>
    if (typeof errObj.message === 'string') return errObj.message
  }

  if (typeof e.message === 'string') return e.message

  return undefined
}

/**
 * Safely extract the HTTP status code from an unknown error.
 */
export function getErrorStatus(error: unknown): number | undefined {
  if (!error || typeof error !== 'object') return undefined

  const e = error as Record<string, unknown>
  const response = e.response as Record<string, unknown> | undefined

  if (typeof response?.status === 'number') return response.status

  return undefined
}

/**
 * Check if an unknown error is a 404 Not Found.
 */
export function isNotFoundError(error: unknown): boolean {
  return getErrorStatus(error) === 404
}

/**
 * Safely get a string message from any unknown value.
 * Falls back to a default message if nothing useful can be extracted.
 */
export function getErrorMessage(error: unknown, fallback = 'Unknown error'): string {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  return fallback
}
