import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { HttpClient } from '../HttpClient'
import { normalizeIdsDeep, shouldNormalizeHttpPayload } from '@/utils/idCompat'

describe('Auth Integration', () => {
  let client: HttpClient
  let fetchMock: ReturnType<typeof vi.fn>
  let mockLogout: ReturnType<typeof vi.fn>

  beforeEach(() => {
    fetchMock = vi.fn()
    global.fetch = fetchMock
    mockLogout = vi.fn()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('Auth header injection', () => {
    it('injects auth token when available', async () => {
      const authToken = 'test-token-123'

      client = new HttpClient({
        baseURL: 'https://api.example.com',
        requestInterceptor: config => {
          if (authToken) {
            config.headers = {
              ...config.headers,
              Authorization: `Bearer ${authToken}`,
            }
          }
          return config
        },
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.get('/test')

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      const headers = callArgs.headers as Headers
      expect(headers.get('Authorization')).toBe('Bearer test-token-123')
    })

    it('omits auth header when token is not available', async () => {
      client = new HttpClient({
        baseURL: 'https://api.example.com',
        requestInterceptor: config => config,
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.get('/test')

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      const headers = callArgs.headers as Headers
      expect(headers.has('Authorization')).toBe(false)
    })
  })

  describe('401 handling', () => {
    it('triggers logout on 401 response', async () => {
      client = new HttpClient({
        baseURL: 'https://api.example.com',
        responseInterceptor: response => {
          if (response.status === 401) {
            mockLogout()
          }
          return response
        },
      })

      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        headers: new Headers(),
        json: () => Promise.resolve({ message: 'Unauthorized' }),
      } as Response)

      await expect(client.get('/test')).rejects.toThrow()
      expect(mockLogout).toHaveBeenCalled()
    })

    it('does not trigger logout on other errors', async () => {
      client = new HttpClient({
        baseURL: 'https://api.example.com',
        responseInterceptor: response => {
          if (response.status === 401) {
            mockLogout()
          }
          return response
        },
      })

      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
        headers: new Headers(),
        json: () => Promise.resolve({ message: 'Forbidden' }),
      } as Response)

      await expect(client.get('/test')).rejects.toThrow()
      expect(mockLogout).not.toHaveBeenCalled()
    })
  })

  describe('ID normalization', () => {
    it('applies request interceptor to modify params', async () => {
      const requestInterceptor = vi.fn(config => {
        if (config.params) {
          config.params = { ...config.params, intercepted: 'true' }
        }
        return config
      })

      client = new HttpClient({
        baseURL: 'https://api.example.com',
        requestInterceptor,
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.get('/test', { params: { page: 1 } })

      expect(requestInterceptor).toHaveBeenCalled()
      const url = fetchMock.mock.calls[0]![0] as string
      expect(url).toContain('intercepted=true')
    })

    it('calls request interceptor with config', async () => {
      const requestInterceptor = vi.fn(config => config)

      client = new HttpClient({
        baseURL: 'https://api.example.com',
        requestInterceptor,
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.post('/test', { message: 'hello' })

      expect(requestInterceptor).toHaveBeenCalled()
      const callArg = requestInterceptor.mock.calls[0]![0]
      expect(callArg.data).toEqual({ message: 'hello' })
    })

    it('applies response interceptor to response data', async () => {
      const mockResponse = {
        id: 'user-1',
        name: 'Test User',
      }

      const responseInterceptor = vi.fn(response => {
        response.data = { ...response.data, intercepted: true }
        return response
      })

      client = new HttpClient({
        baseURL: 'https://api.example.com',
        responseInterceptor,
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(mockResponse),
      } as Response)

      const response = await client.get('/test')

      expect(responseInterceptor).toHaveBeenCalled()
      expect(response.data.intercepted).toBe(true)
    })

    it('does not normalize FormData body', async () => {
      const formData = new FormData()
      formData.append('file', new Blob(['test']), 'test.txt')

      client = new HttpClient({
        baseURL: 'https://api.example.com',
        requestInterceptor: config => {
          if (shouldNormalizeHttpPayload(config)) {
            // Should not be called for FormData
          }
          return config
        },
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.post('/upload', formData)

      // FormData should pass through unchanged
      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      expect(callArgs.body).toBeInstanceOf(FormData)
    })

    it('does not normalize URLSearchParams', async () => {
      const params = new URLSearchParams({ key: 'value' })

      expect(shouldNormalizeHttpPayload(params)).toBe(false)
    })
  })
})
