import { defineStore } from 'pinia'
import { ref } from 'vue'
import { siteApi, type PublicConfig } from '../api/site'
import { useWebSocket } from '../composables/useWebSocket'

export const useConfigStore = defineStore('config', () => {
    const siteConfig = ref<PublicConfig>({
        site_name: 'RustChat',
        logo_url: undefined
    })

    async function fetchPublicConfig() {
        try {
            const { data } = await siteApi.getInfo()
            siteConfig.value = data
        } catch (e) {
            console.error('Failed to fetch site config', e)
        }
    }

    // Initialize WebSocket listener for live updates
    function initSync() {
        const { onEvent } = useWebSocket()

        onEvent('config_updated', (data: any) => {
            if (data.category === 'site') {
                siteConfig.value = {
                    site_name: data.config.site_name,
                    logo_url: data.config.logo_url
                }
            }
        })
    }

    return { siteConfig, fetchPublicConfig, initSync }
})
