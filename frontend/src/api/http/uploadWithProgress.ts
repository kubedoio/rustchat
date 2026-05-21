/**
 * File upload with progress tracking
 * Uses XMLHttpRequest for progress events (Fetch doesn't support upload progress)
 */

import { AbortError, HttpError } from './errors'
import { NetworkError } from '@/core/errors/AppError'
import type { HttpResponse } from './HttpClient'

export interface UploadProgress {
  loaded: number
  total: number
  percentage?: number
}

export interface UploadConfig {
  headers?: Record<string, string>
  signal?: AbortSignal
  onUploadProgress?: (progress: UploadProgress) => void
}

/**
 * Upload file with progress tracking using XHR
 * Falls back to XHR because Fetch doesn't support upload progress
 */
export function uploadWithProgress<T>(
  url: string,
  formData: FormData,
  config: UploadConfig = {}
): Promise<HttpResponse<T>> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()

    // Set up progress tracking
    if (config.onUploadProgress) {
      xhr.upload.addEventListener('progress', event => {
        if (event.lengthComputable) {
          config.onUploadProgress!({
            loaded: event.loaded,
            total: event.total,
            percentage: Math.round((event.loaded / event.total) * 100),
          })
        }
      })
    }

    // Handle completion
    xhr.addEventListener('load', () => {
      const headers = parseHeaders(xhr.getAllResponseHeaders())

      if (xhr.status >= 200 && xhr.status < 300) {
        let data: T
        try {
          data = xhr.responseText ? JSON.parse(xhr.responseText) : (null as T)
        } catch {
          data = xhr.responseText as unknown as T
        }

        resolve({
          data,
          status: xhr.status,
          statusText: xhr.statusText,
          headers,
        })
      } else {
        let errorData: unknown
        try {
          errorData = xhr.responseText ? JSON.parse(xhr.responseText) : null
        } catch {
          errorData = xhr.responseText
        }
        reject(new HttpError(xhr.status, errorData, `HTTP ${xhr.status} error`))
      }
    })

    // Handle network errors
    xhr.addEventListener('error', () => {
      reject(new NetworkError('Network error during upload'))
    })

    // Handle abort
    xhr.addEventListener('abort', () => {
      reject(new AbortError('Upload aborted'))
    })

    // Handle timeout
    xhr.addEventListener('timeout', () => {
      reject(new NetworkError('Upload timed out'))
    })

    // Set up abort signal
    if (config.signal) {
      config.signal.addEventListener('abort', () => {
        xhr.abort()
      })
    }

    // Open and configure request
    xhr.open('POST', url)

    // Set headers
    if (config.headers) {
      for (const [key, value] of Object.entries(config.headers)) {
        xhr.setRequestHeader(key, value)
      }
    }

    // Send request
    xhr.send(formData)
  })
}

/**
 * Parse XHR response headers into Headers object
 */
function parseHeaders(headerStr: string): Headers {
  const headers = new Headers()
  if (!headerStr) return headers

  const lines = headerStr.trim().split(/[\r\n]+/)
  for (const line of lines) {
    const parts = line.split(': ')
    if (parts.length === 2) {
      headers.set(parts[0]!, parts[1]!)
    }
  }

  return headers
}
