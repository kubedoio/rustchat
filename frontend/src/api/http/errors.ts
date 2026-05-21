/**
 * HTTP transport errors
 * Extends AppError to integrate with existing error handling and retry logic
 */

import { AppError } from '@/core/errors/AppError'

/**
 * Thrown when a request times out
 * Marked as recoverable so withRetry() will attempt retry
 */
export class TimeoutError extends AppError {
  constructor(message: string = 'Request timed out') {
    super(message, 'NETWORK_ERROR', true)
    this.name = 'TimeoutError'
  }
}

/**
 * Thrown when a request is aborted (e.g., user navigates away)
 * Not recoverable - user intentionally cancelled
 */
export class AbortError extends AppError {
  constructor(message: string = 'Request aborted') {
    super(message, 'NETWORK_ERROR', false)
    this.name = 'AbortError'
  }
}

/**
 * Thrown for HTTP errors (4xx, 5xx)
 * Contains response status and data for error handling
 */
export class HttpError extends AppError {
  readonly status: number
  readonly data: unknown

  constructor(status: number, data: unknown, message?: string) {
    super(
      message || `HTTP ${status} error`,
      status === 401 ? 'AUTH_REQUIRED' : 'NETWORK_ERROR',
      status >= 500 || status === 429 // retryable: server errors and rate limits
    )
    this.name = 'HttpError'
    this.status = status
    this.data = data
  }
}

/**
 * Check if an error is a timeout error
 */
export function isTimeoutError(error: unknown): error is TimeoutError {
  return error instanceof TimeoutError
}

/**
 * Check if an error is an abort error
 */
export function isAbortError(error: unknown): error is AbortError {
  return error instanceof AbortError
}

/**
 * Check if an error is an HTTP error
 */
export function isHttpError(error: unknown): error is HttpError {
  return error instanceof HttpError
}
