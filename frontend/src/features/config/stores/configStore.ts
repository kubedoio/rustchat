// Config Store - Backwards-compatible state management for site config

import { log } from '@/utils/log';
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { siteApi, type PublicConfig } from '../../../api/site'
import { type AuthConfig } from '../../../api/admin'
import { useWebSocket } from '../../../composables/useWebSocket'

export interface FullConfig {
    site: PublicConfig
    authentication: AuthConfig
}

export const useConfigStore = defineStore('configStore', () => {
    const siteConfig = ref<PublicConfig>({
        site_name: 'RustChat',
        logo_url: undefined,
        enable_sso: false,
        require_sso: false,
        post_edit_time_limit_seconds: -1,
    })

    const authConfig = ref<AuthConfig | null>(null)
    const configLoaded = ref(false)

    // Computed full config
    const config = computed<FullConfig | null>(() => {
        if (!authConfig.value) return null
        return {
            site: siteConfig.value,
            authentication: authConfig.value
        }
    })

    async function fetchPublicConfig() {
        try {
            const { data } = await siteApi.getInfo()
            siteConfig.value = data
        } catch (e) {
            log.error('Failed to fetch site config', e)
        }
    }

    async function loadConfig() {
        if (configLoaded.value) return

        await fetchPublicConfig()

        // Auth config is now included in public site info
        // This allows login page to know SSO status without authentication
        authConfig.value = {
            enable_email_password: true,
            enable_sso: siteConfig.value.enable_sso ?? false,
            require_sso: siteConfig.value.require_sso ?? false,
            allow_registration: true,
            enable_sign_in_with_email: true,
            enable_sign_in_with_username: true,
            enable_sign_up_with_email: true,
            enable_sign_up_with_gitlab: false,
            enable_sign_up_with_google: false,
            enable_sign_up_with_office365: false,
            enable_sign_up_with_openid: false,
            enable_user_creation: true,
            enable_open_server: false,
            enable_guest_accounts: false,
            enable_multifactor_authentication: false,
            enforce_multifactor_authentication: false,
            enable_saml: false,
            enable_ldap: false,
            password_min_length: 8,
            password_require_lowercase: true,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_symbol: false,
            password_enable_forgot_link: true,
            session_length_hours: 24,
        }

        configLoaded.value = true
    }

    function setAuthConfig(newConfig: AuthConfig) {
        authConfig.value = newConfig
    }

    function setConfig(configUpdate: Partial<PublicConfig>) {
        siteConfig.value = { ...siteConfig.value, ...configUpdate }
    }

    // Initialize WebSocket listener for live updates
    function initSync() {
        const { onEvent } = useWebSocket()

        onEvent('config_updated', (payload: unknown) => {
            const data = payload && typeof payload === 'object' ? payload as Record<string, unknown> : {}
            const config = data.config && typeof data.config === 'object'
                ? data.config as Record<string, unknown>
                : {}
            if (data.category === 'site') {
                siteConfig.value = {
                    ...siteConfig.value,
                    ...config,
                }
            } else if (data.category === 'authentication') {
                authConfig.value = { ...authConfig.value, ...config } as AuthConfig
            }
        })
    }

    return {
        siteConfig,
        authConfig,
        config,
        configLoaded,
        fetchPublicConfig,
        loadConfig,
        setAuthConfig,
        setConfig,
        initSync
    }
})
