import api from './client'

export interface UserStatus {
    text: string | null
    emoji: string | null
    expires_at: string | null
}

export interface UpdateStatusRequest {
    text?: string
    emoji?: string
    duration_minutes?: number
}

export interface StatusPreset {
    id: string
    user_id: string | null
    emoji: string
    text: string
    duration_minutes: number | null
    is_default: boolean
    sort_order: number
}

export interface UserPreferences {
    user_id: string
    notify_desktop: string
    notify_push: string
    notify_email: string
    notify_sounds: boolean
    dnd_enabled: boolean
    dnd_start_time: string | null
    dnd_end_time: string | null
    dnd_days: string
    message_display: string
    sidebar_behavior: string
    time_format: string
    mention_keywords: string[] | null
}

export interface UpdatePreferencesRequest {
    notify_desktop?: string
    notify_push?: string
    notify_email?: string
    notify_sounds?: boolean
    dnd_enabled?: boolean
    dnd_start_time?: string
    dnd_end_time?: string
    dnd_days?: string
    message_display?: string
    sidebar_behavior?: string
    time_format?: string
    mention_keywords?: string[]
}

export interface ChannelNotificationSetting {
    id: string
    user_id: string
    channel_id: string
    notify_level: string
    is_muted: boolean
    mute_until: string | null
}

export const preferencesApi = {
    // User status
    getMyStatus: () => api.get<UserStatus>('/users/me/status'),
    updateMyStatus: (data: UpdateStatusRequest) => api.put<UserStatus>('/users/me/status', data),
    clearMyStatus: () => api.delete<UserStatus>('/users/me/status'),
    getUserStatus: (userId: string) => api.get<UserStatus>(`/users/${userId}/status`),

    // User preferences
    getMyPreferences: () => api.get<UserPreferences>('/users/me/preferences'),
    updateMyPreferences: (data: UpdatePreferencesRequest) => api.put<UserPreferences>('/users/me/preferences', data),

    // Status presets
    listStatusPresets: () => api.get<StatusPreset[]>('/users/me/status/presets'),
    createStatusPreset: (data: { emoji: string; text: string; duration_minutes?: number }) =>
        api.post<StatusPreset>('/users/me/status/presets', data),
    deleteStatusPreset: (presetId: string) => api.delete(`/users/me/status/presets/${presetId}`),

    // Channel notifications
    getChannelNotifications: (channelId: string) =>
        api.get<ChannelNotificationSetting | null>(`/channels/${channelId}/notifications`),
    updateChannelNotifications: (channelId: string, data: { notify_level?: string; is_muted?: boolean; mute_until?: string }) =>
        api.put<ChannelNotificationSetting>(`/channels/${channelId}/notifications`, data),
}
