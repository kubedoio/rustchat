<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertCircle, Bot, CheckCircle, Globe, Phone, Plus, Save, Server, Terminal, Trash2, Webhook } from 'lucide-svelte'
  import { adminStore } from '../../stores/admin'
  import type { CallsPluginConfig, IntegrationsConfig } from '../../../api/admin'

  const form = $state<IntegrationsConfig & { max_webhooks_per_team: number; webhook_payload_size_kb: number }>({
    enable_webhooks: true,
    enable_slash_commands: true,
    enable_bots: true,
    max_webhooks_per_team: 10,
    webhook_payload_size_kb: 100,
  })

  const calls = $state<CallsPluginConfig>({
    enabled: false,
    turn_server_enabled: true,
    turn_server_url: '',
    turn_server_username: '',
    turn_server_credential: '',
    udp_port: 8443,
    tcp_port: 8443,
    ice_host_override: '',
    stun_servers: ['stun:stun.l.google.com:19302'],
  })

  let saving = $state(false)
  let saved = $state(false)
  let callsSaved = $state(false)
  let stunInput = $state('')

  onMount(async () => {
    await Promise.all([adminStore.fetchConfig(), adminStore.fetchCallsPluginConfig()])
    if ($adminStore.config?.integrations) {
      Object.assign(form, $adminStore.config.integrations)
    }
    if ($adminStore.callsPluginConfig) {
      Object.assign(calls, {
        ...$adminStore.callsPluginConfig,
        ice_host_override: $adminStore.callsPluginConfig.ice_host_override || '',
        turn_server_credential: $adminStore.callsPluginConfig.turn_server_credential || '',
      })
    }
  })

  async function saveSettings() {
    saving = true
    saved = false
    try {
      saved = await adminStore.updateConfig('integrations', form)
    } finally {
      saving = false
    }
  }

  async function saveCalls() {
    saving = true
    callsSaved = false
    try {
      callsSaved = await adminStore.updateCallsPluginConfig({
        ...calls,
        turn_server_credential: calls.turn_server_credential || null,
        ice_host_override: calls.ice_host_override || null,
        stun_servers: calls.stun_servers.length ? calls.stun_servers : ['stun:stun.l.google.com:19302'],
      })
    } finally {
      saving = false
    }
  }

  function addStunServer() {
    const value = stunInput.trim()
    if (!value) return
    calls.stun_servers = [...calls.stun_servers, value]
    stunInput = ''
  }

  function removeStunServer(index: number) {
    calls.stun_servers = calls.stun_servers.filter((_, current) => current !== index)
  }
</script>

<section class="space-y-6" data-testid="admin-integrations-settings-page">
  <div class="flex flex-col gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Connected services</p>
      <h1 class="mt-2 text-2xl font-semibold tracking-[-0.03em] text-text-1">Integrations</h1>
      <p class="mt-2 text-sm text-text-3">Configure webhooks, slash commands, bots, and calling infrastructure.</p>
    </div>
    <div class="flex items-center gap-3">
      {#if saved}
        <span class="inline-flex items-center gap-1 text-sm font-medium text-success"><CheckCircle class="h-4 w-4" /> Saved</span>
      {/if}
      <button type="button" onclick={saveSettings} disabled={saving} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground disabled:opacity-50">
        <Save class="h-4 w-4" /> {saving ? 'Saving...' : 'Save Changes'}
      </button>
    </div>
  </div>

  {#if $adminStore.error}
    <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
      <AlertCircle class="mr-2 inline h-4 w-4" /> {$adminStore.error}
    </div>
  {/if}

  <div class="space-y-4">
    <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
      <div class="flex items-start justify-between gap-4">
        <div class="flex items-start gap-3">
          <Webhook class="mt-1 h-6 w-6 text-brand" />
          <div>
            <h2 class="font-semibold text-text-1">Webhooks</h2>
            <p class="text-sm text-text-3">Incoming and outgoing webhooks for integrations.</p>
          </div>
        </div>
        <input type="checkbox" bind:checked={form.enable_webhooks} class="h-5 w-5 rounded text-brand" />
      </div>
      {#if form.enable_webhooks}
        <div class="mt-5 grid grid-cols-1 gap-4 border-t border-border-1 pt-5 md:grid-cols-2">
          <div>
            <label for="max-webhooks" class="mb-1 block text-sm font-medium text-text-2">Max per Team</label>
            <input id="max-webhooks" type="number" min="0" bind:value={form.max_webhooks_per_team} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
          </div>
          <div>
            <label for="payload-size" class="mb-1 block text-sm font-medium text-text-2">Max Payload (KB)</label>
            <input id="payload-size" type="number" min="1" bind:value={form.webhook_payload_size_kb} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
          </div>
        </div>
      {/if}
    </section>

    <section class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <label class="flex items-center justify-between gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
        <span class="flex items-start gap-3">
          <Terminal class="mt-1 h-6 w-6 text-success" />
          <span><span class="block font-semibold text-text-1">Slash Commands</span><span class="text-sm text-text-3">Custom commands for teams and channels.</span></span>
        </span>
        <input type="checkbox" bind:checked={form.enable_slash_commands} class="h-5 w-5 rounded text-brand" />
      </label>
      <label class="flex items-center justify-between gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
        <span class="flex items-start gap-3">
          <Bot class="mt-1 h-6 w-6 text-warning" />
          <span><span class="block font-semibold text-text-1">Bot Accounts</span><span class="text-sm text-text-3">Allow creation of bot users for automation.</span></span>
        </span>
        <input type="checkbox" bind:checked={form.enable_bots} class="h-5 w-5 rounded text-brand" />
      </label>
    </section>

    <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-6 shadow-1">
      <div class="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div class="flex items-start gap-3">
          <Phone class="mt-1 h-6 w-6 text-brand" />
          <div>
            <h2 class="font-semibold text-text-1">RustChat Calls Plugin</h2>
            <p class="text-sm text-text-3">Configure WebRTC calling infrastructure and TURN/STUN servers.</p>
          </div>
        </div>
        <div class="flex items-center gap-3">
          {#if callsSaved}
            <span class="text-sm font-medium text-success">Saved</span>
          {/if}
          <input type="checkbox" bind:checked={calls.enabled} class="h-5 w-5 rounded text-brand" aria-label="Enable calls plugin" />
        </div>
      </div>

      {#if calls.enabled}
        <div class="mt-5 space-y-6 border-t border-border-1 pt-5">
          <div>
            <label class="mb-4 flex items-center gap-3 text-sm font-semibold text-text-1">
              <input type="checkbox" bind:checked={calls.turn_server_enabled} class="h-4 w-4 rounded text-brand" />
              Enable TURN Server
            </label>
            <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
              <div>
                <label for="turn-url" class="mb-1 block text-sm font-medium text-text-2">TURN Server URL</label>
                <input id="turn-url" bind:value={calls.turn_server_url} placeholder="turn:turn.example.com:3478" class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
              </div>
              <div>
                <label for="turn-username" class="mb-1 block text-sm font-medium text-text-2">Username</label>
                <input id="turn-username" bind:value={calls.turn_server_username} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
              </div>
              <div>
                <label for="turn-credential" class="mb-1 block text-sm font-medium text-text-2">Credential</label>
                <input id="turn-credential" type="password" bind:value={calls.turn_server_credential} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
              </div>
              <div>
                <label for="ice-host" class="mb-1 block text-sm font-medium text-text-2">ICE Host Override</label>
                <input id="ice-host" bind:value={calls.ice_host_override} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
              </div>
              <div>
                <label for="udp-port" class="mb-1 block text-sm font-medium text-text-2">UDP Port</label>
                <input id="udp-port" type="number" min="1" max="65535" bind:value={calls.udp_port} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
              </div>
              <div>
                <label for="tcp-port" class="mb-1 block text-sm font-medium text-text-2">TCP Port</label>
                <input id="tcp-port" type="number" min="1" max="65535" bind:value={calls.tcp_port} class="w-full rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
              </div>
            </div>
          </div>

          <div>
            <div class="mb-3 flex items-center gap-2">
              <Globe class="h-4 w-4 text-text-4" />
              <h3 class="text-sm font-semibold text-text-1">STUN Servers</h3>
            </div>
            <div class="space-y-2">
              {#each calls.stun_servers as _, index}
                <div class="flex gap-2">
                  <input bind:value={calls.stun_servers[index]} aria-label={`STUN server ${index + 1}`} class="min-w-0 flex-1 rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
                  <button type="button" onclick={() => removeStunServer(index)} class="rounded-r-2 border border-danger/20 px-3 py-2 text-danger">
                    <Trash2 class="h-4 w-4" />
                  </button>
                </div>
              {/each}
              <div class="flex gap-2">
                <input bind:value={stunInput} onkeydown={(event) => { if (event.key === 'Enter') { event.preventDefault(); addStunServer() } }} placeholder="stun:stun.example.com:19302" class="min-w-0 flex-1 rounded-r-2 border border-border-2 bg-bg-surface-1 px-3 py-2 text-sm text-text-1" />
                <button type="button" onclick={addStunServer} class="inline-flex items-center gap-2 rounded-r-2 border border-border-2 px-3 py-2 text-sm font-semibold text-text-2">
                  <Plus class="h-4 w-4" /> Add
                </button>
              </div>
            </div>
          </div>

          <div class="flex justify-end border-t border-border-1 pt-5">
            <button type="button" onclick={saveCalls} disabled={saving} class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2 text-sm font-semibold text-brand-foreground disabled:opacity-50">
              <Server class="h-4 w-4" /> Save Calls Configuration
            </button>
          </div>
        </div>
      {/if}
    </section>
  </div>
</section>
