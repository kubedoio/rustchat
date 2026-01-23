import api from './client'

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

export interface CreateChannelRequest {
    team_id: string
    name: string
    display_name: string
    channel_type: ChannelType
    header?: string
    purpose?: string
    target_user_id?: string
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
}
