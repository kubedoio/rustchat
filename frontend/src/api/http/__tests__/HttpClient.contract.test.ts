import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { HttpClient } from '../HttpClient'
import { TimeoutError, AbortError, HttpError } from '../errors'

describe('HttpClient', () => {
  let client: HttpClient
  let fetchMock: ReturnType<typeof vi.fn>

  beforeEach(() => {
    fetchMock = vi.fn()
    global.fetch = fetchMock
    client = new HttpClient({ baseURL: 'https://api.example.com' })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('GET requests', () => {
    it('returns parsed JSON response', async () => {
      const mockData = { id: '1', name: 'Test' }
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(mockData),
      } as Response)

      const response = await client.get('/users/1')

      expect(response.data).toEqual(mockData)
      expect(response.status).toBe(200)
      expect(fetchMock).toHaveBeenCalledWith(
        'https://api.example.com/users/1',
        expect.objectContaining({ method: 'GET' })
      )
    })

    it('prepends baseURL to request URL', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.get('/test')

      expect(fetchMock).toHaveBeenCalledWith('https://api.example.com/test', expect.any(Object))
    })

    it('merges request headers with defaults', async () => {
      const customClient = new HttpClient({
        baseURL: 'https://api.example.com',
        headers: { 'X-Default': 'value' },
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await customClient.get('/test', { headers: { 'X-Custom': 'override' } })

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      const headers = callArgs.headers as Headers
      expect(headers.get('X-Default')).toBe('value')
      expect(headers.get('X-Custom')).toBe('override')
    })

    it('serializes query params correctly', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.get('/test', {
        params: {
          page: 1,
          tags: ['a', 'b'],
          search: null,
          active: true,
        },
      })

      expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining('page=1'), expect.any(Object))
      expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining('tags=a'), expect.any(Object))
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('active=true'),
        expect.any(Object)
      )
    })
  })

  describe('POST requests', () => {
    it('sends JSON body', async () => {
      const requestData = { name: 'Test User', email: 'test@example.com' }

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 201,
        statusText: 'Created',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve({ id: '1', ...requestData }),
      } as Response)

      await client.post('/users', requestData)

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      expect(callArgs.body).toBe(JSON.stringify(requestData))
      expect(callArgs.method).toBe('POST')
    })

    it('sends FormData without JSON content-type', async () => {
      const formData = new FormData()
      formData.append('file', new Blob(['test']), 'test.txt')

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({}),
      } as Response)

      await client.post('/upload', formData)

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      const headers = callArgs.headers as Headers
      expect(headers.get('Content-Type')).toBeNull()
      expect(callArgs.body).toBe(formData)
    })
  })

  describe('PUT requests', () => {
    it('sends JSON body', async () => {
      const updateData = { name: 'Updated' }

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve(updateData),
      } as Response)

      await client.put('/users/1', updateData)

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      expect(callArgs.body).toBe(JSON.stringify(updateData))
      expect(callArgs.method).toBe('PUT')
    })
  })

  describe('PATCH requests', () => {
    it('sends partial JSON body', async () => {
      const patchData = { status: 'active' }

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve(patchData),
      } as Response)

      await client.patch('/users/1', patchData)

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      expect(callArgs.body).toBe(JSON.stringify(patchData))
      expect(callArgs.method).toBe('PATCH')
    })
  })

  describe('DELETE requests', () => {
    it('sends without body', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 204,
        statusText: 'No Content',
        headers: new Headers(),
        json: () => Promise.resolve(null),
      } as Response)

      await client.delete('/users/1')

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      expect(callArgs.body).toBeUndefined()
      expect(callArgs.method).toBe('DELETE')
    })
  })

  describe('Response handling', () => {
    it('handles empty response body', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 204,
        statusText: 'No Content',
        headers: new Headers(),
        json: () => Promise.resolve(null),
      } as Response)

      const response = await client.delete('/users/1')

      expect(response.data).toBeNull()
      expect(response.status).toBe(204)
    })

    it('returns text for text response type', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'text/plain' }),
        text: () => Promise.resolve('Hello World'),
      } as Response)

      const response = await client.get('/text', { responseType: 'text' })

      expect(response.data).toBe('Hello World')
    })

    it('returns blob for blob response type', async () => {
      const blob = new Blob(['test'])
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        blob: () => Promise.resolve(blob),
      } as Response)

      const response = await client.get('/file', { responseType: 'blob' })

      expect(response.data).toBe(blob)
    })

    it('returns response headers', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'X-Custom-Header': 'value' }),
        json: () => Promise.resolve({}),
      } as Response)

      const response = await client.get('/test')

      expect(response.headers.get('X-Custom-Header')).toBe('value')
    })
  })

  describe('Error handling', () => {
    it('throws HttpError for 4xx responses', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve({ error: 'Invalid input' }),
      } as Response)

      await expect(client.get('/test')).rejects.toBeInstanceOf(HttpError)
    })

    it('throws HttpError for 5xx responses', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        headers: new Headers(),
        json: () => Promise.resolve({ error: 'Server error' }),
      } as Response)

      await expect(client.get('/test')).rejects.toBeInstanceOf(HttpError)
    })

    it('throws TimeoutError on timeout', async () => {
      fetchMock.mockImplementationOnce(() => {
        return new Promise((_, reject) => {
          setTimeout(() => {
            const error = new Error('TIMEOUT')
            error.name = 'AbortError'
            reject(error)
          }, 10)
        })
      })

      await expect(client.get('/test', { timeout: 5 })).rejects.toBeInstanceOf(TimeoutError)
    })

    it('throws AbortError on manual abort', async () => {
      const controller = new AbortController()
      fetchMock.mockImplementationOnce(() => {
        return new Promise((_, reject) => {
          setTimeout(() => {
            controller.abort()
            const error = new Error('AbortError')
            error.name = 'AbortError'
            reject(error)
          }, 10)
        })
      })

      const request = client.get('/test', { signal: controller.signal })
      setTimeout(() => controller.abort(), 5)

      await expect(request).rejects.toBeInstanceOf(AbortError)
    })
  })

  describe('Interceptors', () => {
    it('applies request interceptor', async () => {
      const requestInterceptor = vi.fn(config => ({
        ...config,
        headers: { ...config.headers, 'X-Intercepted': 'true' },
      }))

      const interceptClient = new HttpClient({
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

      await interceptClient.get('/test')

      expect(requestInterceptor).toHaveBeenCalled()
      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      const headers = callArgs.headers as Headers
      expect(headers.get('X-Intercepted')).toBe('true')
    })

    it('applies response interceptor', async () => {
      const responseInterceptor = vi.fn(response => ({
        ...response,
        data: { intercepted: true },
      }))

      const interceptClient = new HttpClient({
        baseURL: 'https://api.example.com',
        responseInterceptor,
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers(),
        json: () => Promise.resolve({ original: true }),
      } as Response)

      const response = await interceptClient.get('/test')

      expect(responseInterceptor).toHaveBeenCalled()
      expect(response.data).toEqual({ intercepted: true })
    })
  })
})
