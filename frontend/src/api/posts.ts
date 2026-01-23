import api from './client'
import type { FileUploadResponse } from './files'

export interface Post {
    id: string
    channel_id: string
    user_id: string
    message: string
    root_post_id?: string
    parent_id?: string
    created_at: string
    updated_at: string
    is_pinned: boolean
    // Populated fields
    username?: string
    avatar_url?: string
    email?: string
    reply_count?: number
    last_reply_at?: string
    files?: FileUploadResponse[]
    reactions?: { emoji: string; count: number; users: string[] }[]
    is_saved?: boolean
    client_msg_id?: string
}

export interface CreatePostRequest {
    channel_id: string
    message: string
    root_post_id?: string
    parent_id?: string
}

export interface Reaction {
    post_id: string
    user_id: string
    emoji: string
}

export const postsApi = {
    list: (channelId: string, params?: { before?: string; limit?: number; is_pinned?: boolean; q?: string }) =>
        api.get<Post[]>(`/channels/${channelId}/posts`, { params }),
    get: (id: string) => api.get<Post>(`/posts/${id}`),
    create: (data: CreatePostRequest) => api.post<Post>(`/channels/${data.channel_id}/posts`, data),
    update: (id: string, message: string) => api.put<Post>(`/posts/${id}`, { message }),
    delete: (id: string) => api.delete(`/posts/${id}`),
    getThread: (id: string) => api.get<Post[]>(`/posts/${id}/thread`),
    pin: (id: string) => api.post(`/posts/${id}/pin`),
    unpin: (id: string) => api.delete(`/posts/${id}/pin`),
    addReaction: (id: string, emoji: string) => api.post(`/posts/${id}/reactions`, { emoji }),
    save: (id: string) => api.post(`/posts/${id}/save`),
    unsave: (id: string) => api.delete(`/posts/${id}/save`),
    getSaved: () => api.get<Post[]>('/active_user/saved_posts'),
}
