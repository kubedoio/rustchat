import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { HttpClient } from '../HttpClient'
import { HttpError } from '../errors'

describe('Integration Tests', () => {
  let client: HttpClient
  let fetchMock: ReturnType<typeof vi.fn>

  beforeEach(() => {
    fetchMock = vi.fn()
    global.fetch = fetchMock
    client = new HttpClient({
      baseURL: 'https://api.example.com',
      headers: { 'X-Client': 'test' },
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('Login flow', () => {
    it('POST /auth/login returns token', async () => {
      const loginResponse = {
        token: 'jwt-token-123',
        user: { id: 'user-1', username: 'testuser' },
      }

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(loginResponse),
      } as Response)

      const response = await client.post('/auth/login', {
        username: 'testuser',
        password: 'secret123',
      })

      expect(response.data.token).toBe('jwt-token-123')
      expect(response.data.user.username).toBe('testuser')

      const callArgs = fetchMock.mock.calls[0]![1] as RequestInit
      expect(callArgs.method).toBe('POST')
      const body = JSON.parse(callArgs.body as string)
      expect(body.username).toBe('testuser')
      expect(body.password).toBe('secret123')
    })

    it('rejects invalid credentials', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve({ message: 'Invalid credentials' }),
      } as Response)

      await expect(
        client.post('/auth/login', {
          username: 'wrong',
          password: 'wrong',
        })
      ).rejects.toBeInstanceOf(HttpError)
    })
  })

  describe('Get current user', () => {
    it('GET /auth/me returns user data', async () => {
      const userData = {
        id: 'user-1',
        username: 'testuser',
        email: 'test@example.com',
        role: 'user',
      }

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(userData),
      } as Response)

      const response = await client.get('/auth/me')

      expect(response.data).toEqual(userData)
      expect(fetchMock).toHaveBeenCalledWith(
        'https://api.example.com/auth/me',
        expect.objectContaining({
          method: 'GET',
          headers: expect.any(Headers),
        })
      )
    })
  })

  describe('List channels', () => {
    it('GET /channels returns channel list', async () => {
      const channels = [
        { id: 'ch-1', name: 'general', channel_type: 'public' },
        { id: 'ch-2', name: 'random', channel_type: 'public' },
      ]

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(channels),
      } as Response)

      const response = await client.get('/channels', {
        params: { team_id: 'team-1' },
      })

      expect(response.data).toEqual(channels)
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('team_id=team-1'),
        expect.any(Object)
      )
    })
  })

  describe('Create post', () => {
    it('POST /channels/:id/posts creates a message', async () => {
      const newPost = {
        id: 'post-1',
        channel_id: 'ch-1',
        user_id: 'user-1',
        message: 'Hello world',
        created_at: '2024-01-15T10:30:00Z',
      }

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 201,
        statusText: 'Created',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(newPost),
      } as Response)

      const response = await client.post('/channels/ch-1/posts', {
        message: 'Hello world',
        channel_id: 'ch-1',
      })

      expect(response.data.message).toBe('Hello world')
      expect(response.status).toBe(201)
    })

    it('validates message content', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve({ message: 'Message cannot be empty' }),
      } as Response)

      await expect(
        client.post('/channels/ch-1/posts', {
          message: '',
          channel_id: 'ch-1',
        })
      ).rejects.toBeInstanceOf(HttpError)
    })
  })

  describe('Get posts with pagination', () => {
    it('GET /channels/:id/posts with pagination params', async () => {
      const postsResponse = {
        messages: [
          { id: 'post-1', message: 'First' },
          { id: 'post-2', message: 'Second' },
        ],
        read_state: { last_read_message_id: 'post-2' },
      }

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(postsResponse),
      } as Response)

      const response = await client.get('/channels/ch-1/posts', {
        params: {
          before: 'post-3',
          limit: 20,
        },
      })

      expect(response.data.messages).toHaveLength(2)
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('before=post-3'),
        expect.any(Object)
      )
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('limit=20'),
        expect.any(Object)
      )
    })
  })

  describe('Mattermost Calls API', () => {
    it('GET /api/v4/plugins/com.mattermost.calls/config', async () => {
      const callsConfig = {
        ICEServersConfigs: [{ urls: ['stun:stun.example.com'] }],
        AllowEnableCalls: true,
        DefaultEnabled: true,
      }

      const v4Client = new HttpClient({
        baseURL: 'https://api.example.com/api/v4',
      })

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        json: () => Promise.resolve(callsConfig),
      } as Response)

      const response = await v4Client.get('/plugins/com.mattermost.calls/config')

      expect(response.data.ICEServersConfigs).toBeDefined()
      expect(fetchMock).toHaveBeenCalledWith(
        'https://api.example.com/api/v4/plugins/com.mattermost.calls/config',
        expect.any(Object)
      )
    })
  })

  describe('Error recovery', () => {
    it('marks 5xx errors as recoverable', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        headers: new Headers(),
        json: () => Promise.resolve({ error: 'Server error' }),
      } as Response)

      try {
        await client.get('/test')
      } catch (error) {
        if (error instanceof HttpError) {
          expect(error.recoverable).toBe(true)
        }
      }
    })

    it('does not mark 4xx errors as recoverable', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        headers: new Headers(),
        json: () => Promise.resolve({ error: 'Bad request' }),
      } as Response)

      try {
        await client.get('/test')
      } catch (error) {
        if (error instanceof HttpError) {
          expect(error.recoverable).toBe(false)
        }
      }
    })
  })
})
