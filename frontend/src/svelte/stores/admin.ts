import { writable } from 'svelte/store'
import { svelteApi } from './http'
import type {
    AdminChannel,
    AdminTeam,
    AdminUser,
    AuditLog,
    HealthStatus,
    MailProvider,
    Permission,
    ServerConfig,
    SsoConfig,
    SystemStats,
    CallsPluginConfig,
    CallsPluginConfigResponse,
} from '../../api/admin'
import type {
    AutoMembershipPolicyAudit,
    ListPoliciesQuery,
    PolicyWithTargets,
    CreatePolicyRequest,
    UpdatePolicyRequest,
} from '../../api/membershipPolicies'

export type {
    AdminChannel,
    AdminTeam,
    AdminUser,
    AuditLog,
    HealthStatus,
    MailProvider,
    Permission,
    ServerConfig,
    SsoConfig,
    SystemStats,
    CallsPluginConfig,
    CallsPluginConfigResponse,
    AutoMembershipPolicyAudit,
    PolicyWithTargets,
}

interface AdminList<T> {
    items: T[]
    total: number
}

interface AdminState {
    loading: boolean
    error: string | null
    stats: SystemStats | null
    health: HealthStatus | null
    config: ServerConfig | null
    users: AdminList<AdminUser>
    teams: AdminList<AdminTeam>
    channels: AdminList<AdminChannel>
    auditLogs: AuditLog[]
    permissions: Permission[]
    mailProviders: MailProvider[]
    ssoConfigs: SsoConfig[]
    membershipPolicies: PolicyWithTargets[]
    membershipAuditLogs: MembershipAuditLog[]
    membershipAuditSummary: MembershipAuditSummary | null
    membershipRecentFailures: MembershipAuditFailure[]
    membershipPolicyFailureStats: MembershipPolicyFailureStat[]
    callsPluginConfig: CallsPluginConfig | null
    terms: TermsOfService[]
    termsStats: TermsStats | null
}

export interface MembershipAuditSummary {
    total_operations_24h: number
    successful_operations_24h: number
    failed_operations_24h: number
    failure_rate_24h: number
    pending_operations: number
    policies_with_failures: number
}

export interface MembershipAuditLog {
    id: string
    policy_id: string | null
    policy_name?: string | null
    user_id: string
    username?: string | null
    target_type: string
    target_id: string
    action: string
    status: 'success' | 'failed' | 'pending'
    error_message?: string | null
    created_at: string
}

export interface MembershipAuditFailure extends MembershipAuditLog {
    policy_name?: string | null
}

export interface MembershipPolicyFailureStat {
    policy_id: string
    policy_name: string
    total_operations: number
    failed_operations: number
    failure_rate: number
    last_error_message: string | null
}

export interface TermsOfService {
    id: string
    version: string
    title: string
    content: string
    summary: string | null
    is_active: boolean
    effective_date: string
    created_at: string
}

export interface TermsStats {
    total_users: number
    accepted_count: number
    pending_count: number
    acceptance_rate: number
    pending_users?: Array<{
        id: string
        username: string
        email: string
        display_name: string | null
        created_at: string
    }>
}

const initialState: AdminState = {
    loading: false,
    error: null,
    stats: null,
    health: null,
    config: null,
    users: { items: [], total: 0 },
    teams: { items: [], total: 0 },
    channels: { items: [], total: 0 },
    auditLogs: [],
    permissions: [],
    mailProviders: [],
    ssoConfigs: [],
    membershipPolicies: [],
    membershipAuditLogs: [],
    membershipAuditSummary: null,
    membershipRecentFailures: [],
    membershipPolicyFailureStats: [],
    callsPluginConfig: null,
    terms: [],
    termsStats: null,
}

function withQuery(path: string, params?: Record<string, string | number | boolean | undefined>): string {
    if (!params) {
        return path
    }

    const search = new URLSearchParams()
    for (const [key, value] of Object.entries(params)) {
        if (value !== undefined && value !== '') {
            search.set(key, String(value))
        }
    }

    const query = search.toString()
    return query ? `${path}?${query}` : path
}

function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : 'Failed to load admin data'
}

function createAdminStore() {
    const { subscribe, set, update } = writable<AdminState>(initialState)

    async function load<T>(task: () => Promise<T>): Promise<T | null> {
        update((state) => ({ ...state, loading: true, error: null }))
        try {
            const result = await task()
            update((state) => ({ ...state, loading: false, error: null }))
            return result
        } catch (error) {
            update((state) => ({ ...state, loading: false, error: errorMessage(error) }))
            return null
        }
    }

    return {
        subscribe,
        reset(): void {
            set(initialState)
        },
        async fetchOverview(): Promise<void> {
            await load(async () => {
                const [stats, health] = await Promise.all([
                    svelteApi.get<SystemStats>('/admin/stats'),
                    svelteApi.get<HealthStatus>('/admin/health'),
                ])
                update((state) => ({ ...state, stats: stats.data, health: health.data }))
            })
        },
        async fetchHealth(): Promise<void> {
            const response = await load(() => svelteApi.get<HealthStatus>('/admin/health'))
            if (response) {
                update((state) => ({ ...state, health: response.data }))
            }
        },
        async fetchConfig(): Promise<void> {
            const response = await load(() => svelteApi.get<ServerConfig>('/admin/config'))
            if (response) {
                update((state) => ({ ...state, config: response.data }))
            }
        },
        async updateConfig(category: string, data: unknown): Promise<boolean> {
            const response = await load(() => svelteApi.patch<unknown>(`/admin/config/${category}`, data))
            if (response) {
                await this.fetchConfig()
                return true
            }
            return false
        },
        async fetchUsers(params?: { page?: number; per_page?: number; search?: string; status?: string }): Promise<void> {
            const response = await load(() =>
                svelteApi.get<{ users: AdminUser[]; total: number }>(withQuery('/admin/users', params)),
            )
            if (response) {
                update((state) => ({
                    ...state,
                    users: { items: response.data.users ?? [], total: response.data.total ?? 0 },
                }))
            }
        },
        async fetchTeams(params?: { page?: number; per_page?: number; search?: string }): Promise<void> {
            const response = await load(() =>
                svelteApi.get<{ teams: AdminTeam[]; total: number }>(withQuery('/admin/teams', params)),
            )
            if (response) {
                update((state) => ({
                    ...state,
                    teams: { items: response.data.teams ?? [], total: response.data.total ?? 0 },
                }))
            }
        },
        async fetchChannels(params?: { page?: number; per_page?: number; search?: string; team_id?: string }): Promise<void> {
            const response = await load(() =>
                svelteApi.get<{ channels: AdminChannel[]; total: number }>(withQuery('/admin/channels', params)),
            )
            if (response) {
                update((state) => ({
                    ...state,
                    channels: { items: response.data.channels ?? [], total: response.data.total ?? 0 },
                }))
            }
        },
        async fetchAuditLogs(params?: { page?: number; per_page?: number; action?: string; target_type?: string }): Promise<void> {
            const response = await load(() => svelteApi.get<AuditLog[]>(withQuery('/admin/audit', params)))
            if (response) {
                update((state) => ({ ...state, auditLogs: response.data ?? [] }))
            }
        },
        async fetchPermissions(): Promise<void> {
            const response = await load(() => svelteApi.get<Permission[]>('/admin/permissions'))
            if (response) {
                update((state) => ({ ...state, permissions: response.data ?? [] }))
            }
        },
        async fetchMailProviders(): Promise<void> {
            const response = await load(() => svelteApi.get<MailProvider[]>('/admin/email/providers'))
            if (response) {
                update((state) => ({ ...state, mailProviders: response.data ?? [] }))
            }
        },
        async fetchSsoConfigs(): Promise<void> {
            const response = await load(() => svelteApi.get<SsoConfig[]>('/admin/sso'))
            if (response) {
                update((state) => ({ ...state, ssoConfigs: response.data ?? [] }))
            }
        },
        async fetchMembershipPolicies(query?: ListPoliciesQuery): Promise<void> {
            const response = await load(() =>
                svelteApi.get<PolicyWithTargets[]>(withQuery('/admin/membership-policies', query ? { ...query } : undefined)),
            )
            if (response) {
                update((state) => ({ ...state, membershipPolicies: response.data ?? [] }))
            }
        },
        async createMembershipPolicy(data: CreatePolicyRequest): Promise<PolicyWithTargets | null> {
            const response = await load(() =>
                svelteApi.post<PolicyWithTargets>('/admin/membership-policies', data),
            )
            if (response) {
                update((state) => ({ ...state, membershipPolicies: [...state.membershipPolicies, response.data] }))
                return response.data
            }
            return null
        },
        async updateMembershipPolicy(id: string, data: UpdatePolicyRequest): Promise<PolicyWithTargets | null> {
            const response = await load(() =>
                svelteApi.put<PolicyWithTargets>(`/admin/membership-policies/${id}`, data),
            )
            if (response) {
                update((state) => ({
                    ...state,
                    membershipPolicies: state.membershipPolicies.map((policy) =>
                        policy.id === id ? response.data : policy,
                    ),
                }))
                return response.data
            }
            return null
        },
        async deleteMembershipPolicy(id: string): Promise<boolean> {
            const response = await load(() => svelteApi.delete<unknown>(`/admin/membership-policies/${id}`))
            if (response) {
                update((state) => ({
                    ...state,
                    membershipPolicies: state.membershipPolicies.filter((policy) => policy.id !== id),
                }))
                return true
            }
            return false
        },
        async fetchPolicyAudit(id: string): Promise<AutoMembershipPolicyAudit[]> {
            const response = await load(() =>
                svelteApi.get<AutoMembershipPolicyAudit[]>(withQuery(`/admin/membership-policies/${id}/audit`, { limit: 50 })),
            )
            return response?.data ?? []
        },
        async fetchMembershipAuditDashboard(params?: {
            status?: string
            action?: string
            from_date?: string
            to_date?: string
        }): Promise<void> {
            await load(async () => {
                const query = {
                    status: params?.status,
                    action: params?.action,
                    from_date: params?.from_date ? new Date(params.from_date).toISOString() : undefined,
                    to_date: params?.to_date ? new Date(params.to_date).toISOString() : undefined,
                }
                const [summary, recentFailures, policyStats, logs] = await Promise.all([
                    svelteApi.get<MembershipAuditSummary>('/admin/audit/membership/summary'),
                    svelteApi.get<MembershipAuditFailure[]>('/admin/audit/membership/recent-failures'),
                    svelteApi.get<MembershipPolicyFailureStat[]>('/admin/audit/membership/failures'),
                    svelteApi.get<MembershipAuditLog[]>(withQuery('/admin/audit/membership', query)),
                ])
                update((state) => ({
                    ...state,
                    membershipAuditSummary: summary.data,
                    membershipRecentFailures: recentFailures.data ?? [],
                    membershipPolicyFailureStats: policyStats.data ?? [],
                    membershipAuditLogs: logs.data ?? [],
                }))
            })
        },
        async startComplianceExport(): Promise<boolean> {
            const response = await load(() => svelteApi.post<unknown>('/admin/compliance/export'))
            return Boolean(response)
        },
        async fetchCallsPluginConfig(): Promise<void> {
            const response = await load(() => svelteApi.get<CallsPluginConfigResponse>('/admin/plugins/calls'))
            if (response) {
                update((state) => ({ ...state, callsPluginConfig: response.data.settings }))
            }
        },
        async updateCallsPluginConfig(config: CallsPluginConfig): Promise<boolean> {
            const response = await load(() =>
                svelteApi.put<CallsPluginConfigResponse>('/admin/plugins/calls', config),
            )
            if (response) {
                update((state) => ({ ...state, callsPluginConfig: response.data.settings }))
                return true
            }
            return false
        },
        async fetchTerms(): Promise<void> {
            await load(async () => {
                const [terms, stats] = await Promise.all([
                    svelteApi.get<TermsOfService[]>('/terms_of_service', { baseURL: '/api/v4' }),
                    svelteApi.get<TermsStats & { has_active_terms?: boolean; current_terms?: TermsOfService }>(
                        '/terms_of_service/stats/summary',
                        { baseURL: '/api/v4' },
                    ),
                ])
                update((state) => ({
                    ...state,
                    terms: terms.data ?? [],
                    termsStats: stats.data?.has_active_terms === false ? null : stats.data,
                }))
            })
        },
        async createTerms(data: {
            version: string
            title: string
            content: string
            summary?: string
            effective_date: string
        }): Promise<boolean> {
            const response = await load(() =>
                svelteApi.post<unknown>('/terms_of_service', {
                    ...data,
                    effective_date: new Date(data.effective_date || new Date()).toISOString(),
                }, { baseURL: '/api/v4' }),
            )
            if (response) {
                await this.fetchTerms()
                return true
            }
            return false
        },
        async updateTerms(id: string, data: {
            title: string
            content: string
            summary?: string
            effective_date: string
        }): Promise<boolean> {
            const response = await load(() =>
                svelteApi.put<unknown>(`/terms_of_service/${id}`, {
                    ...data,
                    summary: data.summary || undefined,
                    effective_date: new Date(data.effective_date || new Date()).toISOString(),
                }, { baseURL: '/api/v4' }),
            )
            if (response) {
                await this.fetchTerms()
                return true
            }
            return false
        },
        async activateTerms(id: string): Promise<boolean> {
            const response = await load(() =>
                svelteApi.post<unknown>(`/terms_of_service/${id}/activate`, undefined, { baseURL: '/api/v4' }),
            )
            if (response) {
                await this.fetchTerms()
                return true
            }
            return false
        },
        async deleteTerms(id: string): Promise<boolean> {
            const response = await load(() =>
                svelteApi.delete<unknown>(`/terms_of_service/${id}`, { baseURL: '/api/v4' }),
            )
            if (response) {
                await this.fetchTerms()
                return true
            }
            return false
        },
    }
}

export const adminStore = createAdminStore()
