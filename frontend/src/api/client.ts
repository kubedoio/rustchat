import { HttpClient, type HttpResponse, type RequestConfig } from './http/HttpClient'
import { useAuthStore } from '../features/auth/stores/authStore'
import { normalizeIdsDeep, shouldNormalizeHttpPayload } from '../utils/idCompat'
import { API_V1_BASE, API_V4_BASE } from '../constants'

/**
 * Shared interceptors used by both v1 and v4 API clients.
 */
const requestInterceptor = (config: RequestConfig): RequestConfig => {
  const authStore = useAuthStore()

  // Add auth header if token exists
  if (authStore.token) {
    config.headers = {
      ...(config.headers ?? {}),
      Authorization: `Bearer ${authStore.token}`,
    }
  }

  // Normalize IDs in params and body
  if (shouldNormalizeHttpPayload(config.params)) {
    config.params = normalizeIdsDeep(config.params)
  }
  if (shouldNormalizeHttpPayload(config.data)) {
    config.data = normalizeIdsDeep(config.data)
  }

  return config
}

const responseInterceptor = <T>(response: HttpResponse<T>): HttpResponse<T> => {
  // Normalize IDs in response data
  if (shouldNormalizeHttpPayload(response.data)) {
    response.data = normalizeIdsDeep(response.data) as T
  }

  // Handle 401 - logout
  if (response.status === 401) {
    const authStore = useAuthStore()
    authStore.logout()
  }

  return response
}

/**
 * Main API client for v1 endpoints
 * Replaces Axios with native Fetch-based HttpClient
 */
const client = new HttpClient({
  baseURL: import.meta.env.VITE_API_URL || API_V1_BASE,
  requestInterceptor,
  responseInterceptor,
})

/**
 * Named v4 API client for MM-compatible endpoints.
 * Shares the same interceptors as the v1 client.
 */
export const v4Api = new HttpClient({
  baseURL: API_V4_BASE,
  requestInterceptor,
  responseInterceptor,
})

export default client
