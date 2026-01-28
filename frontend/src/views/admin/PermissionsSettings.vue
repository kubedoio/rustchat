<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Shield, Save, RefreshCw } from 'lucide-vue-next';
import adminApi, { type Permission } from '../../api/admin';

const roles = ['system_admin', 'org_admin', 'team_admin', 'member', 'guest'];
const activeRole = ref('member');
const permissions = ref<Permission[]>([]);
const selected = ref(new Set<string>());
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);

const grouped = computed(() => {
    const map: Record<string, Permission[]> = {};
    for (const perm of permissions.value) {
        const key = perm.category || 'general';
        map[key] = map[key] || [];
        map[key].push(perm);
    }
    return Object.entries(map).sort(([a], [b]) => a.localeCompare(b));
});

async function loadPermissions() {
    loading.value = true;
    error.value = null;
    try {
        const res = await adminApi.listPermissions();
        permissions.value = res.data;
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to load permissions';
    } finally {
        loading.value = false;
    }
}

async function loadRole(role: string) {
    loading.value = true;
    error.value = null;
    try {
        const res = await adminApi.getRolePermissions(role);
        selected.value = new Set(res.data);
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to load role permissions';
    } finally {
        loading.value = false;
    }
}

async function saveRole() {
    saving.value = true;
    error.value = null;
    try {
        await adminApi.updateRolePermissions(activeRole.value, Array.from(selected.value));
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to save permissions';
    } finally {
        saving.value = false;
    }
}

function togglePermission(id: string) {
    if (selected.value.has(id)) {
        selected.value.delete(id);
    } else {
        selected.value.add(id);
    }
}

onMounted(async () => {
    await loadPermissions();
    await loadRole(activeRole.value);
});
</script>

<template>
    <div class="space-y-6">
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Permissions</h1>
                <p class="text-gray-500 dark:text-gray-400 mt-1">Configure role-based permissions</p>
            </div>
            <div class="flex items-center gap-2">
                <button
                    @click="loadRole(activeRole)"
                    class="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-700 dark:text-gray-300"
                >
                    <RefreshCw class="w-4 h-4 mr-2" />
                    Refresh
                </button>
                <button
                    @click="saveRole"
                    :disabled="saving"
                    class="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-60 text-white rounded-lg text-sm font-medium"
                >
                    <Save class="w-4 h-4 mr-2" />
                    {{ saving ? 'Saving...' : 'Save' }}
                </button>
            </div>
        </div>

        <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 p-6">
            <div class="flex items-center justify-between mb-6">
                <div class="flex items-center text-gray-700 dark:text-gray-200">
                    <Shield class="w-5 h-5 mr-2 text-gray-400" />
                    <span class="font-medium">Role</span>
                </div>
                <select
                    v-model="activeRole"
                    @change="loadRole(activeRole)"
                    class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-900 text-gray-900 dark:text-white"
                >
                    <option v-for="role in roles" :key="role" :value="role">{{ role }}</option>
                </select>
            </div>

            <div v-if="error" class="text-sm text-red-600 mb-4">{{ error }}</div>

            <div v-if="loading" class="text-gray-500">Loading permissions...</div>

            <div v-else class="space-y-6">
                <div v-for="[category, items] in grouped" :key="category">
                    <h3 class="text-sm font-semibold uppercase text-gray-500 dark:text-gray-400 mb-3">{{ category }}</h3>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                        <label
                            v-for="perm in items"
                            :key="perm.id"
                            class="flex items-start gap-3 p-3 rounded-lg border border-gray-200 dark:border-slate-700 bg-gray-50 dark:bg-slate-900"
                        >
                            <input
                                type="checkbox"
                                :checked="selected.has(perm.id)"
                                @change="togglePermission(perm.id)"
                                class="mt-1"
                            />
                            <div>
                                <div class="text-sm font-medium text-gray-900 dark:text-white">{{ perm.id }}</div>
                                <div class="text-xs text-gray-500 dark:text-gray-400">{{ perm.description || 'No description' }}</div>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
