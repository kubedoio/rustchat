import { describe, it, expect } from 'vitest'
import {
  TimeoutError,
  AbortError,
  HttpError,
  isTimeoutError,
  isAbortError,
  isHttpError,
} from '../errors'
import { AppError } from '@/core/errors/AppError'

describe('HTTP Errors', () => {
  describe('TimeoutError', () => {
    it('extends AppError', () => {
      const error = new TimeoutError()
      expect(error).toBeInstanceOf(AppError)
      expect(error).toBeInstanceOf(Error)
    })

    it('has correct error code', () => {
      const error = new TimeoutError()
      expect(error.code).toBe('NETWORK_ERROR')
    })

    it('is recoverable', () => {
      const error = new TimeoutError()
      expect(error.recoverable).toBe(true)
      expect(error.isRetryable).toBe(true)
    })

    it('has correct name', () => {
      const error = new TimeoutError()
      expect(error.name).toBe('TimeoutError')
    })

    it('uses default message', () => {
      const error = new TimeoutError()
      expect(error.message).toBe('Request timed out')
    })

    it('accepts custom message', () => {
      const error = new TimeoutError('Custom timeout')
      expect(error.message).toBe('Custom timeout')
    })
  })

  describe('AbortError', () => {
    it('extends AppError', () => {
      const error = new AbortError()
      expect(error).toBeInstanceOf(AppError)
      expect(error).toBeInstanceOf(Error)
    })

    it('has correct error code', () => {
      const error = new AbortError()
      expect(error.code).toBe('NETWORK_ERROR')
    })

    it('is not recoverable', () => {
      const error = new AbortError()
      expect(error.recoverable).toBe(false)
      expect(error.isRetryable).toBe(false)
    })

    it('has correct name', () => {
      const error = new AbortError()
      expect(error.name).toBe('AbortError')
    })

    it('uses default message', () => {
      const error = new AbortError()
      expect(error.message).toBe('Request aborted')
    })

    it('accepts custom message', () => {
      const error = new AbortError('User cancelled')
      expect(error.message).toBe('User cancelled')
    })
  })

  describe('HttpError', () => {
    it('extends AppError', () => {
      const error = new HttpError(400, { message: 'Bad Request' })
      expect(error).toBeInstanceOf(AppError)
      expect(error).toBeInstanceOf(Error)
    })

    it('stores status code', () => {
      const error = new HttpError(404, null)
      expect(error.status).toBe(404)
    })

    it('stores response data', () => {
      const data = { error: 'Not found' }
      const error = new HttpError(404, data)
      expect(error.data).toEqual(data)
    })

    it('uses AUTH_REQUIRED code for 401', () => {
      const error = new HttpError(401, null)
      expect(error.code).toBe('AUTH_REQUIRED')
    })

    it('uses NETWORK_ERROR code for other errors', () => {
      const error400 = new HttpError(400, null)
      const error500 = new HttpError(500, null)
      expect(error400.code).toBe('NETWORK_ERROR')
      expect(error500.code).toBe('NETWORK_ERROR')
    })

    it('marks server errors (5xx) as recoverable', () => {
      const error500 = new HttpError(500, null)
      const error502 = new HttpError(502, null)
      expect(error500.recoverable).toBe(true)
      expect(error502.recoverable).toBe(true)
    })

    it('marks rate limit (429) as recoverable', () => {
      const error = new HttpError(429, null)
      expect(error.recoverable).toBe(true)
    })

    it('marks client errors (4xx) as not recoverable', () => {
      const error400 = new HttpError(400, null)
      const error404 = new HttpError(404, null)
      expect(error400.recoverable).toBe(false)
      expect(error404.recoverable).toBe(false)
    })

    it('uses custom message if provided', () => {
      const error = new HttpError(500, null, 'Server exploded')
      expect(error.message).toBe('Server exploded')
    })

    it('uses default message for HTTP errors', () => {
      const error = new HttpError(500, null)
      expect(error.message).toBe('HTTP 500 error')
    })

    it('has correct name', () => {
      const error = new HttpError(400, null)
      expect(error.name).toBe('HttpError')
    })
  })

  describe('type guards', () => {
    it('isTimeoutError returns true for TimeoutError', () => {
      expect(isTimeoutError(new TimeoutError())).toBe(true)
      expect(isTimeoutError(new Error())).toBe(false)
      expect(isTimeoutError(null)).toBe(false)
    })

    it('isAbortError returns true for AbortError', () => {
      expect(isAbortError(new AbortError())).toBe(true)
      expect(isAbortError(new Error())).toBe(false)
      expect(isAbortError(null)).toBe(false)
    })

    it('isHttpError returns true for HttpError', () => {
      expect(isHttpError(new HttpError(400, null))).toBe(true)
      expect(isHttpError(new Error())).toBe(false)
      expect(isHttpError(null)).toBe(false)
    })
  })
})
