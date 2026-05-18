import api from './client'
import { v4Api } from './client'

export type ChannelType = 'public' | 'private' | 'direct' | 'group'

export interface Channel {
    id: string
    team_id: string
    name: string
    display_name: string
    channel_type: ChannelType
    header?: string
    purpose?: string
    unreadCount?: number
    mentionCount?: number
    created_at: string
    creator_id: string
}

export interface ChannelMember {
    channel_id: string
    user_id: string
    roles: string
    last_viewed_at: number
    msg_count: number
    mention_count: number
    mention_count_root: number
    urgent_mention_count: number
    msg_count_root: number
    notify_props: ChannelNotifyProps
    last_update_at: number
}

export interface ChannelNotifyProps {
    desktop?: string
    mobile?: string
    mark_unread?: string
    ignore_channel_mentions?: string
}

export interface CreateChannelRequest {
    team_id: string
    name: string
    display_name: string
    channel_type: ChannelType
    header?: string
    purpose?: string
    target_user_id?: string
}

export interface SidebarCategory {
    id: string
    team_id: string
    user_id: string
    type: string
    display_name: string
    sorting: string
    muted: boolean
    collapsed: boolean
    channel_ids: string[]
    sort_order: number
    create_at: number
    update_at: number
    delete_at: number
}

export interface SidebarCategories {
    categories: SidebarCategory[]
    order: string[]
}

export interface ApiStatusResponse {
    status: string
}

export const channelsApi = {
    list: (teamId: string) => api.get<Channel[]>('/channels', { params: { team_id: teamId } }),
    listJoinable: (teamId: string) => api.get<Channel[]>('/channels', { params: { team_id: teamId, available_to_join: true } }),
    get: (id: string) => api.get<Channel>(`/channels/${id}`),
    create: (data: CreateChannelRequest) => api.post<Channel>('/channels', data),
    update: (id: string, data: Partial<CreateChannelRequest>) => api.put<Channel>(`/channels/${id}`, data),
    delete: (id: string) => api.delete(`/channels/${id}`),
    join: (id: string, userId: string) => api.post(`/channels/${id}/members`, { user_id: userId }),
    leave: (id: string) => api.delete(`/channels/${id}/members/me`),
    removeMember: (channelId: string, userId: string) => api.delete(`/channels/${channelId}/members/${userId}`),
    getMembers: (id: string) => api.get(`/channels/${id}/members`),
    getMember: (channelId: string, userId: string) => api.get<ChannelMember>(`/channels/${channelId}/members/${userId}`),
    getUnreadCounts: () => api.get<{ channel_id: string, count: number }[]>('/channels/unreads'),
    
    // Mark as Read / Mark as Unread - MM-compatible endpoints
    markAsRead: (channelId: string, userId: string = 'me') => 
        api.post(`/channels/${channelId}/members/${userId}/read`, {}),
    markAsUnread: (channelId: string, userId: string = 'me') => 
        api.post(`/channels/${channelId}/members/${userId}/set_unread`, {}),
    
    // Notify props for mute/unmute
    updateNotifyProps: (channelId: string, userId: string, props: ChannelNotifyProps) =>
        api.put<ApiStatusResponse>(`/channels/${channelId}/members/${userId}/notify_props`, props),
    
    // Add member to channel
    addMember: (channelId: string, userId: string) =>
        api.post<ChannelMember>(`/channels/${channelId}/members`, { user_id: userId }),
}

// Sidebar categories API (v4 Mattermost-compatible)
export const categoriesApi = {
    getCategories: (userId: string, teamId: string) =>
        v4Api.get<SidebarCategories>(`/users/${userId}/teams/${teamId}/channels/categories`),

    getCategoriesOrder: (userId: string, teamId: string) =>
        v4Api.get<string[]>(`/users/${userId}/teams/${teamId}/channels/categories/order`),
    
    updateCategories: (userId: string, teamId: string, categories: SidebarCategory[]) =>
        v4Api.put<SidebarCategory[]>(`/users/${userId}/teams/${teamId}/channels/categories`, categories),
    
    updateCategory: (userId: string, teamId: string, categoryId: string, category: Partial<SidebarCategory>) =>
        v4Api.put<SidebarCategory>(`/users/${userId}/teams/${teamId}/channels/categories/${categoryId}`, category),
}
