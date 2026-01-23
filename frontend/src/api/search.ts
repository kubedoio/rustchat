import api from './client'
import type { Post } from './posts'

export interface SearchResult {
    posts: Post[]
    total: number
    page: number
    per_page: number
}

export interface SearchParams {
    q: string
    channel_id?: string
    page?: number
    per_page?: number
}

export const searchApi = {
    search: (params: SearchParams) => api.get<SearchResult>('/search', { params }),
}
