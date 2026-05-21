import api from './client'

export type PolicyScopeType = 'global' | 'team'
export type PolicySourceType = 'all_users' | 'auth_service' | 'group' | 'role' | 'org'
export type PolicyTargetType = 'team' | 'channel'
export type RoleMode = 'member' | 'admin'
export type MembershipOrigin = 'manual' | 'policy' | 'invite' | 'sync' | 'default'

export interface AutoMembershipPolicy {
  id: string
  name: string
  description: string | null
  scope_type: PolicyScopeType
  team_id: string | null
  source_type: PolicySourceType
  source_config: Record<string, any>
  enabled: boolean
  priority: number
  created_at: string
  updated_at: string
}

export interface AutoMembershipPolicyTarget {
  id: string
  policy_id: string
  target_type: PolicyTargetType
  target_id: string
  role_mode: RoleMode
  created_at: string
}

export interface PolicyWithTargets extends AutoMembershipPolicy {
  targets: AutoMembershipPolicyTarget[]
}

export interface AutoMembershipPolicyAudit {
  id: string
  policy_id: string | null
  run_id: string
  user_id: string
  target_type: PolicyTargetType
  target_id: string
  action: string
  status: 'success' | 'failed' | 'pending'
  error_message: string | null
  created_at: string
}

export interface CreatePolicyTarget {
  target_type: PolicyTargetType
  target_id: string
  role_mode?: RoleMode
}

export interface CreatePolicyRequest {
  name: string
  description?: string
  scope_type: PolicyScopeType
  team_id?: string
  source_type: PolicySourceType
  source_config?: Record<string, any>
  enabled: boolean
  priority?: number
  targets: CreatePolicyTarget[]
}

export interface UpdatePolicyRequest {
  name?: string
  description?: string | null
  enabled?: boolean
  priority?: number
  source_config?: Record<string, any>
  targets?: CreatePolicyTarget[]
}

export interface PolicyStatus {
  policy_id: string
  last_run: {
    success_count: number
    failed_count: number
    total: number
  } | null
}

export interface UserResyncResult {
  status: string
  user_id: string
  teams_processed: number
  memberships_applied: number
  memberships_failed: number
}

export interface ListPoliciesQuery {
  scope_type?: PolicyScopeType
  team_id?: string
  enabled?: boolean
}

// Metadata types for UI configuration
export interface SourceTypeConfigField {
  key: string
  label: string
  type: string
  description: string
  required: boolean
  placeholder?: string
}

export interface SourceTypeMetadata {
  value: PolicySourceType
  label: string
  description: string
  config_fields: SourceTypeConfigField[]
}

export interface ScopeTypeMetadata {
  value: PolicyScopeType
  label: string
  description: string
}

export interface TargetTypeMetadata {
  value: PolicyTargetType
  label: string
  description: string
}

export interface RoleModeMetadata {
  value: RoleMode
  label: string
  description: string
}

export interface PolicyMetadata {
  source_types: SourceTypeMetadata[]
  scope_types: ScopeTypeMetadata[]
  target_types: TargetTypeMetadata[]
  role_modes: RoleModeMetadata[]
}

export const membershipPoliciesApi = {
  // Policies CRUD
  listPolicies: (query?: ListPoliciesQuery) =>
    api.get<PolicyWithTargets[]>('/admin/membership-policies', { params: query }),

  getPolicy: (id: string) => api.get<PolicyWithTargets>(`/admin/membership-policies/${id}`),

  createPolicy: (data: CreatePolicyRequest) =>
    api.post<PolicyWithTargets>('/admin/membership-policies', data),

  updatePolicy: (id: string, data: UpdatePolicyRequest) =>
    api.put<PolicyWithTargets>(`/admin/membership-policies/${id}`, data),

  deletePolicy: (id: string) => api.delete(`/admin/membership-policies/${id}`),

  // Audit & Status
  getPolicyAudit: (id: string, params?: { limit?: number; offset?: number }) =>
    api.get<AutoMembershipPolicyAudit[]>(`/admin/membership-policies/${id}/audit`, { params }),

  getPolicyStatus: (id: string) => api.get<PolicyStatus>(`/admin/membership-policies/${id}/status`),

  // User re-sync
  resyncUser: (userId: string) =>
    api.post<UserResyncResult>(`/admin/membership-policies/users/${userId}/resync`),

  // Metadata for UI configuration
  getMetadata: () => api.get<PolicyMetadata>('/admin/membership-policies/metadata'),
}

export default membershipPoliciesApi
