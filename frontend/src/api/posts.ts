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
    props?: any
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
    seq: number | string // i64 can be number or string in JSON
    // Actually seq is i64, so it might exceed Number.MAX_SAFE_INTEGER if it grows very large.
    // But for now number is probably fine, or string if we want to be safe.
}

export interface ReadState {
    last_read_message_id: number | null
    first_unread_message_id: number | null
}

export interface PostListResponse {
    messages: Post[]
    read_state: ReadState | null
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
        api.get<PostListResponse>(`/channels/${channelId}/posts`, { params }),
    get: (id: string) => api.get<Post>(`/posts/${id}`),
    create: (data: CreatePostRequest) => api.post<Post>(`/channels/${data.channel_id}/posts`, data),
    update: (id: string, message: string) => api.put<Post>(`/posts/${id}`, { message }),
    delete: (id: string) => api.delete(`/posts/${id}`),
    getThread: (id: string) => api.get<Post[]>(`/posts/${id}/thread`),
    pin: (id: string) => api.post(`/posts/${id}/pin`),
    unpin: (id: string) => api.delete(`/posts/${id}/pin`),
    addReaction: (id: string, emoji: string) => api.post(`/posts/${id}/reactions`, { emoji_name: emoji }),
    removeReaction: (id: string, emoji: string) => api.delete(`/posts/${id}/reactions/${emoji}`),
    save: (id: string) => api.post(`/posts/${id}/save`),
    unsave: (id: string) => api.delete(`/posts/${id}/save`),
    getSaved: () => api.get<Post[]>('/active_user/saved_posts'),
}
