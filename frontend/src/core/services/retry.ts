import { AppError } from '../errors/AppError'

export interface RetryOptions {
  maxAttempts?: number
  initialDelay?: number
  maxDelay?: number
  backoffMultiplier?: number
  shouldRetry?: (error: AppError) => boolean
}

const DEFAULT_OPTIONS: Required<RetryOptions> = {
  maxAttempts: 3,
  initialDelay: 1000,
  maxDelay: 10000,
  backoffMultiplier: 2,
  shouldRetry: error => error.recoverable,
}

export async function withRetry<T>(
  operation: () => Promise<T>,
  options: RetryOptions = {}
): Promise<T> {
  const opts = { ...DEFAULT_OPTIONS, ...options }
  let lastError: AppError
  let delay = opts.initialDelay

  for (let attempt = 1; attempt <= opts.maxAttempts; attempt++) {
    try {
      return await operation()
    } catch (error) {
      lastError = error instanceof AppError ? error : new AppError(String(error))

      // Don't retry on last attempt
      if (attempt === opts.maxAttempts) {
        break
      }

      // Check if we should retry this error
      if (!opts.shouldRetry(lastError)) {
        throw lastError
      }

      // Wait before retrying
      await sleep(delay)

      // Exponential backoff
      delay = Math.min(delay * opts.backoffMultiplier, opts.maxDelay)
    }
  }

  throw lastError!
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms))
}

// Decorator for retry logic
export function retryable(options: RetryOptions = {}) {
  return function (_target: any, _propertyKey: string, descriptor: PropertyDescriptor) {
    const originalMethod = descriptor.value

    descriptor.value = async function (...args: any[]) {
      return withRetry(() => originalMethod.apply(this, args), options)
    }

    return descriptor
  }
}
