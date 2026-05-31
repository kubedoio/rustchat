// Config Service — UNFINISHED REFACTORING, NOT WIRED INTO THE APP
//
// This service registers a WebSocket handler on wsManager, but wsManager.connect()
// is never called anywhere in the app. The active config sync path is in
// frontend/src/stores/config.ts and frontend/src/features/config/stores/configStore.ts,
// both of which use useWebSocket() from composables/useWebSocket.ts.

import { log } from '@/utils/log'
import { configRepository } from '../repositories/configRepository'
import { useConfigStore } from '../stores/configStore'
import { wsManager } from '../../../core/websocket/WebSocketManager'

class ConfigService {
  private get store() {
    return useConfigStore()
  }

  async loadConfig(): Promise<void> {
    try {
      const config = await configRepository.getPublicConfig()
      this.store.setConfig(config)
    } catch (error) {
      log.error('Failed to load site config', error)
    }
  }

  // Initialize WebSocket listener for live config updates
  initSync(): () => void {
    return wsManager.on('config_updated', event => {
      try {
        const data = JSON.parse(event.data)
        if (data.category === 'site') {
          const currentConfig = this.store.siteConfig
          this.store.setConfig({
            ...currentConfig,
            site_name: data.config.site_name,
            logo_url: data.config.logo_url,
          })
        }
      } catch {
        // Ignore parse errors
      }
    })
  }
}

export const configService = new ConfigService()
