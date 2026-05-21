// Config Repository - Data access for site config

import { siteApi } from '../../../api/site'
import type { PublicConfig } from '../../../api/site'
import { withRetry } from '../../../core/services/retry'

export const configRepository = {
  async getPublicConfig(): Promise<PublicConfig> {
    return withRetry(async () => {
      const response = await siteApi.getInfo()
      return response.data
    })
  },
}
