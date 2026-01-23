import api from './client'

export interface User {
    id: string
    username: string
    email: string
    display_name?: string
    avatar_url?: string
    role: string
    presence: 'online' | 'away' | 'dnd' | 'offline'
    created_at: string
}

export interface UpdateUserRequest {
    username?: string
    display_name?: string
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
    presence?: 'online' | 'away' | 'dnd' | 'offline'
    text?: string
    emoji?: string
    expires_at?: string
}

export interface UpdateStatusRequest {
    presence?: 'online' | 'away' | 'dnd' | 'offline'
    text?: string
    emoji?: string
    duration_minutes?: number
}

export const usersApi = {
    list: (params?: { page?: number; per_page?: number }) =>
        api.get<User[]>('/users', { params }),
    get: (id: string) => api.get<User>(`/users/${id}`),
    update: (id: string, data: UpdateUserRequest) => api.put<User>(`/users/${id}`, data),
    changePassword: (id: string, data: ChangePasswordRequest) => api.post(`/users/${id}/password`, data),
    me: () => api.get<User>('/auth/me'),
    getAuthPolicy: () => api.get<AuthConfig>('/auth/policy'),
    getStatus: (userId: string) => api.get<UserStatus>(`/users/${userId}/status`),
    getMyStatus: () => api.get<UserStatus>('/users/me/status'),
    updateStatus: (data: UpdateStatusRequest) => api.put<UserStatus>('/users/me/status', data),
    clearStatus: () => api.delete<UserStatus>('/users/me/status'),
}
