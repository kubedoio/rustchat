<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { X, Users, Building2, Hash, AlertCircle } from 'lucide-vue-next';
import type { PolicyWithTargets } from '../../api/membershipPolicies';
import { usersApi } from '../../api/users';

const props = defineProps<{
    policy: PolicyWithTargets | null;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();

// State
const users = ref<{ id: string; username: string; email: string; role: string }[]>([]);
const loading = ref(false);
const selectedUsers = ref<Set<string>>(new Set());

// Computed
const affectedTeams = computed(() => 
    props.policy?.targets.filter(t => t.target_type === 'team') || []
);

const affectedChannels = computed(() => 
    props.policy?.targets.filter(t => t.target_type === 'channel') || []
);

const sourceLabel = computed(() => {
    if (!props.policy) return '';
    const labels: Record<string, string> = {
        all_users: 'All Users',
        auth_service: 'Auth Service Users',
        group: 'Group Members',
        role: 'Role-based Users',
        org: 'Organization Members'
    };
    return labels[props.policy.source_type] || props.policy.source_type;
});

// Load sample users
async function loadUsers() {
    loading.value = true;
    try {
        const response = await usersApi.list({ per_page: 10 });
        users.value = response.data || [];
    } catch (error) {
        // Silent fail for preview
    } finally {
        loading.value = false;
    }
}

// Toggle user selection
function toggleUser(userId: string) {
    if (selectedUsers.value.has(userId)) {
        selectedUsers.value.delete(userId);
    } else {
        selectedUsers.value.add(userId);
    }
}

onMounted(loadUsers);
</script>

<template>
    <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]" @click.self="emit('close')">
        <div class="bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[80vh] flex flex-col">
            <!-- Header -->
            <div class="flex items-center justify-between p-6 border-b border-gray-200">
                <div>
                    <h2 class="text-xl font-bold text-gray-900">Policy Impact Preview</h2>
                    <p class="text-sm text-gray-500 mt-1">{{ policy?.name }}</p>
                </div>
                <button @click="emit('close')" class="p-2 text-gray-500 hover:text-gray-700 rounded-lg">
                    <X class="w-5 h-5" />
                </button>
            </div>

            <!-- Content -->
            <div class="flex-1 overflow-auto p-6 space-y-6">
                <!-- Summary -->
                <div class="grid grid-cols-3 gap-4">
                    <div class="p-4 bg-indigo-50 rounded-lg">
                        <div class="flex items-center text-indigo-600 mb-2">
                            <Users class="w-5 h-5 mr-2" />
                            <span class="text-sm font-medium">Source</span>
                        </div>
                        <p class="text-lg font-semibold text-gray-900">{{ sourceLabel }}</p>
                    </div>
                    
                    <div class="p-4 bg-blue-50 rounded-lg">
                        <div class="flex items-center text-blue-600 mb-2">
                            <Building2 class="w-5 h-5 mr-2" />
                            <span class="text-sm font-medium">Teams</span>
                        </div>
                        <p class="text-lg font-semibold text-gray-900">{{ affectedTeams.length }}</p>
                    </div>
                    
                    <div class="p-4 bg-green-50 rounded-lg">
                        <div class="flex items-center text-green-600 mb-2">
                            <Hash class="w-5 h-5 mr-2" />
                            <span class="text-sm font-medium">Channels</span>
                        </div>
                        <p class="text-lg font-semibold text-gray-900">{{ affectedChannels.length }}</p>
                    </div>
                </div>

                <!-- Affected Targets -->
                <div>
                    <h3 class="text-sm font-medium text-gray-900 uppercase tracking-wider mb-3">Target Memberships</h3>
                    
                    <div class="space-y-2">
                        <div
                            v-for="target in affectedTeams"
                            :key="target.id"
                            class="flex items-center p-3 bg-gray-50 rounded-lg"
                        >
                            <Building2 class="w-5 h-5 text-gray-400 mr-3" />
                            <span class="flex-1">Team (ID: {{ target.target_id.slice(0, 8) }}...)</span>
                            <span class="px-2 py-1 text-xs rounded-full bg-gray-200 capitalize">{{ target.role_mode }}</span>
                        </div>
                        
                        <div
                            v-for="target in affectedChannels"
                            :key="target.id"
                            class="flex items-center p-3 bg-gray-50 rounded-lg"
                        >
                            <Hash class="w-5 h-5 text-gray-400 mr-3" />
                            <span class="flex-1">Channel (ID: {{ target.target_id.slice(0, 8) }}...)</span>
                            <span class="px-2 py-1 text-xs rounded-full bg-gray-200 capitalize">{{ target.role_mode }}</span>
                        </div>
                    </div>
                </div>

                <!-- Sample Users -->
                <div>
                    <h3 class="text-sm font-medium text-gray-900 uppercase tracking-wider mb-3">
                        Sample Affected Users
                    </h3>
                    
                    <div v-if="loading" class="text-center py-4 text-gray-500">
                        Loading users...
                    </div>
                    
                    <div v-else-if="users.length === 0" class="text-center py-4 text-gray-500">
                        <AlertCircle class="w-8 h-8 mx-auto mb-2 text-gray-300" />
                        <p>No users found</p>
                    </div>
                    
                    <div v-else class="space-y-2">
                        <div
                            v-for="user in users"
                            :key="user.id"
                            class="flex items-center p-3 border border-gray-200 rounded-lg hover:bg-gray-50"
                        >
                            <input
                                type="checkbox"
                                :checked="selectedUsers.has(user.id)"
                                @change="toggleUser(user.id)"
                                class="w-4 h-4 text-indigo-600 rounded mr-3"
                            />
                            <div class="flex-1">
                                <p class="font-medium text-gray-900">@{{ user.username }}</p>
                                <p class="text-sm text-gray-500">{{ user.email }}</p>
                            </div>
                            <span class="px-2 py-1 text-xs rounded-full bg-gray-100 capitalize">{{ user.role }}</span>
                        </div>
                    </div>
                    
                    <p class="text-xs text-gray-500 mt-2">
                        Select users above to simulate the policy application (dry-run feature coming soon)
                    </p>
                </div>

                <!-- Warning -->
                <div class="p-4 bg-yellow-50 rounded-lg flex items-start">
                    <AlertCircle class="w-5 h-5 text-yellow-600 mr-3 mt-0.5" />
                    <div class="text-sm text-yellow-800">
                        <p class="font-medium">Important</p>
                        <p>This is a preview only. The actual number of affected users may vary based on current membership states and policy execution timing.</p>
                    </div>
                </div>
            </div>

            <!-- Footer -->
            <div class="flex justify-end p-6 border-t border-gray-200">
                <button
                    @click="emit('close')"
                    class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
                >
                    Close Preview
                </button>
            </div>
        </div>
    </div>
</template>
