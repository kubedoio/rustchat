import api from './client'

export interface MailProviderResponse {
  id: string
  tenant_id: string | null
  provider_type: string
  host: string
  port: number
  username: string
  has_password: boolean
  tls_mode: string
  skip_cert_verify: boolean
  from_address: string
  from_name: string
  reply_to: string | null
  max_emails_per_minute: number
  max_emails_per_hour: number
  enabled: boolean
  is_default: boolean
  created_at: string
  updated_at: string
}

export interface CreateMailProviderRequest {
  provider_type: string
  host: string
  port: number
  username: string
  password: string
  tls_mode: string
  skip_cert_verify?: boolean
  from_address: string
  from_name: string
  reply_to?: string | null
  max_emails_per_minute?: number
  max_emails_per_hour?: number
  enabled?: boolean
  is_default?: boolean
}

export interface UpdateMailProviderRequest extends Partial<CreateMailProviderRequest> {}

export interface ProviderTestResult {
  success: boolean
  stage?: string
  message?: string
  error?: string
  server_response?: string
}

export interface WorkflowPolicy {
  token_expiry_hours?: number
  require_opt_in?: boolean
  list_unsubscribe?: boolean
  throttle_minutes?: number
  max_per_hour?: number
  include_excerpt?: boolean
  respect_quiet_hours?: boolean
  day_of_week?: number
  hour?: number
}

export interface WorkflowResponse {
  id: string
  tenant_id: string | null
  workflow_key: string
  name: string
  description: string | null
  category: string
  enabled: boolean
  system_required: boolean
  can_disable: boolean
  default_locale: string
  selected_template_family_id: string | null
  policy: WorkflowPolicy
  created_at: string
  updated_at: string
}

export interface UpdateWorkflowRequest {
  enabled?: boolean
  default_locale?: string
  selected_template_family_id?: string | null
  policy?: WorkflowPolicy
}

export interface EmailTemplateFamily {
  id: string
  tenant_id: string | null
  key: string
  name: string
  description: string | null
  workflow_key: string | null
  is_system: boolean
  created_at: string
  updated_at: string
  created_by: string | null
}

export interface CreateTemplateFamilyRequest {
  key: string
  name: string
  description?: string | null
  workflow_key?: string | null
}

export interface UpdateTemplateFamilyRequest {
  name?: string
  description?: string | null
}

export interface TemplateVersionResponse {
  id: string
  family_id: string
  version: number
  status: string
  locale: string
  subject: string
  body_text?: string | null
  body_html?: string | null
  variables: Array<{
    name: string
    required: boolean
    default_value?: string | null
    description?: string | null
  }>
  is_compiled_from_mjml: boolean
  created_at: string
  created_by: string | null
  published_at: string | null
  published_by: string | null
}

export interface CreateTemplateVersionRequest {
  locale: string
  subject: string
  body_text: string
  body_html: string
  variables?: Array<{
    name: string
    required: boolean
    default_value?: string | null
    description?: string | null
  }>
  is_compiled_from_mjml?: boolean
  mjml_source?: string | null
}

export interface EmailOutboxResponse {
  id: string
  workflow_key: string | null
  recipient_email: string
  recipient_user_id: string | null
  subject: string
  status: string
  priority: string
  attempt_count: number
  max_attempts: number
  sent_at: string | null
  created_at: string
}

export interface EmailOutboxDetails extends EmailOutboxResponse {
  tenant_id?: string | null
  template_family_id?: string | null
  template_version?: number | null
  locale?: string | null
  body_text?: string | null
  body_html?: string | null
  payload_json?: unknown
  last_error_category?: string | null
  last_error_message?: string | null
  provider_id?: string | null
  provider_message_id?: string | null
  next_attempt_at?: string | null
}

export interface EmailEventResponse {
  id: string
  outbox_id: string | null
  workflow_key: string | null
  event_type: string
  recipient_email: string
  template_version: number | null
  locale: string | null
  status_code: number | null
  error_category: string | null
  error_message: string | null
  created_at: string
}

export interface SendTestEmailRequest {
  provider_id?: string | null
  to_email: string
  workflow_key?: string | null
  template_family_id?: string | null
  locale?: string | null
  subject?: string | null
  body_text?: string | null
}

export const adminEmailApi = {
  listProviders: () => api.get<MailProviderResponse[]>('/admin/email/providers'),
  createProvider: (body: CreateMailProviderRequest) =>
    api.post<MailProviderResponse>('/admin/email/providers', body),
  updateProvider: (id: string, body: UpdateMailProviderRequest) =>
    api.put<MailProviderResponse>(`/admin/email/providers/${id}`, body),
  deleteProvider: (id: string) => api.delete(`/admin/email/providers/${id}`),
  setDefaultProvider: (id: string) =>
    api.post<MailProviderResponse>(`/admin/email/providers/${id}/default`),
  testProvider: (id: string, to_email: string) =>
    api.post<ProviderTestResult>(`/admin/email/providers/${id}/test`, { to_email }),

  listWorkflows: () => api.get<WorkflowResponse[]>('/admin/email/workflows'),
  updateWorkflow: (id: string, body: UpdateWorkflowRequest) =>
    api.patch<WorkflowResponse>(`/admin/email/workflows/${id}`, body),

  listTemplateFamilies: () => api.get<EmailTemplateFamily[]>('/admin/email/template-families'),
  createTemplateFamily: (body: CreateTemplateFamilyRequest) =>
    api.post<EmailTemplateFamily>('/admin/email/template-families', body),
  updateTemplateFamily: (id: string, body: UpdateTemplateFamilyRequest) =>
    api.patch<EmailTemplateFamily>(`/admin/email/template-families/${id}`, body),
  deleteTemplateFamily: (id: string) => api.delete(`/admin/email/template-families/${id}`),

  listTemplateVersions: (familyId: string) =>
    api.get<TemplateVersionResponse[]>(`/admin/email/template-families/${familyId}/versions`),
  createTemplateVersion: (familyId: string, body: CreateTemplateVersionRequest) =>
    api.post<TemplateVersionResponse>(`/admin/email/template-families/${familyId}/versions`, body),
  publishTemplateVersion: (versionId: string) =>
    api.post(`/admin/email/template-versions/${versionId}/publish`),

  listOutbox: (params?: {
    status?: string
    workflow_key?: string
    recipient_email?: string
    page?: number
    per_page?: number
  }) => api.get<EmailOutboxResponse[]>('/admin/email/outbox', { params }),
  getOutboxEntry: (id: string) => api.get<EmailOutboxDetails>(`/admin/email/outbox/${id}`),
  cancelOutboxEntry: (id: string) => api.post(`/admin/email/outbox/${id}/cancel`),
  retryOutboxEntry: (id: string) => api.post(`/admin/email/outbox/${id}/retry`),

  listEvents: (params?: {
    outbox_id?: string
    workflow_key?: string
    event_type?: string
    page?: number
    per_page?: number
  }) => api.get<EmailEventResponse[]>('/admin/email/events', { params }),

  sendTestEmail: (body: SendTestEmailRequest) => api.post('/admin/email/send-test', body),
}

export default adminEmailApi
