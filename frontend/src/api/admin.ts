import api from './client';

// Types
export interface ServerConfig {
    site: SiteConfig;
    authentication: AuthConfig;
    integrations: IntegrationsConfig;
    compliance: ComplianceConfig;
    email: EmailConfig;
    experimental: Record<string, boolean>;
}

export interface EmailConfig {
    smtp_host: string;
    smtp_port: number;
    smtp_username: string;
    smtp_password_encrypted: string;
    smtp_tls: boolean;
    from_address: string;
    from_name: string;
}

export interface SiteConfig {
    site_name: string;
    logo_url?: string;
    site_description: string;
    site_url: string;
    max_file_size_mb: number;
    max_simultaneous_connections: number;
    default_locale: string;
    default_timezone: string;
}

export interface AuthConfig {
    enable_email_password: boolean;
    enable_sso: boolean;
    require_sso: boolean;
    allow_registration: boolean;
    password_min_length: number;
    password_require_uppercase: boolean;
    password_require_number: boolean;
    password_require_symbol: boolean;
    session_length_hours: number;
}

export interface IntegrationsConfig {
    enable_webhooks: boolean;
    enable_slash_commands: boolean;
    enable_bots: boolean;
}

export interface ComplianceConfig {
    message_retention_days: number;
    file_retention_days: number;
}

export interface AdminUser {
    id: string;
    username: string;
    email: string;
    display_name: string | null;
    role: string;
    is_active: boolean;
    is_bot: boolean;
    last_login_at: string | null;
    created_at: string;
}

export interface AdminTeam {
    id: string;
    org_id: string;
    name: string;
    display_name: string | null;
    description: string | null;
    is_public: boolean;
    allow_open_invite: boolean;
    created_at: string;
    updated_at: string;
    members_count: number;
    channels_count: number;
}

export interface AdminChannel {
    id: string;
    team_id: string;
    channel_type: 'public' | 'private' | 'direct' | 'group';
    name: string;
    display_name: string | null;
    purpose: string | null;
    header: string | null;
    is_archived: boolean;
    creator_id: string | null;
    created_at: string;
    updated_at: string;
    members_count: number;
}

export interface AuditLog {
    id: string;
    actor_user_id: string | null;
    actor_ip: string | null;
    action: string;
    target_type: string;
    target_id: string | null;
    old_values: any;
    new_values: any;
    created_at: string;
}

export interface SystemStats {
    total_users: number;
    active_users: number;
    total_teams: number;
    total_channels: number;
    messages_24h: number;
    files_count: number;
    storage_used_mb: number;
}

export interface HealthStatus {
    status: 'healthy' | 'degraded' | 'unhealthy';
    database: { connected: boolean; latency_ms: number };
    storage: { connected: boolean; type: string };
    websocket: { active_connections: number };
    version: string;
    uptime_seconds: number;
}

export interface MiroTalkConfig {
    is_active: boolean;
    mode: 'disabled' | 'sfu' | 'p2p';
    base_url: string;
    api_key_secret: string;
    default_room_prefix?: string;
    join_behavior: 'embed_iframe' | 'new_tab';
}

export interface MiroTalkStats {
    peers?: number;
    rooms?: number;
    active_rooms?: string[];
    [key: string]: any;
}

export interface Permission {
    id: string;
    description: string | null;
    category: string | null;
}

// API functions
export const adminApi = {
    // Config
    getConfig: () => api.get<ServerConfig>('/admin/config'),
    updateConfig: (category: string, data: any) =>
        api.patch(`/admin/config/${category}`, data),

    // Users
    listUsers: (params?: {
        page?: number;
        per_page?: number;
        status?: 'active' | 'inactive' | 'all';
        role?: string;
        search?: string;
    }) => api.get<{ users: AdminUser[]; total: number }>('/admin/users', { params }),

    getUser: (id: string) => api.get<AdminUser>(`/admin/users/${id}`),
    createUser: (data: { username: string; email: string; password: string; role?: string; display_name?: string }) =>
        api.post<AdminUser>('/admin/users', data),
    updateUser: (id: string, data: { role?: string; display_name?: string }) =>
        api.patch<AdminUser>(`/admin/users/${id}`, data),
    deactivateUser: (id: string) => api.post(`/admin/users/${id}/deactivate`),
    reactivateUser: (id: string) => api.post(`/admin/users/${id}/reactivate`),
    resetPassword: (id: string) => api.post(`/admin/users/${id}/reset-password`),

    // Audit Logs
    listAuditLogs: (params?: {
        page?: number;
        per_page?: number;
        action?: string;
        target_type?: string;
        from_date?: string;
        to_date?: string;
    }) => api.get<AuditLog[]>('/admin/audit', { params }),

    // Stats & Health
    getStats: () => api.get<SystemStats>('/admin/stats'),
    getHealth: () => api.get<HealthStatus>('/admin/health'),

    // Permissions
    listPermissions: () => api.get<Permission[]>('/admin/permissions'),
    getRolePermissions: (role: string) => api.get<string[]>(`/admin/roles/${role}/permissions`),
    updateRolePermissions: (role: string, permissions: string[]) =>
        api.put<string[]>(`/admin/roles/${role}/permissions`, { permissions }),

    // Teams & Channels
    listTeams: (params?: {
        page?: number;
        per_page?: number;
        search?: string;
    }) => api.get<{ teams: AdminTeam[]; total: number }>('/admin/teams', { params }),

    getTeam: (id: string) => api.get<AdminTeam>(`/admin/teams/${id}`),
    deleteTeam: (id: string) => api.delete(`/admin/teams/${id}`),

    listChannels: (params?: {
        team_id?: string;
        page?: number;
        per_page?: number;
        search?: string;
    }) => api.get<{ channels: AdminChannel[]; total: number }>('/admin/channels', { params }),

    createChannel: (data: {
        team_id: string;
        name: string;
        display_name?: string;
        purpose?: string;
        channel_type: 'public' | 'private';
    }) => api.post<AdminChannel>('/admin/channels', data),

    updateChannel: (id: string, data: {
        display_name?: string;
        purpose?: string;
        header?: string;
    }) => api.patch<AdminChannel>(`/admin/channels/${id}`, data),

    deleteChannel: (id: string) => api.delete(`/admin/channels/${id}`),

    // Team Members
    listTeamMembers: (teamId: string) => api.get<any[]>(`/admin/teams/${teamId}/members`),
    addTeamMember: (teamId: string, userId: string, role?: string) => 
        api.post(`/admin/teams/${teamId}/members`, { user_id: userId, role }),
    removeTeamMember: (teamId: string, userId: string) => 
        api.delete(`/admin/teams/${teamId}/members/${userId}`),

    // Email
    testEmail: (to: string) => api.post('/admin/email/test', { to }),

    // Integrations - MiroTalk
    getMiroTalkConfig: () => api.get<MiroTalkConfig>('/admin/integrations/mirotalk'),
    updateMiroTalkConfig: (config: MiroTalkConfig) => api.put<MiroTalkConfig>('/admin/integrations/mirotalk', config),
    testMiroTalkConnection: () => api.post<MiroTalkStats>('/admin/integrations/mirotalk/test'),
};

export default adminApi;
