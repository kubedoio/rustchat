import api from './client'

export interface PublicConfig {
  site_name: string
  logo_url?: string
  enable_sso: boolean
  require_sso: boolean
  post_edit_time_limit_seconds: number
}

export const siteApi = {
  getInfo: () => api.get<PublicConfig>('/site/info'),
}
