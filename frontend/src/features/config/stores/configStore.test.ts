import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useConfigStore } from './configStore'
import type { AuthConfig } from '@/api/admin'

const { onEventMock } = vi.hoisted(() => ({ onEventMock: vi.fn() }))

vi.mock('@/composables/useWebSocket', () => ({
  useWebSocket: () => ({ onEvent: onEventMock }),
}))

function buildAuthConfig(overrides: Partial<AuthConfig> = {}): AuthConfig {
  return {
    enable_email_password: true,
    enable_sso: false,
    require_sso: false,
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
    ...overrides,
  }
}

function lastConfigUpdatedHandler(): (payload: unknown) => void {
  const calls = onEventMock.mock.calls.filter(([event]) => event === 'config_updated')
  expect(calls.length).toBeGreaterThan(0)
  return calls[calls.length - 1][1] as (payload: unknown) => void
}

describe('configStore live update sync', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    onEventMock.mockReset()
  })

  it('subscribes to config_updated on initSync and merges site updates into state', () => {
    const store = useConfigStore()
    store.initSync()

    expect(onEventMock).toHaveBeenCalledWith('config_updated', expect.any(Function))

    lastConfigUpdatedHandler()({ category: 'site', config: { site_name: 'Acme Chat' } })

    expect(store.siteConfig.site_name).toBe('Acme Chat')
    // Untouched fields keep their previous values.
    expect(store.siteConfig.post_edit_time_limit_seconds).toBe(-1)
  })

  it('merges authentication updates when an auth config has been loaded', () => {
    const store = useConfigStore()
    store.setAuthConfig(buildAuthConfig({ allow_registration: false }))
    store.initSync()

    lastConfigUpdatedHandler()({ category: 'authentication', config: { allow_registration: true } })

    expect(store.authConfig?.allow_registration).toBe(true)
    expect(store.authConfig?.password_min_length).toBe(8)
  })

  it('ignores authentication updates while auth config is not loaded', () => {
    const store = useConfigStore()
    store.initSync()

    lastConfigUpdatedHandler()({
      category: 'authentication',
      config: { allow_registration: true, password_min_length: 99 },
    })

    // A partial payload must not replace the null placeholder with incomplete data.
    expect(store.authConfig).toBeNull()
    expect(store.config).toBeNull()
  })

  it('ignores malformed config_updated payloads', () => {
    const store = useConfigStore()
    store.initSync()

    const handler = lastConfigUpdatedHandler()
    handler(null)
    handler('nope')
    handler({ category: 'site' })

    expect(store.siteConfig.site_name).toBe('RustChat')
  })
})
