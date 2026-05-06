<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { adminApi, type CallsPluginConfig } from '../../../api/admin';
import { Phone, Server, Globe, AlertCircle, CheckCircle, Save, TestTube } from 'lucide-vue-next';

const loading = ref(true);
const saving = ref(false);
const testing = ref(false);
const saveSuccess = ref(false);
const saveError = ref('');
const testResult = ref<string | null>(null);
const testSuccess = ref(false);

const config = ref<CallsPluginConfig>({
    enabled: false,
    turn_server_enabled: true,
    turn_server_url: '',
    turn_server_username: '',
    turn_server_credential: '',
    udp_port: 8443,
    tcp_port: 8443,
    ice_host_override: null,
    stun_servers: ['stun:stun.l.google.com:19302']
});

const stunServerInput = ref('');

onMounted(async () => {
    try {
        const { data } = await adminApi.getCallsPluginConfig();
        config.value = {
            ...data.settings,
            ice_host_override: data.settings.ice_host_override || '',
            turn_server_credential: data.settings.turn_server_credential || ''
        };
    } catch (e: any) {
        console.error("Failed to load Calls Plugin config", e);
        saveError.value = 'Failed to load configuration';
    } finally {
        loading.value = false;
    }
});

async function saveSettings() {
    saving.value = true;
    saveError.value = '';
    saveSuccess.value = false;

    try {
        // Create a flat copy of config (extract from settings if nested)
        const configToSend = {
            enabled: config.value.enabled ?? true,
            turn_server_enabled: config.value.turn_server_enabled ?? true,
            turn_server_url: config.value.turn_server_url || '',
            turn_server_username: config.value.turn_server_username || '',
            turn_server_credential: config.value.turn_server_credential || null,
            udp_port: config.value.udp_port || 8443,
            tcp_port: config.value.tcp_port || 8443,
            ice_host_override: config.value.ice_host_override || null,
            stun_servers: config.value.stun_servers || ['stun:stun.l.google.com:19302']
        };

        const { data } = await adminApi.updateCallsPluginConfig(configToSend);
        config.value = data.settings;
        saveSuccess.value = true;
        setTimeout(() => saveSuccess.value = false, 3000);
    } catch (e: any) {
        saveError.value = e.response?.data?.message || 'Failed to save configuration';
        console.error(e);
    } finally {
        saving.value = false;
    }
}

async function testConfiguration() {
    testing.value = true;
    testResult.value = null;

    try {
        // Save first, then test
        const configToSend = {
            enabled: config.value.enabled ?? true,
            turn_server_enabled: config.value.turn_server_enabled ?? true,
            turn_server_url: config.value.turn_server_url || '',
            turn_server_username: config.value.turn_server_username || '',
            turn_server_credential: config.value.turn_server_credential || null,
            udp_port: config.value.udp_port || 8443,
            tcp_port: config.value.tcp_port || 8443,
            ice_host_override: config.value.ice_host_override || null,
            stun_servers: config.value.stun_servers || ['stun:stun.l.google.com:19302']
        };
        await adminApi.updateCallsPluginConfig(configToSend);
        const { data } = await adminApi.getCallsPluginConfig();
        testSuccess.value = true;
        testResult.value = `Configuration saved successfully. Plugin ${data.settings.enabled ? 'enabled' : 'disabled'}.`;
    } catch (e: any) {
        testSuccess.value = false;
        testResult.value = e.response?.data?.message || e.message || 'Test failed';
    } finally {
        testing.value = false;
    }
}

function addStunServer() {
    if (stunServerInput.value.trim()) {
        config.value.stun_servers.push(stunServerInput.value.trim());
        stunServerInput.value = '';
    }
}

function removeStunServer(index: number) {
    config.value.stun_servers.splice(index, 1);
}
</script>

<template>
    <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
        <div class="flex items-center justify-between mb-4">
            <div class="flex items-center">
                <Phone class="w-6 h-6 text-indigo-500 mr-3" />
                <div>
                    <h3 class="font-semibold text-gray-900">RustChat Calls Plugin</h3>
                    <p class="text-sm text-gray-500">Configure WebRTC calling infrastructure and TURN servers</p>
                </div>
            </div>

            <input
                type="checkbox"
                v-model="config.enabled"
                class="w-5 h-5 text-indigo-600 rounded"
            />
        </div>

        <div v-if="loading" class="text-gray-500">Loading...</div>
        <div v-else-if="config.enabled" class="mt-4 pt-4 border-t border-gray-200 space-y-6">

            <!-- TURN Server Settings -->
            <div>
                <div class="flex items-center justify-between mb-3">
                    <h4 class="text-sm font-semibold text-gray-900 flex items-center">
                        <Server class="w-4 h-4 mr-2 text-blue-500" />
                        TURN Server Configuration
                    </h4>
                    <label class="flex items-center space-x-2">
                        <input
                            type="checkbox"
                            v-model="config.turn_server_enabled"
                            class="w-4 h-4 text-indigo-600 rounded"
                        />
                        <span class="text-sm text-gray-700">Enable TURN Server</span>
                    </label>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            TURN Server URL
                        </label>
                        <input
                            type="text"
                            v-model="config.turn_server_url"
                            placeholder="turn:turn.example.com:3478"
                            class="w-full px-3 py-2 border rounded-lg"
                        />
                        <p class="text-xs text-gray-500 mt-1">Format: turn:hostname:port or turns:hostname:port</p>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            Username
                        </label>
                        <input
                            type="text"
                            v-model="config.turn_server_username"
                            placeholder="TURN username"
                            class="w-full px-3 py-2 border rounded-lg"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            Credential (Password)
                        </label>
                        <input
                            type="password"
                            v-model="config.turn_server_credential"
                            placeholder="TURN password"
                            class="w-full px-3 py-2 border rounded-lg"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            ICE Host Override
                        </label>
                        <input
                            type="text"
                            v-model="config.ice_host_override"
                            placeholder="Optional: public IP or hostname"
                            class="w-full px-3 py-2 border rounded-lg"
                        />
                        <p class="text-xs text-gray-500 mt-1">Override the public IP address for ICE candidates</p>
                    </div>
                </div>
            </div>

            <!-- Port Configuration -->
            <div>
                <h4 class="text-sm font-semibold text-gray-900 mb-3">Port Configuration</h4>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            UDP Port
                        </label>
                        <input
                            type="number"
                            v-model.number="config.udp_port"
                            min="1"
                            max="65535"
                            class="w-full px-3 py-2 border rounded-lg"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            TCP Port
                        </label>
                        <input
                            type="number"
                            v-model.number="config.tcp_port"
                            min="1"
                            max="65535"
                            class="w-full px-3 py-2 border rounded-lg"
                        />
                    </div>
                </div>
            </div>

            <!-- STUN Servers -->
            <div>
                <h4 class="text-sm font-semibold text-gray-900 mb-3 flex items-center">
                    <Globe class="w-4 h-4 mr-2 text-green-500" />
                    STUN Servers
                </h4>
                <div class="space-y-2">
                    <div
                        v-for="(_, index) in config.stun_servers"
                        :key="index"
                        class="flex items-center gap-2"
                    >
                        <input
                            type="text"
                            v-model="config.stun_servers[index]"
                            :placeholder="'STUN Server ' + (index + 1)"
                            class="flex-1 px-3 py-2 border rounded-lg"
                        />
                        <button
                            @click="removeStunServer(index)"
                            class="px-3 py-2 text-red-600 hover:bg-red-50 rounded-lg"
                        >
                            Remove
                        </button>
                    </div>
                    <div class="flex items-center gap-2">
                        <input
                            type="text"
                            v-model="stunServerInput"
                            placeholder="stun:stun.example.com:19302"
                            class="flex-1 px-3 py-2 border rounded-lg"
                            @keyup.enter="addStunServer"
                        />
                        <button
                            @click="addStunServer"
                            class="px-4 py-2 bg-gray-200 hover:bg-gray-300 rounded-lg text-sm font-medium"
                        >
                            Add Server
                        </button>
                    </div>
                </div>
            </div>

            <!-- Actions -->
            <div class="flex items-center justify-between pt-4 mt-4 border-t border-gray-200">
                <div class="flex items-center gap-4">
                    <button
                        @click="testConfiguration"
                        :disabled="testing"
                        class="inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none"
                    >
                        <TestTube class="w-4 h-4 mr-2" />
                        <span v-if="testing">Testing...</span>
                        <span v-else>Test Configuration</span>
                    </button>

                    <span v-if="saveSuccess" class="flex items-center text-green-600 text-sm">
                        <CheckCircle class="w-4 h-4 mr-1" /> Saved successfully
                    </span>
                </div>

                <button
                    @click="saveSettings"
                    :disabled="saving"
                    class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none"
                >
                    <Save class="w-4 h-4 mr-2" />
                    <span v-if="saving">Saving...</span>
                    <span v-else>Save Configuration</span>
                </button>
            </div>

            <!-- Test Result -->
            <div
                v-if="testResult"
                class="mt-4 p-4 rounded-md"
                :class="testSuccess ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'"
            >
                <pre class="text-sm whitespace-pre-wrap">{{ testResult }}</pre>
            </div>

            <!-- Error Alert -->
            <div
                v-if="saveError"
                class="flex items-center gap-2 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700"
            >
                <AlertCircle class="w-5 h-5 shrink-0" />
                {{ saveError }}
            </div>
        </div>
    </div>
</template>
