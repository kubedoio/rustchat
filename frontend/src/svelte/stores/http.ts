import { readLocalStorage } from '../lib/storage'

export interface SvelteHttpResponse<T> {
    data: T
    headers: Headers
    status: number
    statusText: string
}

export class SvelteHttpError<T = unknown> extends Error {
    readonly status: number
    readonly data: T

    constructor(status: number, data: T, message = `HTTP request failed with status ${status}`) {
        super(message)
        this.name = 'SvelteHttpError'
        this.status = status
        this.data = data
    }
}

export interface SvelteHttpOptions {
    baseURL?: string
    headers?: Record<string, string>
    authenticated?: boolean
    params?: Record<string, string | number | boolean | undefined>
}

const AUTH_TOKEN_KEY = 'auth_token'
const DEFAULT_API_BASE = import.meta.env.VITE_API_URL || '/api/v1'

function buildUrl(path: string, baseURL = DEFAULT_API_BASE, params?: Record<string, string | number | boolean | undefined>): string {
    if (/^https?:\/\//.test(path)) {
        return path
    }

    let url = `${baseURL.replace(/\/$/, '')}/${path.replace(/^\//, '')}`

    if (params) {
        const searchParams = new URLSearchParams()
        for (const [key, value] of Object.entries(params)) {
            if (value !== undefined && value !== null) {
                searchParams.append(key, String(value))
            }
        }
        const queryString = searchParams.toString()
        if (queryString) {
            url += `?${queryString}`
        }
    }

    return url
}

async function parseBody<T>(response: Response): Promise<T> {
    const contentType = response.headers.get('content-type') || ''

    if (response.status === 204 || !contentType) {
        return null as T
    }

    if (contentType.includes('application/json')) {
        return response.json() as Promise<T>
    }

    return response.text() as Promise<T>
}

export async function svelteHttp<T>(
    method: string,
    path: string,
    body?: unknown,
    options: SvelteHttpOptions = {},
): Promise<SvelteHttpResponse<T>> {
    const headers = new Headers(options.headers)
    const token = options.authenticated === false ? '' : readLocalStorage(AUTH_TOKEN_KEY, '')

    if (token) {
        headers.set('Authorization', `Bearer ${token}`)
    }

    const init: RequestInit = {
        method,
        headers,
    }

    if (body !== undefined) {
        headers.set('Content-Type', 'application/json')
        init.body = JSON.stringify(body)
    }

    const response = await fetch(buildUrl(path, options.baseURL, options.params), init)
    const data = await parseBody<T>(response)

    if (!response.ok) {
        throw new SvelteHttpError(response.status, data)
    }

    return {
        data,
        headers: response.headers,
        status: response.status,
        statusText: response.statusText,
    }
}

export async function svelteHttpFormData<T>(
    path: string,
    formData: FormData,
    options: SvelteHttpOptions = {},
): Promise<SvelteHttpResponse<T>> {
    const headers = new Headers(options.headers)
    const token = options.authenticated === false ? '' : readLocalStorage(AUTH_TOKEN_KEY, '')

    if (token) {
        headers.set('Authorization', `Bearer ${token}`)
    }

    const init: RequestInit = {
        method: 'POST',
        headers,
        body: formData,
    }

    const response = await fetch(buildUrl(path, options.baseURL, options.params), init)
    const data = await parseBody<T>(response)

    if (!response.ok) {
        throw new SvelteHttpError(response.status, data)
    }

    return {
        data,
        headers: response.headers,
        status: response.status,
        statusText: response.statusText,
    }
}

export const svelteApi = {
    get: <T>(path: string, options?: SvelteHttpOptions) => svelteHttp<T>('GET', path, undefined, options),
    post: <T>(path: string, body?: unknown, options?: SvelteHttpOptions) =>
        svelteHttp<T>('POST', path, body, options),
    postFormData: <T>(path: string, formData: FormData, options?: SvelteHttpOptions) =>
        svelteHttpFormData<T>(path, formData, options),
    put: <T>(path: string, body?: unknown, options?: SvelteHttpOptions) =>
        svelteHttp<T>('PUT', path, body, options),
    patch: <T>(path: string, body?: unknown, options?: SvelteHttpOptions) =>
        svelteHttp<T>('PATCH', path, body, options),
    delete: <T>(path: string, options?: SvelteHttpOptions) =>
        svelteHttp<T>('DELETE', path, undefined, options),
}
