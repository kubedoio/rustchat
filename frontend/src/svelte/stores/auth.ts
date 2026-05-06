import { derived, writable } from 'svelte/store'
import { readLocalStorage, writeLocalStorage } from '../lib/storage'
import { svelteApi, SvelteHttpError } from './http'
import { themeStore } from './theme'
import type { AuthResponse, AuthUser, LoginCredentials } from '../../core/entities/Auth'

export type LogoutReason = 'manual' | 'expired' | 'unauthorized'

export interface AuthState {
    token: string
    user: AuthUser | null
    loading: boolean
    error: string | null
}

const AUTH_TOKEN_KEY = 'auth_token'
let tokenExpiryTimer: ReturnType<typeof setTimeout> | null = null
let isLoggingOut = false

function setAuthCookie(token: string): void {
    if (typeof document === 'undefined') {
        return
    }

    document.cookie = `MMAUTHTOKEN=${token}; path=/; SameSite=Strict`
}

function clearAuthCookie(): void {
    if (typeof document === 'undefined') {
        return
    }

    document.cookie = 'MMAUTHTOKEN=; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT'
}

function parseJwtExpiryMs(token: string): number | null {
    if (!token) {
        return null
    }

    const payloadPart = token.split('.')[1]
    if (!payloadPart || typeof atob === 'undefined') {
        return null
    }

    try {
        const normalized = payloadPart.replace(/-/g, '+').replace(/_/g, '/')
        const paddingLength = (4 - (normalized.length % 4)) % 4
        const payload = JSON.parse(atob(normalized + '='.repeat(paddingLength))) as { exp?: unknown }
        const expSeconds = Number(payload.exp)
        return Number.isFinite(expSeconds) && expSeconds > 0 ? expSeconds * 1000 : null
    } catch {
        return null
    }
}

function clearTokenExpiryTimer(): void {
    if (!tokenExpiryTimer) {
        return
    }

    clearTimeout(tokenExpiryTimer)
    tokenExpiryTimer = null
}

function normalizeUser(user: AuthUser): AuthUser {
    if (!user.custom_status) {
        return user
    }

    return {
        ...user,
        status_text: user.custom_status.text,
        status_emoji: user.custom_status.emoji,
        status_expires_at: user.custom_status.expires_at ?? user.custom_status.expiresAt ?? null,
    }
}

function createAuthStore() {
    const { subscribe, set, update } = writable<AuthState>({
        token: readLocalStorage(AUTH_TOKEN_KEY, ''),
        user: null,
        loading: false,
        error: null,
    })

    async function logout(_reason: LogoutReason = 'manual'): Promise<void> {
        if (isLoggingOut) {
            return
        }

        isLoggingOut = true
        clearTokenExpiryTimer()

        try {
            writeLocalStorage(AUTH_TOKEN_KEY, '')
            clearAuthCookie()
            set({
                token: '',
                user: null,
                loading: false,
                error: null,
            })

            if (typeof window !== 'undefined' && window.location.pathname !== '/login') {
                window.location.replace('/login')
            }
        } finally {
            isLoggingOut = false
        }
    }

    function scheduleTokenExpiryLogout(token: string): void {
        clearTokenExpiryTimer()

        const expiryMs = parseJwtExpiryMs(token)
        if (!expiryMs) {
            return
        }

        const remainingMs = expiryMs - Date.now()
        if (remainingMs <= 0) {
            void logout('expired')
            return
        }

        tokenExpiryTimer = setTimeout(() => {
            void logout('expired')
        }, remainingMs)
    }

    async function fetchMe(): Promise<AuthUser | null> {
        const token = readLocalStorage(AUTH_TOKEN_KEY, '')
        if (!token) {
            return null
        }

        setAuthCookie(token)
        update((state) => ({ ...state, token, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<AuthUser>('/auth/me')
            const user = normalizeUser(data)
            update((state) => ({ ...state, user, token, loading: false, error: null }))
            scheduleTokenExpiryLogout(token)
            await themeStore.syncFromServer()
            return user
        } catch (error) {
            await logout(error instanceof SvelteHttpError && error.status === 401 ? 'unauthorized' : 'expired')
            update((state) => ({
                ...state,
                loading: false,
                error: error instanceof Error ? error.message : 'Failed to fetch current user',
            }))
            return null
        }
    }

    return {
        subscribe,
        async login(credentials: LoginCredentials): Promise<AuthUser> {
            update((state) => ({ ...state, loading: true, error: null }))

            try {
                const { data } = await svelteApi.post<AuthResponse>('/auth/login', credentials, {
                    authenticated: false,
                })
                writeLocalStorage(AUTH_TOKEN_KEY, data.token)
                setAuthCookie(data.token)
                scheduleTokenExpiryLogout(data.token)

                const user = normalizeUser(data.user)
                update((state) => ({ ...state, token: data.token, user, loading: false, error: null }))

                return await fetchMe() ?? user
            } catch (error) {
                clearAuthCookie()
                writeLocalStorage(AUTH_TOKEN_KEY, '')
                update((state) => ({
                    ...state,
                    token: '',
                    user: null,
                    loading: false,
                    error: error instanceof Error ? error.message : 'Login failed',
                }))
                throw error
            }
        },
        fetchMe,
        logout,
        setToken(token: string): void {
            writeLocalStorage(AUTH_TOKEN_KEY, token)
            if (token) {
                setAuthCookie(token)
                scheduleTokenExpiryLogout(token)
            } else {
                clearAuthCookie()
                clearTokenExpiryTimer()
            }
            update((state) => ({ ...state, token }))
        },
        reset(): void {
            clearTokenExpiryTimer()
            set({
                token: readLocalStorage(AUTH_TOKEN_KEY, ''),
                user: null,
                loading: false,
                error: null,
            })
        },
    }
}

export const authStore = createAuthStore()
export const isAuthenticated = derived(authStore, ($authStore) => Boolean($authStore.token))

if (typeof window !== 'undefined' && readLocalStorage(AUTH_TOKEN_KEY, '')) {
    setAuthCookie(readLocalStorage(AUTH_TOKEN_KEY, ''))
}
