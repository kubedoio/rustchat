// Rust-like Result type for explicit error handling

export type Result<T, E = Error> = { ok: true; value: T } | { ok: false; error: E }

export interface AsyncResult<T, E = Error> {
  data: T | null
  error: E | null
  isLoading: boolean
  isSuccess: boolean
  isError: boolean
}

export function success<T>(value: T): Result<T, never> {
  return { ok: true, value }
}

export function failure<E>(error: E): Result<never, E> {
  return { ok: false, error }
}

export function isSuccess<T, E>(result: Result<T, E>): result is { ok: true; value: T } {
  return result.ok === true
}

export function isFailure<T, E>(result: Result<T, E>): result is { ok: false; error: E } {
  return result.ok === false
}
