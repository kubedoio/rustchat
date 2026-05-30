// Error hierarchy for different error types

export type ErrorCode =
  | 'NETWORK_ERROR'
  | 'NOT_FOUND'
  | 'VALIDATION_ERROR'
  | 'AUTH_REQUIRED'
  | 'CALLS_NOT_AVAILABLE'
  | 'NO_ACTIVE_CALL'
  | 'CALL_STATE_ERROR'
  | 'SESSION_ERROR'
  | 'UNKNOWN_ERROR'

export class AppError extends Error {
  readonly code: ErrorCode
  readonly recoverable: boolean
  readonly isRetryable: boolean

  constructor(message: string, code: ErrorCode = 'UNKNOWN_ERROR', recoverable: boolean = false) {
    super(message)
    this.name = 'AppError'
    this.code = code
    this.recoverable = recoverable
    this.isRetryable = recoverable
  }
}

export class NetworkError extends AppError {
  constructor(message: string = 'Network error') {
    super(message, 'NETWORK_ERROR', true)
  }
}

export class NotFoundError extends AppError {
  constructor(resource: string, id: string) {
    super(`${resource} not found: ${id}`, 'NOT_FOUND', false)
  }
}

export class ValidationError extends AppError {
  constructor(message: string) {
    super(message, 'VALIDATION_ERROR', false)
  }
}
