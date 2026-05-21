import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { uploadWithProgress } from '../uploadWithProgress'
import { HttpError, AbortError } from '../errors'
import { NetworkError } from '@/core/errors/AppError'

describe('uploadWithProgress', () => {
  let mockInstance: any
  let MockXHR: any
  let originalXHR: typeof XMLHttpRequest

  beforeEach(() => {
    originalXHR = global.XMLHttpRequest

    // Create a mock constructor function
    mockInstance = {
      open: vi.fn(),
      setRequestHeader: vi.fn(),
      send: vi.fn(),
      abort: vi.fn(),
      upload: { addEventListener: vi.fn() },
      addEventListener: vi.fn(),
      getAllResponseHeaders: vi.fn(() => 'Content-Type: application/json'),
      status: 200,
      statusText: 'OK',
      responseText: JSON.stringify({ id: 'file-1', name: 'test.txt' }),
    }

    // Create a proper constructor function that returns the mock instance
    MockXHR = function () {
      Object.assign(this, mockInstance)
      return this
    }

    global.XMLHttpRequest = MockXHR as unknown as typeof XMLHttpRequest
  })

  afterEach(() => {
    global.XMLHttpRequest = originalXHR
    vi.restoreAllMocks()
  })

  it('creates XHR request', async () => {
    const formData = new FormData()
    formData.append('file', new Blob(['test']), 'test.txt')

    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    // Trigger load event
    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    await promise

    expect(mockInstance.open).toHaveBeenCalledWith('POST', 'https://api.example.com/upload')
  })

  it('sets headers correctly', async () => {
    const formData = new FormData()
    const promise = uploadWithProgress('https://api.example.com/upload', formData, {
      headers: { Authorization: 'Bearer token123', 'X-Custom': 'value' },
    })

    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    await promise

    expect(mockInstance.setRequestHeader).toHaveBeenCalledWith('Authorization', 'Bearer token123')
    expect(mockInstance.setRequestHeader).toHaveBeenCalledWith('X-Custom', 'value')
  })

  it('calls progress callback with loaded and total bytes', async () => {
    const formData = new FormData()
    const onProgress = vi.fn()

    const promise = uploadWithProgress('https://api.example.com/upload', formData, {
      onUploadProgress: onProgress,
    })

    // Trigger progress event
    const progressHandler = mockInstance.upload.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'progress'
    )?.[1]
    progressHandler?.({ lengthComputable: true, loaded: 500, total: 1000 })

    // Complete upload
    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    await promise

    expect(onProgress).toHaveBeenCalledWith({
      loaded: 500,
      total: 1000,
      percentage: 50,
    })
  })

  it('resolves with response data on success', async () => {
    const formData = new FormData()
    const mockResponse = { id: 'file-1', name: 'test.txt', size: 1024 }
    mockInstance.responseText = JSON.stringify(mockResponse)

    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    const response = await promise

    expect(response.data).toEqual(mockResponse)
    expect(response.status).toBe(200)
    expect(response.statusText).toBe('OK')
  })

  it('rejects with HttpError on 4xx/5xx response', async () => {
    const formData = new FormData()

    // Set error status before creating the XHR
    mockInstance.status = 400
    mockInstance.statusText = 'Bad Request'
    mockInstance.responseText = JSON.stringify({ error: 'Invalid file' })

    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    await expect(promise).rejects.toBeInstanceOf(HttpError)
  })

  it('rejects with NetworkError on network failure', async () => {
    const formData = new FormData()
    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    const errorHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'error'
    )?.[1]
    errorHandler?.()

    await expect(promise).rejects.toBeInstanceOf(NetworkError)
  })

  it('rejects with AbortError when aborted', async () => {
    const formData = new FormData()
    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    const abortHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'abort'
    )?.[1]
    abortHandler?.()

    await expect(promise).rejects.toBeInstanceOf(AbortError)
  })

  it('aborts XHR when signal is triggered', async () => {
    const formData = new FormData()
    const controller = new AbortController()

    uploadWithProgress('https://api.example.com/upload', formData, {
      signal: controller.signal,
    })

    // Trigger abort
    controller.abort()

    expect(mockInstance.abort).toHaveBeenCalled()
  })

  it('parses response headers correctly', async () => {
    const formData = new FormData()
    mockInstance.getAllResponseHeaders = vi.fn(
      () => 'Content-Type: application/json\r\nX-Custom-Header: value'
    )

    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    const response = await promise

    expect(response.headers.get('Content-Type')).toBe('application/json')
    expect(response.headers.get('X-Custom-Header')).toBe('value')
  })

  it('handles non-JSON response', async () => {
    const formData = new FormData()
    mockInstance.responseText = 'plain text response'

    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    const response = await promise

    expect(response.data).toBe('plain text response')
  })

  it('handles empty response', async () => {
    const formData = new FormData()
    mockInstance.responseText = ''

    const promise = uploadWithProgress('https://api.example.com/upload', formData, {})

    const loadHandler = mockInstance.addEventListener.mock.calls.find(
      (call: any[]) => call[0] === 'load'
    )?.[1]
    loadHandler?.()

    const response = await promise

    expect(response.data).toBeNull()
  })
})
