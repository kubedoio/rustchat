<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { Users, Plus, Search, MoreHorizontal, UserCheck, UserX, Edit2 } from 'lucide-vue-next';
import CreateUserModal from '../../components/modals/CreateUserModal.vue';
import EditUserModal from '../../components/modals/EditUserModal.vue';
import type { AdminUser } from '../../api/admin';

const adminStore = useAdminStore();
const searchQuery = ref('');
const statusFilter = ref<'all' | 'active' | 'inactive'>('all');
const showCreateModal = ref(false);
const showEditModal = ref(false);
const editingUser = ref<AdminUser | null>(null);
const activeMenuUserId = ref<string | null>(null);

let searchTimeout: ReturnType<typeof setTimeout>;

onMounted(() => {
    fetchUsers();
});

function fetchUsers() {
    adminStore.fetchUsers({ 
        status: statusFilter.value,
        search: searchQuery.value || undefined
    });
}

// Watchers for filters
watch(statusFilter, () => {
    fetchUsers();
});

watch(searchQuery, () => {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
        fetchUsers();
    }, 300);
});

async function handleEdit(user: AdminUser) {
    editingUser.value = user;
    showEditModal.value = true;
    activeMenuUserId.value = null;
}

async function handleDeactivate(user: AdminUser) {
    if (!confirm(`Are you sure you want to deactivate ${user.username}?`)) return;
    try {
        await adminStore.deactivateUser(user.id);
        activeMenuUserId.value = null;
    } catch (e) {
        console.error('Failed to deactivate user', e);
    }
}

async function handleReactivate(user: AdminUser) {
    try {
        await adminStore.reactivateUser(user.id);
        activeMenuUserId.value = null;
    } catch (e) {
        console.error('Failed to reactivate user', e);
    }
}

function toggleMenu(userId: string) {
    if (activeMenuUserId.value === userId) {
        activeMenuUserId.value = null;
    } else {
        activeMenuUserId.value = userId;
    }
}

// Close menu when clicking outside (simple implementation)
function closeMenu() {
    activeMenuUserId.value = null;
}

const roleColors: Record<string, string> = {
    system_admin: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
    org_admin: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400',
    team_admin: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400',
    member: 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300',
    guest: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
};

const formatDate = (date: string | null) => {
    if (!date) return 'Never';
    return new Date(date).toLocaleDateString();
};
</script>

<template>
    <div class="space-y-6" @click="closeMenu">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-2xl font-bold text-gray-900 dark:text-white">User Management</h1>
                <p class="text-gray-500 dark:text-gray-400 mt-1">Manage users, roles, and permissions</p>
            </div>
            <button 
                @click.stop="showCreateModal = true"
                class="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg font-medium transition-colors"
            >
                <Plus class="w-5 h-5 mr-2" />
                Add User
            </button>
        </div>

        <!-- Filters -->
        <div class="flex items-center space-x-4">
            <div class="relative flex-1 max-w-md">
                <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
                <input 
                    v-model="searchQuery"
                    type="text"
                    placeholder="Search by name or email..."
                    class="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
                />
            </div>
            <select 
                v-model="statusFilter"
                class="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white"
            >
                <option value="all">All Users</option>
                <option value="active">Active</option>
                <option value="inactive">Inactive</option>
            </select>
        </div>

        <!-- Users Table -->
        <div class="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-gray-200 dark:border-slate-700 overflow-hidden">
            <table class="min-w-full divide-y divide-gray-200 dark:divide-slate-700">
                <thead class="bg-gray-50 dark:bg-slate-900">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">User</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Role</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Status</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Last Login</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Actions</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-200 dark:divide-slate-700">
                    <tr v-for="user in adminStore.users" :key="user.id" class="hover:bg-gray-50 dark:hover:bg-slate-700/50">
                        <td class="px-6 py-4 whitespace-nowrap">
                            <div class="flex items-center">
                                <div class="w-10 h-10 rounded-full bg-indigo-600 flex items-center justify-center text-white font-bold">
                                    {{ user.username.charAt(0).toUpperCase() }}
                                </div>
                                <div class="ml-4">
                                    <div class="text-sm font-medium text-gray-900 dark:text-white">
                                        {{ user.display_name || user.username }}
                                    </div>
                                    <div class="text-sm text-gray-500 dark:text-gray-400">{{ user.email }}</div>
                                </div>
                            </div>
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap">
                            <span :class="[roleColors[user.role] || roleColors.member, 'px-2 py-1 text-xs font-medium rounded-full']">
                                {{ user.role.replace('_', ' ') }}
                            </span>
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap">
                            <span v-if="user.is_active" class="flex items-center text-green-600 dark:text-green-400">
                                <UserCheck class="w-4 h-4 mr-1" /> Active
                            </span>
                            <span v-else class="flex items-center text-gray-500">
                                <UserX class="w-4 h-4 mr-1" /> Inactive
                            </span>
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                            {{ formatDate(user.last_login_at) }}
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium relative">
                            <button 
                                @click.stop="handleEdit(user)"
                                class="text-indigo-600 hover:text-indigo-900 dark:text-indigo-400 mr-3"
                                title="Edit User"
                            >
                                <Edit2 class="w-4 h-4" />
                            </button>
                            <div class="relative inline-block text-left">
                                <button 
                                    @click.stop="toggleMenu(user.id)"
                                    class="text-gray-400 hover:text-gray-600"
                                >
                                    <MoreHorizontal class="w-4 h-4" />
                                </button>
                                <!-- Dropdown -->
                                <div 
                                    v-if="activeMenuUserId === user.id"
                                    class="absolute right-0 mt-2 w-48 bg-white dark:bg-slate-800 rounded-md shadow-lg py-1 z-10 border border-gray-200 dark:border-slate-700 ring-1 ring-black ring-opacity-5"
                                >
                                    <button
                                        v-if="user.is_active"
                                        @click.stop="handleDeactivate(user)"
                                        class="flex w-full items-center px-4 py-2 text-sm text-red-600 hover:bg-gray-100 dark:hover:bg-slate-700"
                                    >
                                        <UserX class="w-4 h-4 mr-2" />
                                        Deactivate User
                                    </button>
                                    <button
                                        v-else
                                        @click.stop="handleReactivate(user)"
                                        class="flex w-full items-center px-4 py-2 text-sm text-green-600 hover:bg-gray-100 dark:hover:bg-slate-700"
                                    >
                                        <UserCheck class="w-4 h-4 mr-2" />
                                        Reactivate User
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                    <tr v-if="adminStore.users.length === 0 && !adminStore.loading">
                        <td colspan="5" class="px-6 py-12 text-center text-gray-500">
                            <Users class="w-12 h-12 mx-auto mb-4 text-gray-300 dark:text-gray-600" />
                            <p>No users found matching your criteria</p>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>

        <CreateUserModal 
            :open="showCreateModal" 
            @close="showCreateModal = false"
            @created="fetchUsers"
        />

        <EditUserModal
            :open="showEditModal"
            :user="editingUser"
            @close="showEditModal = false; editingUser = null"
            @updated="fetchUsers"
        />
    </div>
</template>
