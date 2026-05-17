import api from './client';

// Types
export interface ServerConfig {
    site: SiteConfig;
    authentication: AuthConfig;
    integrations: IntegrationsConfig;
    compliance: ComplianceConfig;
    experimental: Record<string, unknown>;
}

// Email Provider (new provider-based system)
export interface MailProvider {
    id: string;
    provider_type: 'smtp' | 'ses' | 'sendgrid';
    host: string;
    port: number;
    username: string;
    has_password: boolean;
    tls_mode: 'starttls' | 'implicit_tls' | 'none';
    skip_cert_verify: boolean;
    from_address: string;
    from_name: string;
    reply_to: string | null;
    max_emails_per_minute: number;
    max_emails_per_hour: number;
    enabled: boolean;
    is_default: boolean;
    created_at: string;
    updated_at: string;
}

export interface CreateMailProviderRequest {
    provider_type: string;
    host: string;
    port: number;
    username: string;
    password: string;
    tls_mode: string;
    skip_cert_verify?: boolean;
    from_address: string;
    from_name: string;
    reply_to?: string;
    max_emails_per_minute?: number;
    max_emails_per_hour?: number;
    enabled?: boolean;
    is_default?: boolean;
}

export interface SiteConfig {
    site_name: string;
    logo_url?: string;
    site_description: string;
    site_url: string;
    about_link: string;
    help_link: string;
    terms_of_service_link: string;
    privacy_policy_link: string;
    report_a_problem_link: string;
    support_email: string;
    app_download_link: string;
    android_app_download_link: string;
    ios_app_download_link: string;
    custom_brand_text: string;
    custom_description_text: string;
    service_environment: string;
    max_file_size_mb: number;
    max_simultaneous_connections: number;
    enable_file: boolean;
    enable_user_statuses: boolean;
    enable_custom_emoji: boolean;
    enable_custom_brand: boolean;
    enable_mobile_file_download: boolean;
    enable_mobile_file_upload: boolean;
    allow_download_logs: boolean;
    diagnostics_enabled: boolean;
    default_locale: string;
    default_timezone: string;
    post_edit_time_limit_seconds: number;
}

export interface AuthConfig {
    enable_email_password: boolean;
    enable_sso: boolean;
    require_sso: boolean;
    allow_registration: boolean;
    enable_sign_in_with_email: boolean;
    enable_sign_in_with_username: boolean;
    enable_sign_up_with_email: boolean;
    enable_sign_up_with_gitlab: boolean;
    enable_sign_up_with_google: boolean;
    enable_sign_up_with_office365: boolean;
    enable_sign_up_with_openid: boolean;
    enable_user_creation: boolean;
    enable_open_server: boolean;
    enable_guest_accounts: boolean;
    enable_multifactor_authentication: boolean;
    enforce_multifactor_authentication: boolean;
    enable_saml: boolean;
    enable_ldap: boolean;
    password_min_length: number;
    password_require_lowercase: boolean;
    password_require_uppercase: boolean;
    password_require_number: boolean;
    password_require_symbol: boolean;
    password_enable_forgot_link: boolean;
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
    deleted_at?: string | null;
    deleted_by?: string | null;
    delete_reason?: string | null;
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
    redis: { connected: boolean; latency_ms: number };
    disk: { connected: boolean; used_percent: number; available_mb: number };
    websocket: { active_connections: number };
    version: string;
    uptime_seconds: number;
}

// Calls Plugin Configuration
export interface CallsPluginConfig {
    enabled: boolean;
    turn_server_enabled: boolean;
    turn_server_url: string;
    turn_server_username: string;
    turn_server_credential: string | null;
    udp_port: number;
    tcp_port: number;
    ice_host_override: string | null;
    stun_servers: string[];
}

export interface CallsPluginConfigResponse {
    plugin_id: string;
    plugin_name: string;
    settings: CallsPluginConfig;
}

export interface Permission {
    id: string;
    description: string | null;
    category: string | null;
}

// SSO Configuration
export interface SsoConfig {
    id: string;
    org_id: string;
    provider_key: string;
    provider_type: 'github' | 'google' | 'oidc' | 'saml';
    display_name: string | null;
    issuer_url: string | null;
    client_id: string | null;
    scopes: string[];
    is_active: boolean;
    auto_provision: boolean;
    default_role: string | null;
    allow_domains: string[] | null;
    github_org: string | null;
    github_team: string | null;
    groups_claim: string | null;
    role_mappings: Record<string, string> | null;
    created_at: string;
    updated_at: string;
}

export interface SsoProviderInfo {
    id: string;
    provider_key: string;
    provider_type: string;
    display_name: string;
    login_url: string;
}

// Groups for membership policies
export interface AdminGroup {
    id: string;
    name: string | null;
    display_name: string;
    description: string;
    source: string;
    remote_id: string | null;
    member_count: number;
}

export interface SsoTestResult {
    success: boolean;
    message: string;
    details: Record<string, any> | null;
}

export interface CreateSsoConfigRequest {
    provider_key: string;
    provider_type: 'github' | 'google' | 'oidc';
    display_name?: string;
    issuer_url?: string;
    client_id?: string;
    client_secret?: string;
    scopes?: string[];
    is_active?: boolean;
    auto_provision?: boolean;
    default_role?: string;
    allow_domains?: string[];
    github_org?: string;
    github_team?: string;
    groups_claim?: string;
    role_mappings?: Record<string, string>;
}

export interface UpdateSsoConfigRequest {
    provider_key?: string;
    display_name?: string;
    issuer_url?: string;
    client_id?: string;
    client_secret?: string;
    scopes?: string[];
    is_active?: boolean;
    auto_provision?: boolean;
    default_role?: string;
    allow_domains?: string[];
    github_org?: string;
    github_team?: string;
    groups_claim?: string;
    role_mappings?: Record<string, string>;
}

// API functions
export const adminApi = {
    // Config
    getConfig: () => api.get<ServerConfig>('/admin/config'),
    updateConfig: (category: string, data: Record<string, unknown>) =>
        api.patch(`/admin/config/${category}`, data),

    // Users
    listUsers: (params?: {
        page?: number;
        per_page?: number;
        status?: 'active' | 'inactive' | 'all';
        role?: string;
        search?: string;
        include_deleted?: boolean;
    }) => api.get<{ users: AdminUser[]; total: number }>('/admin/users', { params }),

    getUser: (id: string) => api.get<AdminUser>(`/admin/users/${id}`),
    createUser: (data: { username: string; email: string; password: string; role?: string; display_name?: string }) =>
        api.post<AdminUser>('/admin/users', data),
    updateUser: (id: string, data: { role?: string; display_name?: string }) =>
        api.patch<AdminUser>(`/admin/users/${id}`, data),
    deactivateUser: (id: string) => api.post(`/admin/users/${id}/deactivate`),
    reactivateUser: (id: string) => api.post(`/admin/users/${id}/reactivate`),
    deleteUser: (id: string, data: { confirm: string; reason?: string }) =>
        api.delete(`/admin/users/${id}`, { data }),
    wipeUser: (id: string) => api.post(`/admin/users/${id}/wipe`),
    resetPassword: (id: string) => api.post(`/admin/users/${id}/reset-password`),
    setUserPassword: (id: string, password: string) => api.post(`/admin/users/${id}/set-password`, { password }),

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
    createTeam: (data: { name: string; display_name?: string; description?: string; is_public?: boolean; allow_open_invite?: boolean }) =>
        api.post<AdminTeam>('/admin/teams', data),
    updateTeam: (id: string, data: { display_name?: string; description?: string; is_public?: boolean; allow_open_invite?: boolean }) =>
        api.patch<AdminTeam>(`/admin/teams/${id}`, data),
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
    updateTeamMemberRole: (teamId: string, userId: string, role: string) =>
        api.patch(`/admin/teams/${teamId}/members/${userId}`, { role }),
    
    // Groups (for membership policies)
    listGroups: (params?: { source?: string; search?: string }) => 
        api.get<AdminGroup[]>('/admin/groups', { params }),
    
    // All teams (for membership policy targets - requires TEAM_MANAGE permission)
    listAllTeams: () => api.get<{ id: string; name: string; display_name: string }[]>('/teams/all'),

    // Email Providers
    listMailProviders: () => api.get<MailProvider[]>('/admin/email/providers'),
    getMailProvider: (id: string) => api.get<MailProvider>(`/admin/email/providers/${id}`),
    createMailProvider: (data: CreateMailProviderRequest) => api.post<MailProvider>('/admin/email/providers', data),
    updateMailProvider: (id: string, data: Partial<CreateMailProviderRequest>) => api.put<MailProvider>(`/admin/email/providers/${id}`, data),
    deleteMailProvider: (id: string) => api.delete(`/admin/email/providers/${id}`),
    testMailProvider: (id: string, toEmail: string) => api.post(`/admin/email/providers/${id}/test`, { to_email: toEmail }),
    setDefaultMailProvider: (id: string) => api.post(`/admin/email/providers/${id}/default`),
    
    // Legacy email test (uses default provider)
    testEmail: (to: string) => api.post('/admin/email/test', { to }),

    // Calls Plugin
    getCallsPluginConfig: () => api.get<CallsPluginConfigResponse>('/admin/plugins/calls'),
    updateCallsPluginConfig: (config: CallsPluginConfig) => api.put<CallsPluginConfigResponse>('/admin/plugins/calls', config),

    // SSO Configuration
    getSsoConfigs: () => api.get<SsoConfig[]>('/admin/sso'),
    getSsoConfig: (id: string) => api.get<SsoConfig>(`/admin/sso/${id}`),
    createSsoConfig: (data: CreateSsoConfigRequest) => api.post<SsoConfig>('/admin/sso', data),
    updateSsoConfig: (id: string, data: UpdateSsoConfigRequest) => api.put<SsoConfig>(`/admin/sso/${id}`, data),
    deleteSsoConfig: (id: string) => api.delete(`/admin/sso/${id}`),
    testSsoConfig: (id: string) => api.post<SsoTestResult>(`/admin/sso/${id}/test`),
};

export default adminApi;
