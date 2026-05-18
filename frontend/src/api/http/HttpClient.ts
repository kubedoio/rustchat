/**
 * HTTP Client - Fetch-based transport layer
 * Replaces Axios with native Fetch API
 */

import { buildURL } from './querySerializer'
import { uploadWithProgress, type UploadConfig } from './uploadWithProgress'
import { HttpError, TimeoutError, AbortError } from './errors'
import { HTTP_DEFAULT_TIMEOUT } from '../../constants'

export interface HttpClientConfig {
    baseURL?: string
    headers?: Record<string, string>
    timeout?: number
    requestInterceptor?: (config: RequestConfig) => RequestConfig
    responseInterceptor?: <T>(response: HttpResponse<T>) => HttpResponse<T>
}

export interface RequestConfig {
    headers?: Record<string, string>
    params?: Record<string, any>
    data?: unknown
    baseURL?: string
    timeout?: number
    signal?: AbortSignal
    onUploadProgress?: (progress: { loaded: number; total: number }) => void
    responseType?: 'json' | 'text' | 'blob' | 'arraybuffer'
}

export interface HttpResponse<T> {
    data: T
    status: number
    statusText: string
    headers: Headers
}

const DEFAULT_TIMEOUT = HTTP_DEFAULT_TIMEOUT

export class HttpClient {
    private baseURL?: string
    private defaultHeaders: Record<string, string>
    private defaultTimeout: number
    private requestInterceptor?: (config: RequestConfig) => RequestConfig
    private responseInterceptor?: <T>(response: HttpResponse<T>) => HttpResponse<T>

    constructor(config: HttpClientConfig = {}) {
        this.baseURL = config.baseURL
        this.defaultHeaders = config.headers || {}
        this.defaultTimeout = config.timeout || DEFAULT_TIMEOUT
        this.requestInterceptor = config.requestInterceptor
        this.responseInterceptor = config.responseInterceptor
    }

    async get<T = any>(url: string, config?: RequestConfig): Promise<HttpResponse<T>> {
        return this.request<T>('GET', url, undefined, config)
    }

    async post<T = any>(url: string, data?: unknown, config?: RequestConfig): Promise<HttpResponse<T>> {
        return this.request<T>('POST', url, data, config)
    }

    async put<T = any>(url: string, data?: unknown, config?: RequestConfig): Promise<HttpResponse<T>> {
        return this.request<T>('PUT', url, data, config)
    }

    async patch<T = any>(url: string, data?: unknown, config?: RequestConfig): Promise<HttpResponse<T>> {
        return this.request<T>('PATCH', url, data, config)
    }

    async delete<T = any>(url: string, config?: RequestConfig): Promise<HttpResponse<T>> {
        return this.request<T>('DELETE', url, undefined, config)
    }

    private async request<T>(
        method: string,
        url: string,
        data?: unknown,
        config: RequestConfig = {}
    ): Promise<HttpResponse<T>> {
        // Create full config with data for interceptor
        const configWithData: RequestConfig = { ...config, data }

        // Apply request interceptor if provided
        let finalConfig: RequestConfig = configWithData
        if (this.requestInterceptor) {
            finalConfig = this.requestInterceptor(configWithData)
        }

        // Get request data from finalConfig
        const requestData = finalConfig.data

        // Build URL with query params (use per-request baseURL if provided)
        const baseURL = finalConfig.baseURL ?? this.baseURL
        const fullURL = buildURL(baseURL, url, finalConfig.params)

        // Merge headers
        const headers = new Headers(this.defaultHeaders)
        if (finalConfig.headers) {
            for (const [key, value] of Object.entries(finalConfig.headers)) {
                headers.set(key, value)
            }
        }

        // Set content type for JSON body if not already set and not FormData
        if (requestData !== undefined && !(requestData instanceof FormData)) {
            if (!headers.has('Content-Type')) {
                headers.set('Content-Type', 'application/json')
            }
        }

        // Determine timeout
        const timeout = finalConfig.timeout ?? this.defaultTimeout

        // Check if we need XHR for upload progress
        if (finalConfig.onUploadProgress && requestData instanceof FormData) {
            return this.uploadWithXHR<T>(fullURL, requestData, {
                headers: Object.fromEntries(headers.entries()),
                signal: finalConfig.signal,
                onUploadProgress: finalConfig.onUploadProgress,
            })
        }

        // Set up AbortController for timeout
        const controller = new AbortController()
        const timeoutId = setTimeout(() => {
            controller.abort(new Error('TIMEOUT'))
        }, timeout)

        // Connect external signal if provided
        if (finalConfig.signal) {
            finalConfig.signal.addEventListener('abort', () => {
                controller.abort()
            })
        }

        try {
            // Prepare request init
            const init: RequestInit = {
                method,
                headers,
                signal: controller.signal,
            }

            // Add body for non-GET/HEAD requests
            if (requestData !== undefined && method !== 'GET' && method !== 'HEAD') {
                if (requestData instanceof FormData) {
                    init.body = requestData
                    // Don't set Content-Type for FormData - browser sets it with boundary
                    headers.delete('Content-Type')
                } else if (typeof requestData === 'string') {
                    init.body = requestData
                } else {
                    init.body = JSON.stringify(requestData)
                }
            }

            // Execute fetch
            const response = await fetch(fullURL, init)

            // Clear timeout on success
            clearTimeout(timeoutId)

            // Parse response body based on responseType or content-type
            let responseData: T
            const contentType = response.headers.get('content-type') || ''

            if (finalConfig.responseType === 'text') {
                responseData = await response.text() as unknown as T
            } else if (finalConfig.responseType === 'blob') {
                responseData = await response.blob() as unknown as T
            } else if (finalConfig.responseType === 'arraybuffer') {
                responseData = await response.arrayBuffer() as unknown as T
            } else if (response.status === 204 || !contentType) {
                // No content
                responseData = null as T
            } else if (contentType.includes('application/json')) {
                responseData = await response.json() as T
            } else {
                // Default to text for unknown content types
                responseData = await response.text() as unknown as T
            }

            // Build response object
            const httpResponse: HttpResponse<T> = {
                data: responseData,
                status: response.status,
                statusText: response.statusText,
                headers: response.headers,
            }

            // Apply response interceptor if provided (for both success and error responses)
            let interceptedResponse: HttpResponse<T> = httpResponse
            if (this.responseInterceptor) {
                interceptedResponse = this.responseInterceptor(httpResponse) as HttpResponse<T>
            }

            // Handle HTTP errors after interceptor
            if (!response.ok) {
                throw new HttpError(interceptedResponse.status, interceptedResponse.data)
            }

            return interceptedResponse

        } catch (error) {
            clearTimeout(timeoutId)

            // Handle different error types
            if (error instanceof HttpError) {
                throw error
            }

            if (error instanceof Error) {
                if (error.name === 'AbortError') {
                    // Distinguish timeout from user abort
                    if (error.message === 'TIMEOUT') {
                        throw new TimeoutError()
                    }
                    throw new AbortError()
                }
            }

            // Network errors or other fetch failures
            throw new HttpError(0, null, error instanceof Error ? error.message : 'Network error')
        }
    }

    private async uploadWithXHR<T>(
        url: string,
        formData: FormData,
        config: UploadConfig
    ): Promise<HttpResponse<T>> {
        const response = await uploadWithProgress<T>(url, formData, config)

        // Apply response interceptor if provided
        if (this.responseInterceptor) {
            return this.responseInterceptor(response)
        }

        return response
    }
}

export default HttpClient
