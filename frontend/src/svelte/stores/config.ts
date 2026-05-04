import { derived, writable } from 'svelte/store'
import { svelteApi } from './http'

export interface PublicConfig {
    site_name: string
    logo_url?: string
    enable_sso: boolean
    require_sso: boolean
    post_edit_time_limit_seconds: number
}

export interface AuthConfig {
    enable_email_password: boolean
    enable_sso: boolean
    require_sso: boolean
    allow_registration: boolean
    enable_sign_in_with_email: boolean
    enable_sign_in_with_username: boolean
    enable_sign_up_with_email: boolean
    enable_sign_up_with_gitlab: boolean
    enable_sign_up_with_google: boolean
    enable_sign_up_with_office365: boolean
    enable_sign_up_with_openid: boolean
    enable_user_creation: boolean
    enable_open_server: boolean
    enable_guest_accounts: boolean
    enable_multifactor_authentication: boolean
    enforce_multifactor_authentication: boolean
    enable_saml: boolean
    enable_ldap: boolean
    password_min_length: number
    password_require_lowercase: boolean
    password_require_uppercase: boolean
    password_require_number: boolean
    password_require_symbol: boolean
    password_enable_forgot_link: boolean
    session_length_hours: number
}

export interface FullConfig {
    site: PublicConfig
    authentication: AuthConfig
}

export interface ConfigState {
    siteConfig: PublicConfig
    authConfig: AuthConfig | null
    configLoaded: boolean
    loading: boolean
    error: string | null
}

const DEFAULT_SITE_CONFIG: PublicConfig = {
    site_name: 'RustChat',
    logo_url: undefined,
    enable_sso: false,
    require_sso: false,
    post_edit_time_limit_seconds: -1,
}

function buildAuthConfig(siteConfig: PublicConfig): AuthConfig {
    return {
        enable_email_password: true,
        enable_sso: siteConfig.enable_sso ?? false,
        require_sso: siteConfig.require_sso ?? false,
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
}

function createConfigStore() {
    const { subscribe, update, set } = writable<ConfigState>({
        siteConfig: DEFAULT_SITE_CONFIG,
        authConfig: null,
        configLoaded: false,
        loading: false,
        error: null,
    })

    async function fetchPublicConfig(): Promise<PublicConfig | null> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<PublicConfig>('/site/info', { authenticated: false })
            update((state) => ({
                ...state,
                siteConfig: data,
                loading: false,
                error: null,
            }))
            return data
        } catch (error) {
            console.error('Failed to fetch site config', error)
            update((state) => ({
                ...state,
                loading: false,
                error: error instanceof Error ? error.message : 'Failed to fetch site config',
            }))
            return null
        }
    }

    return {
        subscribe,
        fetchPublicConfig,
        async loadConfig(): Promise<void> {
            let alreadyLoaded = false
            update((state) => {
                alreadyLoaded = state.configLoaded
                return state
            })

            if (alreadyLoaded) {
                return
            }

            const siteConfig = await fetchPublicConfig()
            if (!siteConfig) {
                return
            }

            update((state) => ({
                ...state,
                authConfig: buildAuthConfig(siteConfig),
                configLoaded: true,
            }))
        },
        setAuthConfig(authConfig: AuthConfig): void {
            update((state) => ({ ...state, authConfig }))
        },
        applyPublicConfig(siteConfig: Partial<PublicConfig>): void {
            update((state) => ({
                ...state,
                siteConfig: { ...state.siteConfig, ...siteConfig },
                authConfig: state.authConfig
                    ? { ...state.authConfig, enable_sso: siteConfig.enable_sso ?? state.authConfig.enable_sso }
                    : state.authConfig,
            }))
        },
        reset(): void {
            set({
                siteConfig: DEFAULT_SITE_CONFIG,
                authConfig: null,
                configLoaded: false,
                loading: false,
                error: null,
            })
        },
        initSync(): void {
            // WebSocket wiring will be added when the Svelte shell owns realtime setup.
        },
    }
}

export const configStore = createConfigStore()
export const fullConfig = derived(configStore, ($configStore): FullConfig | null => {
    if (!$configStore.authConfig) {
        return null
    }

    return {
        site: $configStore.siteConfig,
        authentication: $configStore.authConfig,
    }
})
