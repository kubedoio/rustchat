<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Shield, Save, RefreshCw, Check, AlertTriangle } from 'lucide-vue-next';
import adminApi, { type Permission } from '../../api/admin';

const roles = [
    { value: 'system_admin', label: 'System Admin', description: 'Full system access' },
    { value: 'org_admin', label: 'Org Admin', description: 'Organization-wide management' },
    { value: 'team_admin', label: 'Team Admin', description: 'Team and channel management' },
    { value: 'member', label: 'Member', description: 'Standard user permissions' },
    { value: 'guest', label: 'Guest', description: 'Limited external access' },
];
const activeRole = ref('member');
const permissions = ref<Permission[]>([]);
const selected = ref(new Set<string>());
const originalSelected = ref(new Set<string>());
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const saveSuccess = ref(false);

const hasChanges = computed(() => {
    if (selected.value.size !== originalSelected.value.size) return true;
    for (const perm of selected.value) {
        if (!originalSelected.value.has(perm)) return true;
    }
    for (const perm of originalSelected.value) {
        if (!selected.value.has(perm)) return true;
    }
    return false;
});

const grouped = computed(() => {
    const map: Record<string, Permission[]> = {};
    for (const perm of permissions.value) {
        const key = perm.category || 'general';
        map[key] = map[key] || [];
        map[key].push(perm);
    }
    return Object.entries(map).sort(([a], [b]) => a.localeCompare(b));
});

const stats = computed(() => {
    const total = permissions.value.length;
    const granted = selected.value.size;
    const denied = total - granted;
    return { total, granted, denied };
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
    saveSuccess.value = false;
    try {
        const res = await adminApi.getRolePermissions(role);
        selected.value = new Set(res.data);
        originalSelected.value = new Set(res.data);
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to load role permissions';
    } finally {
        loading.value = false;
    }
}

async function saveRole() {
    saving.value = true;
    error.value = null;
    saveSuccess.value = false;
    try {
        await adminApi.updateRolePermissions(activeRole.value, Array.from(selected.value));
        originalSelected.value = new Set(selected.value);
        saveSuccess.value = true;
        setTimeout(() => saveSuccess.value = false, 3000);
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to save permissions';
    } finally {
        saving.value = false;
    }
}

function togglePermission(id: string) {
    const newSelected = new Set(selected.value);
    if (newSelected.has(id)) {
        newSelected.delete(id);
    } else {
        newSelected.add(id);
    }
    selected.value = newSelected;
}

function resetChanges() {
    selected.value = new Set(originalSelected.value);
}

function selectAllInCategory(category: string) {
    const categoryPerms = grouped.value.find(([c]) => c === category)?.[1] || [];
    const newSelected = new Set(selected.value);
    categoryPerms.forEach(p => newSelected.add(p.id));
    selected.value = newSelected;
}

function deselectAllInCategory(category: string) {
    const categoryPerms = grouped.value.find(([c]) => c === category)?.[1] || [];
    const newSelected = new Set(selected.value);
    categoryPerms.forEach(p => newSelected.delete(p.id));
    selected.value = newSelected;
}

onMounted(async () => {
    await loadPermissions();
    await loadRole(activeRole.value);
});
</script>

<template>
    <div class="space-y-5">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-lg font-semibold text-text-1">Permissions</h1>
                <p class="text-text-3 text-xs mt-0.5">Configure role-based access control</p>
            </div>
            <div class="flex items-center gap-2">
                <button
                    v-if="hasChanges"
                    @click="resetChanges"
                    class="flex items-center gap-1.5 px-3 py-2 border border-border-1 rounded-lg text-xs font-medium text-text-2 hover:bg-bg-surface-2 transition-colors"
                >
                    Reset
                </button>
                <button
                    @click="loadRole(activeRole)"
                    class="flex items-center gap-1.5 px-3 py-2 border border-border-1 rounded-lg text-xs font-medium text-text-2 hover:bg-bg-surface-2 transition-colors"
                >
                    <RefreshCw class="w-3.5 h-3.5" />
                    Refresh
                </button>
                <button
                    @click="saveRole"
                    :disabled="saving || !hasChanges"
                    class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
                >
                    <Save class="w-3.5 h-3.5" />
                    {{ saving ? 'Saving...' : 'Save Changes' }}
                </button>
            </div>
        </div>

        <!-- Stats Bar -->
        <div class="grid grid-cols-3 gap-3">
            <div class="bg-bg-surface-1 rounded-lg border border-border-1 p-3">
                <div class="text-[10px] text-text-3 uppercase tracking-wider">Total Permissions</div>
                <div class="text-xl font-semibold text-text-1 mt-1">{{ stats.total }}</div>
            </div>
            <div class="bg-bg-surface-1 rounded-lg border border-border-1 p-3">
                <div class="text-[10px] text-success uppercase tracking-wider">Granted</div>
                <div class="text-xl font-semibold text-success mt-1">{{ stats.granted }}</div>
            </div>
            <div class="bg-bg-surface-1 rounded-lg border border-border-1 p-3">
                <div class="text-[10px] text-danger uppercase tracking-wider">Denied</div>
                <div class="text-xl font-semibold text-danger mt-1">{{ stats.denied }}</div>
            </div>
        </div>

        <!-- Role Selector -->
        <div class="bg-bg-surface-1 rounded-xl border border-border-1 p-5">
            <div class="flex items-center gap-2.5 mb-4">
                <div class="rounded-lg bg-brand/10 p-1.5">
                    <Shield class="w-4 h-4 text-brand" />
                </div>
                <span class="text-sm font-medium text-text-1">Select Role</span>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-5 gap-2">
                <button
                    v-for="role in roles"
                    :key="role.value"
                    @click="activeRole = role.value; loadRole(role.value)"
                    class="text-left p-3 rounded-lg border transition-all"
                    :class="activeRole === role.value ? 'border-brand bg-brand/5' : 'border-border-1 hover:bg-bg-surface-2'"
                >
                    <div class="text-xs font-medium" :class="activeRole === role.value ? 'text-brand' : 'text-text-1'">
                        {{ role.label }}
                    </div>
                    <div class="text-[10px] text-text-3 mt-0.5">{{ role.description }}</div>
                </button>
            </div>
        </div>

        <!-- Alerts -->
        <div v-if="error" class="flex items-center gap-2 p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs">
            <AlertTriangle class="w-4 h-4 shrink-0" />
            {{ error }}
        </div>

        <div v-if="saveSuccess" class="flex items-center gap-2 p-3 bg-success/10 border border-success/20 rounded-lg text-success text-xs">
            <Check class="w-4 h-4 shrink-0" />
            Permissions saved successfully
        </div>

        <!-- Permissions List -->
        <div class="bg-bg-surface-1 rounded-xl border border-border-1 overflow-hidden">
            <div v-if="loading" class="p-8 text-center text-text-3 text-xs">
                <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-brand mx-auto mb-3"></div>
                Loading permissions...
            </div>

            <div v-else-if="permissions.length === 0" class="p-8 text-center text-text-3 text-xs">
                No permissions available
            </div>

            <div v-else class="divide-y divide-border-1">
                <div v-for="[category, items] in grouped" :key="category" class="p-5">
                    <div class="flex items-center justify-between mb-3">
                        <h3 class="text-xs font-semibold uppercase text-text-3 tracking-wider">{{ category }}</h3>
                        <div class="flex items-center gap-1.5">
                            <button 
                                @click="selectAllInCategory(category)"
                                class="px-2 py-1 text-[10px] text-brand hover:bg-brand/10 rounded transition-colors"
                            >
                                Select All
                            </button>
                            <span class="text-text-4">|</span>
                            <button 
                                @click="deselectAllInCategory(category)"
                                class="px-2 py-1 text-[10px] text-danger hover:bg-danger/10 rounded transition-colors"
                            >
                                Deselect All
                            </button>
                        </div>
                    </div>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                        <label
                            v-for="perm in items"
                            :key="perm.id"
                            class="flex items-start gap-2.5 p-2.5 rounded-lg border cursor-pointer transition-all"
                            :class="selected.has(perm.id) ? 'border-brand bg-brand/5' : 'border-border-1 hover:bg-bg-surface-2'"
                        >
                            <input
                                type="checkbox"
                                :checked="selected.has(perm.id)"
                                @change="togglePermission(perm.id)"
                                class="w-4 h-4 text-brand rounded border-border-1 mt-0.5 shrink-0"
                            />
                            <div class="min-w-0">
                                <div class="text-xs font-medium text-text-1 truncate">{{ perm.id }}</div>
                                <div class="text-[10px] text-text-3">{{ perm.description || 'No description' }}</div>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
        </div>

        <!-- Unsaved Changes Warning -->
        <div 
            v-if="hasChanges" 
            class="fixed bottom-6 right-6 flex items-center gap-3 px-4 py-3 bg-bg-surface-1 border border-brand rounded-lg shadow-lg"
        >
            <span class="text-xs text-text-2">You have unsaved changes</span>
            <button 
                @click="saveRole"
                :disabled="saving"
                class="px-3 py-1.5 bg-brand hover:bg-brand/90 disabled:opacity-50 text-white rounded text-xs font-medium transition-colors"
            >
                {{ saving ? 'Saving...' : 'Save Now' }}
            </button>
        </div>
    </div>
</template>
