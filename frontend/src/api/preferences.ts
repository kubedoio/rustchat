import api, { v4Api } from './client'

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
  // Display settings
  collapsed_reply_threads?: boolean
  use_military_time?: boolean
  teammate_name_display?: 'username' | 'nickname' | 'full_name'
  availability_status_visible?: boolean
  show_last_active_time?: boolean
  timezone?: string
  link_previews_enabled?: boolean
  image_previews_enabled?: boolean
  click_to_reply?: boolean
  channel_display_mode?: 'full' | 'centered'
  quick_reactions_enabled?: boolean
  emoji_picker_enabled?: boolean
  language?: string
  // Sidebar settings
  group_unread_channels?: 'never' | 'only_for_favorites' | 'always'
  limit_visible_dms_gms?: 'all' | '10' | '20' | '40'
  // Advanced settings
  send_on_ctrl_enter?: boolean
  enable_post_formatting?: boolean
  enable_join_leave_messages?: boolean
  enable_performance_debugging?: boolean
  unread_scroll_position?: 'start' | 'last' | 'end'
  sync_drafts?: boolean
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
  // Display settings
  collapsed_reply_threads?: boolean
  use_military_time?: boolean
  teammate_name_display?: 'username' | 'nickname' | 'full_name'
  availability_status_visible?: boolean
  show_last_active_time?: boolean
  timezone?: string
  link_previews_enabled?: boolean
  image_previews_enabled?: boolean
  click_to_reply?: boolean
  channel_display_mode?: 'full' | 'centered'
  quick_reactions_enabled?: boolean
  emoji_picker_enabled?: boolean
  language?: string
  // Sidebar settings
  group_unread_channels?: 'never' | 'only_for_favorites' | 'always'
  limit_visible_dms_gms?: 'all' | '10' | '20' | '40'
  // Advanced settings
  send_on_ctrl_enter?: boolean
  enable_post_formatting?: boolean
  enable_join_leave_messages?: boolean
  enable_performance_debugging?: boolean
  unread_scroll_position?: 'start' | 'last' | 'end'
  sync_drafts?: boolean
}

export interface ChannelNotificationSetting {
  id: string
  user_id: string
  channel_id: string
  notify_level: string
  is_muted: boolean
  mute_until: string | null
}

// Mattermost-compatible preference for favorites, etc.
export interface Preference {
  user_id: string
  category: string
  name: string
  value: string
}

export const preferencesApi = {
  // User status
  getMyStatus: () => api.get<UserStatus>('/users/me/status'),
  updateMyStatus: (data: UpdateStatusRequest) => api.put<UserStatus>('/users/me/status', data),
  clearMyStatus: () => api.delete<UserStatus>('/users/me/status'),
  getUserStatus: (userId: string) => api.get<UserStatus>(`/users/${userId}/status`),

  // User preferences
  getMyPreferences: () => api.get<UserPreferences>('/users/me/preferences'),
  updateMyPreferences: (data: UpdatePreferencesRequest) =>
    api.put<UserPreferences>('/users/me/preferences', data),

  // Status presets
  listStatusPresets: () => api.get<StatusPreset[]>('/users/me/status/presets'),
  createStatusPreset: (data: { emoji: string; text: string; duration_minutes?: number }) =>
    api.post<StatusPreset>('/users/me/status/presets', data),
  deleteStatusPreset: (presetId: string) => api.delete(`/users/me/status/presets/${presetId}`),

  // Channel notifications
  getChannelNotifications: (channelId: string) =>
    api.get<ChannelNotificationSetting | null>(`/channels/${channelId}/notifications`),
  updateChannelNotifications: (
    channelId: string,
    data: { notify_level?: string; is_muted?: boolean; mute_until?: string }
  ) => api.put<ChannelNotificationSetting>(`/channels/${channelId}/notifications`, data),

  // Mattermost-compatible preferences (for favorites)
  getMyPreferencesMm: () => api.get<Preference[]>('/users/me/preferences'),
  getMyPreferencesMmV4: () => v4Api.get<Preference[]>('/users/me/preferences'),
  getPreferencesByCategory: (userId: string, category: string) =>
    api.get<Preference[]>(`/users/${userId}/preferences/${category}`),
  updatePreferences: (userId: string, preferences: Preference[]) =>
    api.put(`/users/${userId}/preferences`, preferences),
  updatePreferencesV4: (userId: string, preferences: Preference[]) =>
    v4Api.put(`/users/${userId}/preferences`, preferences),
  deletePreferences: (userId: string, preferences: Preference[]) =>
    api.delete(`/users/${userId}/preferences`, { data: preferences }),
}
