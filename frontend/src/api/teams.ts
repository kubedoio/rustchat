import api from './client'

export interface Team {
    id: string
    name: string
    display_name: string
    description?: string
    invite_id?: string
    is_public?: boolean
    allow_open_invite?: boolean
    created_at: string
}

export interface TeamMember {
    team_id: string
    user_id: string
    role: string
    username: string
    display_name?: string
    avatar_url?: string
    presence?: 'online' | 'away' | 'dnd' | 'offline'
    created_at: string
}

export interface CreateTeamRequest {
    name: string
    display_name: string
    description?: string
    is_public?: boolean
    allow_open_invite?: boolean
}

export const teamsApi = {
    list: () => api.get<Team[]>('/teams'),
    listPublic: () => api.get<Team[]>('/teams/public'),
    get: (id: string) => api.get<Team>(`/teams/${id}`),
    create: (data: CreateTeamRequest) => api.post<Team>('/teams', data),
    update: (id: string, data: Partial<CreateTeamRequest>) => api.put<Team>(`/teams/${id}`, data),
    delete: (id: string) => api.delete(`/teams/${id}`),
    join: (id: string) => api.post<TeamMember>(`/teams/${id}/join`),
    leave: (id: string) => api.post(`/teams/${id}/leave`),
    getMembers: (id: string) => api.get(`/teams/${id}/members`),
    addMember: (teamId: string, userId: string) => api.post(`/teams/${teamId}/members`, { user_id: userId }),
    removeMember: (teamId: string, userId: string) => api.delete(`/teams/${teamId}/members/${userId}`),
    getChannels: (teamId: string) => api.get(`/teams/${teamId}/channels`),
}

