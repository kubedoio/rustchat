<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { Save, Globe, Upload, Clock, Activity, Sliders } from 'lucide-vue-next';

const adminStore = useAdminStore();
const TEAM_DEFAULT_CHANNELS_KEY = 'team_default_channels';

const form = ref({
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
});

const saving = ref(false);
const showAdvanced = ref(false);
const defaultChannelsInput = ref('');

function parseDefaultChannelsInput(raw: string): string[] {
    const seen = new Set<string>();
    const channels: string[] = [];
    for (const item of raw.split(',')) {
        const normalized = item.trim().toLowerCase();
        if (!normalized || seen.has(normalized)) {
            continue;
        }
        seen.add(normalized);
        channels.push(normalized);
    }
    return channels;
}

function syncDefaultChannelsFromConfig(experimental: Record<string, unknown> | null | undefined) {
    const value = experimental?.[TEAM_DEFAULT_CHANNELS_KEY];
    if (!Array.isArray(value)) {
        defaultChannelsInput.value = '';
        return;
    }

    defaultChannelsInput.value = value
        .filter((item): item is string => typeof item === 'string')
        .join(', ');
}

onMounted(async () => {
    await adminStore.fetchConfig();
    if (adminStore.config?.site) {
        form.value = { ...form.value, ...adminStore.config.site };
    }
    syncDefaultChannelsFromConfig(adminStore.config?.experimental as Record<string, unknown> | undefined);
});

watch(() => adminStore.config?.site, (site) => {
    if (site) {
        form.value = { ...form.value, ...site };
    }
});

watch(
    () => adminStore.config?.experimental,
    (experimental) => {
        syncDefaultChannelsFromConfig(experimental as Record<string, unknown> | undefined);
    },
);

const saveSettings = async () => {
    saving.value = true;
    try {
        await adminStore.updateConfig('site', form.value);
        const existingExperimental =
            adminStore.config?.experimental &&
            typeof adminStore.config.experimental === 'object' &&
            !Array.isArray(adminStore.config.experimental)
                ? (adminStore.config.experimental as Record<string, unknown>)
                : {};

        await adminStore.updateConfig('experimental', {
            ...existingExperimental,
            [TEAM_DEFAULT_CHANNELS_KEY]: parseDefaultChannelsInput(defaultChannelsInput.value),
        });
    } finally {
        saving.value = false;
    }
};
</script>

<template>
    <div class="space-y-6">
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-text-1">Server Settings</h1>
                <p class="text-text-3 mt-1">Configure your RustChat instance</p>
            </div>
            <button 
                @click="saveSettings"
                :disabled="saving"
                class="flex items-center px-4 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
            >
                <Save class="w-5 h-5 mr-2" />
                {{ saving ? 'Saving...' : 'Save Changes' }}
            </button>
        </div>

        <div class="bg-bg-surface-1 rounded-xl shadow-sm border border-border-1 divide-y divide-border-1">
            <!-- Site Information -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Globe class="w-5 h-5 text-text-4 mr-2" />
                    <h2 class="text-lg font-semibold text-text-1">Site Information</h2>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Site Name</label>
                        <input 
                            v-model="form.site_name"
                            type="text"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                            placeholder="RustChat"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Site URL</label>
                        <input 
                            v-model="form.site_url"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                            placeholder="https://chat.example.com"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Logo URL (50x50 recommended)</label>
                        <input 
                            v-model="form.logo_url"
                            type="text"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                            placeholder="https://example.com/logo.png"
                        />
                    </div>
                    <div class="md:col-span-2">
                        <label class="block text-sm font-medium text-text-2 mb-1">Site Description</label>
                        <textarea 
                            v-model="form.site_description"
                            rows="2"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                            placeholder="A self-hosted team collaboration platform"
                        ></textarea>
                    </div>
                </div>
            </div>

            <!-- File Uploads -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Upload class="w-5 h-5 text-text-4 mr-2" />
                    <h2 class="text-lg font-semibold text-text-1">File Uploads</h2>
                </div>
                <div class="max-w-xs">
                    <label class="block text-sm font-medium text-text-2 mb-1">Max File Size (MB)</label>
                    <input 
                        v-model.number="form.max_file_size_mb"
                        type="number"
                        min="1"
                        max="500"
                        class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                    />
                </div>
            </div>

            <!-- Connection Limits -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Activity class="w-5 h-5 text-text-4 mr-2" />
                    <h2 class="text-lg font-semibold text-text-1">Connection Limits</h2>
                </div>
                <div class="max-w-xs">
                    <label class="block text-sm font-medium text-text-2 mb-1">Max Simultaneous Connections per User</label>
                    <input 
                        v-model.number="form.max_simultaneous_connections"
                        type="number"
                        min="1"
                        max="100"
                        class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                    />
                </div>
            </div>

            <!-- Client Configuration (Advanced) -->
            <div class="p-6">
                <div class="flex items-center justify-between mb-4">
                    <div class="flex items-center">
                        <Sliders class="w-5 h-5 text-text-4 mr-2" />
                        <h2 class="text-lg font-semibold text-text-1">Client Configuration (Advanced)</h2>
                    </div>
                    <button
                        type="button"
                        @click="showAdvanced = !showAdvanced"
                        class="px-3 py-1.5 text-sm font-medium rounded-md border border-border-2 text-text-2 hover:bg-bg-surface-2"
                    >
                        {{ showAdvanced ? 'Hide' : 'Show' }} Advanced
                    </button>
                </div>

                <div v-if="showAdvanced" class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">About Link</label>
                        <input
                            v-model="form.about_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Help Link</label>
                        <input
                            v-model="form.help_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Terms of Service Link</label>
                        <input
                            v-model="form.terms_of_service_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Privacy Policy Link</label>
                        <input
                            v-model="form.privacy_policy_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Report a Problem Link</label>
                        <input
                            v-model="form.report_a_problem_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Support Email</label>
                        <input
                            v-model="form.support_email"
                            type="email"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">App Download Link</label>
                        <input
                            v-model="form.app_download_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Android App Download Link</label>
                        <input
                            v-model="form.android_app_download_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">iOS App Download Link</label>
                        <input
                            v-model="form.ios_app_download_link"
                            type="url"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Custom Brand Text</label>
                        <input
                            v-model="form.custom_brand_text"
                            type="text"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Custom Description Text</label>
                        <input
                            v-model="form.custom_description_text"
                            type="text"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Service Environment</label>
                        <select
                            v-model="form.service_environment"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        >
                            <option value="production">Production</option>
                            <option value="staging">Staging</option>
                            <option value="development">Development</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">
                            Message Edit Time Limit (seconds)
                        </label>
                        <input
                            v-model.number="form.post_edit_time_limit_seconds"
                            type="number"
                            min="-1"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                            placeholder="-1 for unlimited, 0 to disable, e.g. 1800 for 30 min"
                        />
                        <p class="mt-1 text-xs text-text-3">
                            Use <code>-1</code> for unlimited editing, <code>0</code> to disable editing, or a positive number for a time window.
                        </p>
                    </div>
                </div>

                <div v-if="showAdvanced" class="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4">
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Enable Files</p>
                            <p class="text-sm text-text-3">Allow file uploads and downloads</p>
                        </div>
                        <input type="checkbox" v-model="form.enable_file" class="w-5 h-5 text-brand rounded" />
                    </label>
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Enable User Statuses</p>
                            <p class="text-sm text-text-3">Allow users to set custom statuses</p>
                        </div>
                        <input type="checkbox" v-model="form.enable_user_statuses" class="w-5 h-5 text-brand rounded" />
                    </label>
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Enable Custom Emoji</p>
                            <p class="text-sm text-text-3">Allow custom emoji uploads</p>
                        </div>
                        <input type="checkbox" v-model="form.enable_custom_emoji" class="w-5 h-5 text-brand rounded" />
                    </label>
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Enable Custom Branding</p>
                            <p class="text-sm text-text-3">Show custom brand text in clients</p>
                        </div>
                        <input type="checkbox" v-model="form.enable_custom_brand" class="w-5 h-5 text-brand rounded" />
                    </label>
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Mobile File Download</p>
                            <p class="text-sm text-text-3">Allow downloads on mobile clients</p>
                        </div>
                        <input type="checkbox" v-model="form.enable_mobile_file_download" class="w-5 h-5 text-brand rounded" />
                    </label>
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Mobile File Upload</p>
                            <p class="text-sm text-text-3">Allow uploads on mobile clients</p>
                        </div>
                        <input type="checkbox" v-model="form.enable_mobile_file_upload" class="w-5 h-5 text-brand rounded" />
                    </label>
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Allow Download Logs</p>
                            <p class="text-sm text-text-3">Allow clients to download logs</p>
                        </div>
                        <input type="checkbox" v-model="form.allow_download_logs" class="w-5 h-5 text-brand rounded" />
                    </label>
                    <label class="flex items-center justify-between p-4 bg-bg-surface-2 rounded-lg">
                        <div>
                            <p class="font-medium text-text-1">Diagnostics Enabled</p>
                            <p class="text-sm text-text-3">Expose diagnostics and telemetry</p>
                        </div>
                        <input type="checkbox" v-model="form.diagnostics_enabled" class="w-5 h-5 text-brand rounded" />
                    </label>
                </div>
                <p v-else class="text-sm text-text-3">
                    These settings control advanced client behavior and legacy config responses.
                </p>
            </div>

            <!-- Team Membership Defaults -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Globe class="w-5 h-5 text-text-4 mr-2" />
                    <h2 class="text-lg font-semibold text-text-1">Team Membership Defaults</h2>
                </div>
                <div>
                    <label class="block text-sm font-medium text-text-2 mb-1">
                        Default Channels For New Team Members
                    </label>
                    <input
                        v-model="defaultChannelsInput"
                        type="text"
                        class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        placeholder="off-topic, announcements"
                    />
                    <p class="mt-2 text-sm text-text-3">
                        Comma-separated channel names. Leave empty to use the fallback (<code>town-square</code> + <code>off-topic</code>).
                        <code>town-square</code> is always included.
                    </p>
                </div>
            </div>

            <!-- Localization -->
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <Clock class="w-5 h-5 text-text-4 mr-2" />
                    <h2 class="text-lg font-semibold text-text-1">Localization</h2>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Default Locale</label>
                        <select 
                            v-model="form.default_locale"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        >
                            <option value="en">English</option>
                            <option value="es">Spanish</option>
                            <option value="fr">French</option>
                            <option value="de">German</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-text-2 mb-1">Default Timezone</label>
                        <select 
                            v-model="form.default_timezone"
                            class="w-full px-4 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1"
                        >
                            <option value="UTC">UTC</option>
                            <option value="America/New_York">Eastern Time</option>
                            <option value="America/Los_Angeles">Pacific Time</option>
                            <option value="Europe/London">London</option>
                            <option value="Europe/Paris">Paris</option>
                        </select>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
