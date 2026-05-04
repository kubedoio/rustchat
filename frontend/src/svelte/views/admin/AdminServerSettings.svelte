<script lang="ts">
  import { onMount } from 'svelte'
  import { Activity, CheckCircle, Clock, Globe, Save, Sliders, Upload } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'
  import type { SiteConfig } from '../../../api/admin'

  const TEAM_DEFAULT_CHANNELS_KEY = 'team_default_channels'

  const form = $state<SiteConfig>({
    site_name: '',
    logo_url: '',
    site_description: '',
    site_url: '',
    about_link: 'https://docs.mattermost.com/about/product.html/',
    help_link: 'https://mattermost.com/default-help/',
    terms_of_service_link: 'https://about.mattermost.com/default-terms/',
    privacy_policy_link: '',
    report_a_problem_link: 'https://mattermost.com/default-report-a-problem/',
    support_email: '',
    app_download_link: 'https://mattermost.com/download/#mattermostApps',
    android_app_download_link: 'https://mattermost.com/mattermost-android-app/',
    ios_app_download_link: 'https://mattermost.com/mattermost-ios-app/',
    custom_brand_text: '',
    custom_description_text: '',
    service_environment: 'production',
    max_file_size_mb: 50,
    max_simultaneous_connections: 20,
    enable_file: true,
    enable_user_statuses: true,
    enable_custom_emoji: true,
    enable_custom_brand: false,
    enable_mobile_file_download: true,
    enable_mobile_file_upload: true,
    allow_download_logs: true,
    diagnostics_enabled: false,
    default_locale: 'en',
    default_timezone: 'UTC',
    post_edit_time_limit_seconds: -1,
  })

  let defaultChannelsInput = $state('')
  let showAdvanced = $state(false)
  let saving = $state(false)
  let saved = $state(false)
  const siteToggles = [
    ['enable_file', 'Enable Files', 'Allow file uploads and downloads'],
    ['enable_user_statuses', 'Enable User Statuses', 'Allow users to set custom statuses'],
    ['enable_custom_emoji', 'Enable Custom Emoji', 'Allow custom emoji uploads'],
    ['enable_custom_brand', 'Enable Custom Branding', 'Show custom brand text in clients'],
    ['enable_mobile_file_download', 'Mobile File Download', 'Allow downloads on mobile clients'],
    ['enable_mobile_file_upload', 'Mobile File Upload', 'Allow uploads on mobile clients'],
    ['allow_download_logs', 'Allow Download Logs', 'Allow clients to download logs'],
    ['diagnostics_enabled', 'Diagnostics Enabled', 'Expose diagnostics and telemetry'],
  ] as const

  function syncFromConfig() {
    if ($adminStore.config?.site) {
      Object.assign(form, $adminStore.config.site)
    }

    const experimental = $adminStore.config?.experimental
    const channels = experimental?.[TEAM_DEFAULT_CHANNELS_KEY]
    defaultChannelsInput = Array.isArray(channels)
      ? channels.filter((item): item is string => typeof item === 'string').join(', ')
      : ''
  }

  function parseDefaultChannels(raw: string): string[] {
    const seen = new Set<string>()
    return raw
      .split(',')
      .map((item) => item.trim().toLowerCase())
      .filter((item) => item && !seen.has(item) && seen.add(item))
  }

  async function saveSettings() {
    saving = true
    saved = false
    try {
      const siteSaved = await adminStore.updateConfig('site', form)
      const experimental = $adminStore.config?.experimental ?? {}
      const experimentalSaved = await adminStore.updateConfig('experimental', {
        ...experimental,
        [TEAM_DEFAULT_CHANNELS_KEY]: parseDefaultChannels(defaultChannelsInput),
      })
      saved = siteSaved && experimentalSaved
    } finally {
      saving = false
    }
  }

  function setSiteToggle(key: (typeof siteToggles)[number][0], checked: boolean) {
    form[key] = checked
  }

  onMount(async () => {
    await adminStore.fetchConfig()
    syncFromConfig()
  })
</script>

<section class="space-y-6" data-testid="admin-server-settings-page">
  <div class="flex flex-col gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Configuration</p>
      <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Server Settings</h1>
      <p class="mt-2 text-sm text-text-3">Configure site identity, file limits, client defaults, and localization.</p>
    </div>
    <div class="flex items-center gap-3">
      {#if saved}
        <span class="inline-flex items-center gap-1 text-sm font-medium text-success">
          <CheckCircle class="h-4 w-4" /> Saved
        </span>
      {/if}
      <button
        type="button"
        onclick={saveSettings}
        disabled={saving}
        class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground transition-standard hover:opacity-90 disabled:opacity-50"
      >
        <Save class="h-4 w-4" />
        {saving ? 'Saving...' : 'Save Changes'}
      </button>
    </div>
  </div>

  {#if $adminStore.error}
    <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
      {$adminStore.error}
    </div>
  {/if}

  <form class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1" onsubmit={(event) => { event.preventDefault(); void saveSettings() }}>
    <section class="border-b border-border-1 p-6">
      <div class="mb-4 flex items-center gap-2">
        <Globe class="h-5 w-5 text-text-4" />
        <h2 class="text-lg font-semibold text-text-1">Site Information</h2>
      </div>
      <div class="grid grid-cols-1 gap-5 md:grid-cols-2">
        <div>
          <label for="site-name" class="mb-1 block text-sm font-medium text-text-2">Site Name</label>
          <input id="site-name" bind:value={form.site_name} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
        <div>
          <label for="site-url" class="mb-1 block text-sm font-medium text-text-2">Site URL</label>
          <input id="site-url" type="url" bind:value={form.site_url} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
        <div>
          <label for="logo-url" class="mb-1 block text-sm font-medium text-text-2">Logo URL</label>
          <input id="logo-url" bind:value={form.logo_url} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
        <div>
          <label for="support-email" class="mb-1 block text-sm font-medium text-text-2">Support Email</label>
          <input id="support-email" type="email" bind:value={form.support_email} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        </div>
        <div class="md:col-span-2">
          <label for="site-description" class="mb-1 block text-sm font-medium text-text-2">Site Description</label>
          <textarea id="site-description" rows="2" bind:value={form.site_description} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1"></textarea>
        </div>
      </div>
    </section>

    <section class="grid grid-cols-1 divide-y divide-border-1 md:grid-cols-2 md:divide-x md:divide-y-0">
      <div class="p-6">
        <div class="mb-4 flex items-center gap-2">
          <Upload class="h-5 w-5 text-text-4" />
          <h2 class="text-lg font-semibold text-text-1">File Uploads</h2>
        </div>
        <label for="max-file-size" class="mb-1 block text-sm font-medium text-text-2">Max File Size (MB)</label>
        <input id="max-file-size" type="number" min="1" max="500" bind:value={form.max_file_size_mb} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
      </div>
      <div class="p-6">
        <div class="mb-4 flex items-center gap-2">
          <Activity class="h-5 w-5 text-text-4" />
          <h2 class="text-lg font-semibold text-text-1">Connection Limits</h2>
        </div>
        <label for="max-connections" class="mb-1 block text-sm font-medium text-text-2">Max Simultaneous Connections per User</label>
        <input id="max-connections" type="number" min="1" max="100" bind:value={form.max_simultaneous_connections} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
      </div>
    </section>

    <section class="border-t border-border-1 p-6">
      <div class="mb-4 flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <Sliders class="h-5 w-5 text-text-4" />
          <h2 class="text-lg font-semibold text-text-1">Client Configuration</h2>
        </div>
        <button type="button" onclick={() => (showAdvanced = !showAdvanced)} class="rounded-r-2 border border-border-2 px-3 py-1.5 text-sm font-medium text-text-2">
          {showAdvanced ? 'Hide' : 'Show'} Advanced
        </button>
      </div>

      {#if showAdvanced}
        <div class="grid grid-cols-1 gap-5 md:grid-cols-2">
          <div>
            <label for="terms-link" class="mb-1 block text-sm font-medium text-text-2">Terms of Service Link</label>
            <input id="terms-link" type="url" bind:value={form.terms_of_service_link} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
          </div>
          <div>
            <label for="privacy-link" class="mb-1 block text-sm font-medium text-text-2">Privacy Policy Link</label>
            <input id="privacy-link" type="url" bind:value={form.privacy_policy_link} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
          </div>
          <div>
            <label for="service-env" class="mb-1 block text-sm font-medium text-text-2">Service Environment</label>
            <select id="service-env" bind:value={form.service_environment} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
              <option value="production">Production</option>
              <option value="staging">Staging</option>
              <option value="development">Development</option>
            </select>
          </div>
          <div>
            <label for="edit-window" class="mb-1 block text-sm font-medium text-text-2">Message Edit Time Limit (seconds)</label>
            <input id="edit-window" type="number" min="-1" bind:value={form.post_edit_time_limit_seconds} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
          </div>
        </div>

        <div class="mt-5 grid grid-cols-1 gap-3 md:grid-cols-2">
          {#each siteToggles as item}
            <label class="flex items-center justify-between gap-4 rounded-r-2 bg-bg-surface-2 p-4">
              <span>
                <span class="block text-sm font-semibold text-text-1">{item[1]}</span>
                <span class="mt-1 block text-xs text-text-3">{item[2]}</span>
              </span>
              <input
                type="checkbox"
                checked={Boolean(form[item[0]])}
                onchange={(event) => setSiteToggle(item[0], event.currentTarget.checked)}
                class="h-4 w-4 rounded border-border-1 text-brand"
              />
            </label>
          {/each}
        </div>
      {:else}
        <p class="text-sm text-text-3">Advanced client behavior and legacy config fields are hidden to keep the common path calm.</p>
      {/if}
    </section>

    <section class="grid grid-cols-1 gap-5 border-t border-border-1 p-6 md:grid-cols-2">
      <div>
        <div class="mb-4 flex items-center gap-2">
          <Globe class="h-5 w-5 text-text-4" />
          <h2 class="text-lg font-semibold text-text-1">Team Membership Defaults</h2>
        </div>
        <label for="default-channels" class="mb-1 block text-sm font-medium text-text-2">Default Channels For New Team Members</label>
        <input id="default-channels" bind:value={defaultChannelsInput} placeholder="off-topic, announcements" class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
        <p class="mt-2 text-xs text-text-3">Comma-separated channel names. `town-square` is always included by the backend.</p>
      </div>
      <div>
        <div class="mb-4 flex items-center gap-2">
          <Clock class="h-5 w-5 text-text-4" />
          <h2 class="text-lg font-semibold text-text-1">Localization</h2>
        </div>
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label for="locale" class="mb-1 block text-sm font-medium text-text-2">Default Locale</label>
            <select id="locale" bind:value={form.default_locale} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
              <option value="en">English</option>
              <option value="es">Spanish</option>
              <option value="fr">French</option>
              <option value="de">German</option>
            </select>
          </div>
          <div>
            <label for="timezone" class="mb-1 block text-sm font-medium text-text-2">Default Timezone</label>
            <select id="timezone" bind:value={form.default_timezone} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1">
              <option value="UTC">UTC</option>
              <option value="America/New_York">Eastern Time</option>
              <option value="America/Los_Angeles">Pacific Time</option>
              <option value="Europe/London">London</option>
              <option value="Europe/Paris">Paris</option>
            </select>
          </div>
        </div>
      </div>
    </section>
  </form>
</section>
