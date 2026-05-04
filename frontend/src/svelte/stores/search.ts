import { writable } from 'svelte/store'
import { svelteApi } from './http'

export interface SearchResult {
    id: string
    channel_id: string
    message: string
    created_at: string
    username?: string
}

export type SearchFilter = 'messages' | 'files' | 'channels' | 'users'

export interface SearchState {
    query: string
    results: SearchResult[]
    filter: SearchFilter
    loading: boolean
    error: string
    recentSearches: string[]
}

const initialState: SearchState = {
    query: '',
    results: [],
    filter: 'messages',
    loading: false,
    error: '',
    recentSearches: [],
}

function createSearchStore() {
    const { subscribe, set, update } = writable<SearchState>(initialState)
    let searchTimeout: ReturnType<typeof setTimeout> | null = null

    async function doSearch(query: string, filter: SearchFilter) {
        update((state) => ({ ...state, loading: true, error: '', query, filter }))

        try {
            const params = new URLSearchParams()
            params.set('q', query)
            params.set('per_page', '20')
            if (filter !== 'messages') {
                params.set('type', filter)
            }

            const { data } = await svelteApi.get<{
                posts: Array<{
                    id: string
                    channel_id: string
                    message: string
                    created_at: string
                    username?: string
                }>
                total: number
            }>(`/search?${params.toString()}`)

            const results: SearchResult[] = (data.posts || []).map((post) => ({
                id: post.id,
                channel_id: post.channel_id,
                message: post.message,
                created_at: post.created_at,
                username: post.username,
            }))

            update((state) => {
                const recentSearches = state.recentSearches.includes(query)
                    ? state.recentSearches
                    : [query, ...state.recentSearches].slice(0, 5)
                return { ...state, results, loading: false, recentSearches }
            })
        } catch (err: unknown) {
            const message = err instanceof Error ? err.message : 'Search failed'
            update((state) => ({ ...state, loading: false, error: message }))
        }
    }

    function performSearch(query: string, filter: SearchFilter = 'messages') {
        if (searchTimeout) {
            clearTimeout(searchTimeout)
        }

        const trimmed = query.trim()
        if (trimmed.length < 2) {
            update((state) => ({
                ...state,
                results: [],
                query: trimmed,
                filter,
                loading: false,
                error: '',
            }))
            return
        }

        searchTimeout = setTimeout(() => {
            doSearch(trimmed, filter)
        }, 300)
    }

    function clearSearch() {
        if (searchTimeout) {
            clearTimeout(searchTimeout)
            searchTimeout = null
        }
        set(initialState)
    }

    return {
        subscribe,
        performSearch,
        clearSearch,
    }
}

export const searchStore = createSearchStore()
