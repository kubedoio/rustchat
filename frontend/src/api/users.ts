import api from './client'

export interface User {
  id: string
  username: string
  email: string
  display_name?: string
  avatar_url?: string
  nickname?: string
  first_name?: string
  last_name?: string
  position?: string
  role: string
  presence: 'online' | 'away' | 'dnd' | 'offline'
  status_text?: string | null
  status_emoji?: string | null
  status_expires_at?: string | number | null
  created_at: string
}

export interface UpdateUserRequest {
  username?: string
  first_name?: string
  last_name?: string
  display_name?: string
  nickname?: string
  position?: string
  avatar_url?: string
}

export interface ChangePasswordRequest {
  new_password: string
}

export interface AuthConfig {
  enable_email_password: boolean
  enable_sso: boolean
  require_sso: boolean
  allow_registration: boolean
  password_min_length: number
  password_require_uppercase: boolean
  password_require_number: boolean
  password_require_symbol: boolean
  session_length_hours: number
}

export interface UserStatus {
  status?: 'online' | 'away' | 'dnd' | 'offline'
  presence?: 'online' | 'away' | 'dnd' | 'offline'
  text?: string
  emoji?: string
  expires_at?: string
}

export interface UpdateStatusRequest {
  status?: 'online' | 'away' | 'dnd' | 'offline'
  // Legacy compatibility field; backend should prefer `status`.
  presence?: 'online' | 'away' | 'dnd' | 'offline'
  text?: string
  emoji?: string
  duration?: string
  duration_minutes?: number
  dnd_end_time?: number
}

export interface UserStatusResponse {
  user_id: string
  status: string
  manual: boolean
  last_activity_at: number
}

export const usersApi = {
  list: (params?: { page?: number; per_page?: number; q?: string }) =>
    api.get<User[]>('/users', { params }),
  get: (id: string) => api.get<User>(`/users/${id}`),
  update: (id: string, data: UpdateUserRequest) => api.put<User>(`/users/${id}`, data),
  changePassword: (id: string, data: ChangePasswordRequest) =>
    api.post(`/users/${id}/password`, data),
  me: () => api.get<User>('/auth/me'),
  getAuthPolicy: () => api.get<AuthConfig>('/auth/policy'),
  getStatus: (userId: string) => api.get<UserStatus>(`/users/${userId}/status`),
  getMyStatus: () => api.get<UserStatus>('/users/me/status'),
  updateStatus: (data: UpdateStatusRequest) => api.put<UserStatus>('/users/me/status', data),
  clearStatus: () => api.delete<UserStatus>('/users/me/status'),
  getStatusesByIds: (userIds: string[]) =>
    api.post<UserStatusResponse[]>('/users/status/ids', userIds),
  getByIds: (userIds: string[]) => api.post<User[]>('/users/ids', userIds),
}
