<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { useAdminStore } from '../../stores/admin';
import { useAuthStore } from '../../stores/auth';
import { Users, Plus, Search, MoreHorizontal, UserCheck, UserX, Edit2, Trash2, AlertTriangle, X, Eraser, RefreshCw, UserPlus, KeyRound, Eye, EyeOff } from 'lucide-vue-next';
import membershipPoliciesApi from '../../api/membershipPolicies';
import CreateUserModal from '../../components/modals/CreateUserModal.vue';
import EditUserModal from '../../components/modals/EditUserModal.vue';
import type { AdminUser } from '../../api/admin';
import adminApi from '../../api/admin';

const adminStore = useAdminStore();
const authStore = useAuthStore();
const searchQuery = ref('');
const statusFilter = ref<'all' | 'active' | 'inactive'>('all');
const includeDeleted = ref(false);
const showCreateModal = ref(false);
const showEditModal = ref(false);
const showPasswordModal = ref(false);
const editingUser = ref<AdminUser | null>(null);
const passwordUser = ref<AdminUser | null>(null);
const activeMenuUserId = ref<string | null>(null);
const showDeleteModal = ref(false);
const deletingUser = ref<AdminUser | null>(null);
const deleteConfirmInput = ref('');
const deleteReason = ref('');
const deleteSubmitting = ref(false);
const deleteError = ref('');
const showWipeModal = ref(false);
const wipingUser = ref<AdminUser | null>(null);
const wipeSubmitting = ref(false);
const wipeError = ref('');

// Password form
const passwordForm = ref({
    newPassword: '',
    confirmPassword: ''
});
const showPassword = ref(false);
const passwordSubmitting = ref(false);
const passwordError = ref('');

// Re-sync membership state
const showResyncModal = ref(false);
const resyncingUser = ref<AdminUser | null>(null);
const resyncSubmitting = ref(false);
const resyncResult = ref<{ teams_processed: number; memberships_applied: number; memberships_failed: number } | null>(null);
const resyncError = ref('');

let searchTimeout: ReturnType<typeof setTimeout>;

onMounted(() => {
    fetchUsers();
});

function fetchUsers() {
    adminStore.fetchUsers({ 
        status: statusFilter.value,
        search: searchQuery.value || undefined,
        include_deleted: includeDeleted.value,
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

watch(includeDeleted, () => {
    fetchUsers();
});

async function handleEdit(user: AdminUser) {
    editingUser.value = user;
    showEditModal.value = true;
    activeMenuUserId.value = null;
}

async function handleSetPassword(user: AdminUser) {
    passwordUser.value = user;
    passwordForm.value = { newPassword: '', confirmPassword: '' };
    passwordError.value = '';
    showPasswordModal.value = true;
    activeMenuUserId.value = null;
}

async function submitPasswordChange() {
    if (!passwordUser.value) return;
    if (passwordForm.value.newPassword !== passwordForm.value.confirmPassword) {
        passwordError.value = 'Passwords do not match';
        return;
    }
    if (passwordForm.value.newPassword.length < 8) {
        passwordError.value = 'Password must be at least 8 characters';
        return;
    }
    
    passwordSubmitting.value = true;
    passwordError.value = '';
    
    try {
        await adminApi.setUserPassword(passwordUser.value.id, passwordForm.value.newPassword);
        showPasswordModal.value = false;
        passwordUser.value = null;
        passwordForm.value = { newPassword: '', confirmPassword: '' };
    } catch (e: any) {
        passwordError.value = e?.response?.data?.error?.message || e?.response?.data?.message || 'Failed to set password';
    } finally {
        passwordSubmitting.value = false;
    }
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

function openDeleteModal(user: AdminUser) {
    deletingUser.value = user;
    deleteConfirmInput.value = '';
    deleteReason.value = '';
    deleteError.value = '';
    deleteSubmitting.value = false;
    showDeleteModal.value = true;
    activeMenuUserId.value = null;
}

function closeDeleteModal() {
    showDeleteModal.value = false;
    deletingUser.value = null;
    deleteConfirmInput.value = '';
    deleteReason.value = '';
    deleteError.value = '';
    deleteSubmitting.value = false;
}

const canDeleteSelectedUser = computed(() => {
    const user = deletingUser.value;
    if (!user) return false;
    const typed = deleteConfirmInput.value.trim();
    return typed === user.username || typed === user.email;
});

const isGlobalAdmin = computed(() => authStore.user?.role === 'system_admin');

async function confirmDeleteUser() {
    if (!deletingUser.value || !canDeleteSelectedUser.value) return;
    deleteSubmitting.value = true;
    deleteError.value = '';
    try {
        await adminStore.deleteUser(deletingUser.value.id, {
            confirm: deleteConfirmInput.value.trim(),
            reason: deleteReason.value.trim() || undefined,
        });
        closeDeleteModal();
        await adminStore.fetchUsers({
            status: statusFilter.value,
            search: searchQuery.value || undefined,
            include_deleted: includeDeleted.value,
        });
    } catch (e: any) {
        deleteError.value = e?.response?.data?.error?.message || e?.response?.data?.message || 'Failed to delete user';
    } finally {
        deleteSubmitting.value = false;
    }
}

function openWipeModal(user: AdminUser) {
    wipingUser.value = user;
    wipeSubmitting.value = false;
    wipeError.value = '';
    showWipeModal.value = true;
    activeMenuUserId.value = null;
}

function closeWipeModal() {
    showWipeModal.value = false;
    wipingUser.value = null;
    wipeSubmitting.value = false;
    wipeError.value = '';
}

async function confirmWipeUser() {
    if (!wipingUser.value) return;
    wipeSubmitting.value = true;
    wipeError.value = '';
    try {
        await adminStore.wipeUser(wipingUser.value.id);
        closeWipeModal();
        await adminStore.fetchUsers({
            status: statusFilter.value,
            search: searchQuery.value || undefined,
            include_deleted: includeDeleted.value,
        });
    } catch (e: any) {
        wipeError.value = e?.response?.data?.error?.message || e?.response?.data?.message || 'Failed to wipe user';
    } finally {
        wipeSubmitting.value = false;
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

// Re-sync membership functions
function openResyncModal(user: AdminUser) {
    resyncingUser.value = user;
    resyncSubmitting.value = false;
    resyncResult.value = null;
    resyncError.value = '';
    showResyncModal.value = true;
    activeMenuUserId.value = null;
}

function closeResyncModal() {
    showResyncModal.value = false;
    resyncingUser.value = null;
    resyncResult.value = null;
    resyncError.value = '';
}

async function confirmResyncUser() {
    if (!resyncingUser.value) return;
    resyncSubmitting.value = true;
    resyncError.value = '';
    try {
        const response = await membershipPoliciesApi.resyncUser(resyncingUser.value.id);
        resyncResult.value = response.data;
    } catch (e: any) {
        resyncError.value = e?.response?.data?.error?.message || e?.response?.data?.message || 'Failed to re-sync user memberships';
    } finally {
        resyncSubmitting.value = false;
    }
}

const roleColors: Record<string, string> = {
    system_admin: 'bg-danger/10 text-danger border-danger/20',
    org_admin: 'bg-primary/10 text-primary border-primary/20',
    team_admin: 'bg-brand/10 text-brand border-brand/20',
    member: 'bg-bg-surface-2 text-text-2 border-border-1',
    guest: 'bg-warning/10 text-warning border-warning/20',
};

const formatDate = (date: string | null) => {
    if (!date) return 'Never';
    return new Date(date).toLocaleDateString();
};

const isDeleted = (user: AdminUser) => Boolean(user.deleted_at);
</script>

<template>
    <div class="space-y-5" @click="closeMenu">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <h1 class="text-lg font-semibold text-text-1">User Management</h1>
                <p class="text-text-3 text-xs mt-0.5">Manage users, roles, and permissions</p>
            </div>
            <button 
                @click.stop="showCreateModal = true"
                class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
            >
                <Plus class="w-3.5 h-3.5" />
                Add User
            </button>
        </div>

        <!-- Filters -->
        <div class="flex items-center gap-3">
            <div class="relative flex-1 max-w-sm">
                <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-4" />
                <input 
                    v-model="searchQuery"
                    type="text"
                    placeholder="Search by name or email..."
                    class="w-full pl-9 pr-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                />
            </div>
            <select 
                v-model="statusFilter"
                class="px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
            >
                <option value="all">All Users</option>
                <option value="active">Active</option>
                <option value="inactive">Inactive</option>
            </select>
            <label class="inline-flex items-center gap-2 text-xs text-text-2">
                <input v-model="includeDeleted" type="checkbox" class="w-3.5 h-3.5 text-brand rounded border-border-1" />
                Show deleted
            </label>
        </div>

        <!-- Users Table -->
        <div class="bg-bg-surface-1 rounded-xl border border-border-1 overflow-hidden shadow-sm">
            <table class="min-w-full divide-y divide-border-1">
                <thead class="bg-bg-surface-2">
                    <tr>
                        <th class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider">User</th>
                        <th class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider">Role</th>
                        <th class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider">Status</th>
                        <th class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider">Last Login</th>
                        <th class="px-4 py-3 text-right text-[10px] font-semibold text-text-3 uppercase tracking-wider">Actions</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-border-1">
                    <tr
                        v-for="user in adminStore.users"
                        :key="user.id"
                        :class="[
                            'hover:bg-bg-surface-2/50 transition-colors',
                            isDeleted(user) ? 'opacity-70' : ''
                        ]"
                    >
                        <td class="px-4 py-3 whitespace-nowrap">
                            <div class="flex items-center gap-3">
                                <div class="w-8 h-8 rounded-full bg-brand flex items-center justify-center text-white text-xs font-bold">
                                    {{ user.username.charAt(0).toUpperCase() }}
                                </div>
                                <div class="min-w-0">
                                    <div class="text-xs font-medium text-text-1 flex items-center gap-1.5">
                                        <span class="truncate">{{ user.display_name || user.username }}</span>
                                        <span v-if="isDeleted(user)" class="px-1.5 py-0.5 text-[9px] rounded bg-danger/10 text-danger font-medium shrink-0">
                                            Deleted
                                        </span>
                                    </div>
                                    <div class="text-[10px] text-text-3 truncate">{{ user.email }}</div>
                                    <div v-if="isDeleted(user) && user.delete_reason" class="text-[9px] text-danger truncate">
                                        {{ user.delete_reason }}
                                    </div>
                                </div>
                            </div>
                        </td>
                        <td class="px-4 py-3 whitespace-nowrap">
                            <span :class="[roleColors[user.role] || roleColors.member, 'px-2 py-0.5 text-[10px] font-medium rounded-full border']">
                                {{ user.role.replace('_', ' ') }}
                            </span>
                        </td>
                        <td class="px-4 py-3 whitespace-nowrap">
                            <span v-if="isDeleted(user)" class="flex items-center gap-1 text-danger text-xs">
                                <Trash2 class="w-3 h-3" /> Deleted
                            </span>
                            <span v-else-if="user.is_active" class="flex items-center gap-1 text-success text-xs">
                                <UserCheck class="w-3 h-3" /> Active
                            </span>
                            <span v-else class="flex items-center gap-1 text-text-3 text-xs">
                                <UserX class="w-3 h-3" /> Inactive
                            </span>
                        </td>
                        <td class="px-4 py-3 whitespace-nowrap text-xs text-text-3">
                            {{ formatDate(user.last_login_at) }}
                        </td>
                        <td class="px-4 py-3 whitespace-nowrap text-right text-xs font-medium relative">
                            <button 
                                @click.stop="handleEdit(user)"
                                class="text-brand hover:text-brand/80 mr-2 p-1 hover:bg-brand/10 rounded transition-colors"
                                title="Edit User"
                            >
                                <Edit2 class="w-3.5 h-3.5" />
                            </button>
                            <div class="relative inline-block text-left">
                                <button 
                                    @click.stop="toggleMenu(user.id)"
                                    class="text-text-4 hover:text-text-2 p-1 hover:bg-bg-surface-2 rounded transition-colors"
                                >
                                    <MoreHorizontal class="w-3.5 h-3.5" />
                                </button>
                                <!-- Dropdown -->
                                <div 
                                    v-if="activeMenuUserId === user.id"
                                    class="absolute right-0 mt-1 w-44 bg-bg-surface-1 rounded-lg shadow-lg py-1 z-10 border border-border-1 ring-1 ring-text-1/5"
                                >
                                    <button
                                        @click.stop="handleSetPassword(user)"
                                        class="flex w-full items-center gap-2 px-3 py-2 text-xs text-text-2 hover:bg-bg-surface-2 transition-colors"
                                    >
                                        <KeyRound class="w-3.5 h-3.5" />
                                        Set Password
                                    </button>
                                    <div class="h-px bg-border-1 my-1"></div>
                                    <button
                                        v-if="user.is_active && !isDeleted(user)"
                                        @click.stop="handleDeactivate(user)"
                                        class="flex w-full items-center gap-2 px-3 py-2 text-xs text-danger hover:bg-danger/10 transition-colors"
                                    >
                                        <UserX class="w-3.5 h-3.5" />
                                        Deactivate User
                                    </button>
                                    <button
                                        v-else-if="!isDeleted(user)"
                                        @click.stop="handleReactivate(user)"
                                        class="flex w-full items-center gap-2 px-3 py-2 text-xs text-success hover:bg-success/10 transition-colors"
                                    >
                                        <UserCheck class="w-3.5 h-3.5" />
                                        Reactivate User
                                    </button>
                                    <button
                                        v-if="isGlobalAdmin && !isDeleted(user)"
                                        @click.stop="openDeleteModal(user)"
                                        class="flex w-full items-center gap-2 px-3 py-2 text-xs text-danger hover:bg-danger/10 transition-colors"
                                    >
                                        <Trash2 class="w-3.5 h-3.5" />
                                        Delete User
                                    </button>
                                    <button
                                        v-if="isGlobalAdmin && isDeleted(user)"
                                        @click.stop="openWipeModal(user)"
                                        class="flex w-full items-center gap-2 px-3 py-2 text-xs text-warning hover:bg-warning/10 transition-colors"
                                    >
                                        <Eraser class="w-3.5 h-3.5" />
                                        Wipe User
                                    </button>
                                    <button
                                        v-if="!isDeleted(user)"
                                        @click.stop="openResyncModal(user)"
                                        class="flex w-full items-center gap-2 px-3 py-2 text-xs text-brand hover:bg-brand/10 transition-colors"
                                    >
                                        <RefreshCw class="w-3.5 h-3.5" />
                                        Re-sync Memberships
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                    <tr v-if="adminStore.users.length === 0 && !adminStore.loading">
                        <td colspan="5" class="px-4 py-12 text-center text-text-3">
                            <Users class="w-10 h-10 mx-auto mb-3 text-text-4" />
                            <p class="text-xs">No users found matching your criteria</p>
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

        <!-- Set Password Modal -->
        <div
            v-if="showPasswordModal && passwordUser"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
            @click.self="showPasswordModal = false"
        >
            <div class="w-full max-w-md rounded-xl border border-border-1 bg-bg-surface-1 shadow-xl">
                <div class="flex items-center justify-between px-5 py-4 border-b border-border-1">
                    <div class="flex items-center gap-2.5">
                        <div class="rounded-lg bg-brand/10 p-1.5">
                            <KeyRound class="w-4 h-4 text-brand" />
                        </div>
                        <div>
                            <h3 class="text-sm font-semibold text-text-1">Set Password</h3>
                            <p class="text-[10px] text-text-3">{{ passwordUser.username }}</p>
                        </div>
                    </div>
                    <button @click="showPasswordModal = false" class="text-text-4 hover:text-text-2 p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4" />
                    </button>
                </div>

                <div class="p-5 space-y-4">
                    <div v-if="passwordError" class="rounded-lg border border-danger/20 bg-danger/10 px-3 py-2 text-xs text-danger">
                        {{ passwordError }}
                    </div>

                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1.5">New Password</label>
                        <div class="relative">
                            <input
                                v-model="passwordForm.newPassword"
                                :type="showPassword ? 'text' : 'password'"
                                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
                                placeholder="Enter new password"
                            />
                            <button 
                                @click="showPassword = !showPassword"
                                class="absolute right-3 top-1/2 -translate-y-1/2 text-text-4 hover:text-text-2"
                            >
                                <Eye v-if="!showPassword" class="w-3.5 h-3.5" />
                                <EyeOff v-else class="w-3.5 h-3.5" />
                            </button>
                        </div>
                    </div>

                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1.5">Confirm Password</label>
                        <input
                            v-model="passwordForm.confirmPassword"
                            :type="showPassword ? 'text' : 'password'"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
                            placeholder="Confirm new password"
                        />
                    </div>

                    <p class="text-[10px] text-text-3">Password must be at least 8 characters long.</p>
                </div>

                <div class="flex items-center justify-end gap-2 px-5 py-4 border-t border-border-1">
                    <button
                        @click="showPasswordModal = false"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button
                        @click="submitPasswordChange"
                        :disabled="passwordSubmitting || !passwordForm.newPassword"
                        class="px-3 py-2 rounded-lg bg-brand hover:bg-brand/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
                    >
                        {{ passwordSubmitting ? 'Saving...' : 'Set Password' }}
                    </button>
                </div>
            </div>
        </div>

        <!-- Delete Modal -->
        <div
            v-if="showDeleteModal && deletingUser"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
            @click.self="closeDeleteModal"
        >
            <div class="w-full max-w-lg rounded-xl border border-border-1 bg-bg-surface-1 shadow-xl">
                <div class="flex items-start justify-between p-5 border-b border-border-1">
                    <div class="flex items-start gap-3">
                        <div class="rounded-lg bg-danger/10 p-2">
                            <AlertTriangle class="w-4 h-4 text-danger" />
                        </div>
                        <div>
                            <h3 class="text-sm font-semibold text-text-1">Delete User</h3>
                            <p class="text-[10px] text-text-3 mt-0.5">
                                This soft-deletes the account, revokes access, and hides it from normal user lists.
                            </p>
                        </div>
                    </div>
                    <button @click="closeDeleteModal" class="text-text-4 hover:text-text-2 p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4" />
                    </button>
                </div>

                <div class="p-5 space-y-4">
                    <div class="rounded-lg border border-danger/20 bg-danger/10 p-3 text-xs text-danger">
                        <p><strong>Consequences:</strong> login is blocked immediately, active sessions are revoked, and the account is marked deleted for audit/history. Messages remain for history.</p>
                    </div>

                    <div class="text-xs text-text-2">
                        <div>Type the exact <span class="font-semibold">username</span> or <span class="font-semibold">email</span> to confirm deletion.</div>
                        <div class="mt-2 rounded-md bg-bg-surface-2 px-3 py-2 font-mono text-[10px] break-all">
                            {{ deletingUser.username }} or {{ deletingUser.email }}
                        </div>
                    </div>

                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1">Confirmation text</label>
                        <input
                            v-model="deleteConfirmInput"
                            type="text"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
                            :placeholder="deletingUser.username"
                        />
                    </div>

                    <div>
                        <label class="block text-xs font-medium text-text-2 mb-1">Reason (optional)</label>
                        <textarea
                            v-model="deleteReason"
                            rows="2"
                            class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none resize-none"
                            placeholder="Why this account is being deleted"
                        />
                    </div>

                    <div v-if="deleteError" class="rounded-lg border border-danger/20 bg-danger/10 px-3 py-2 text-xs text-danger">
                        {{ deleteError }}
                    </div>
                </div>

                <div class="flex items-center justify-end gap-2 p-5 border-t border-border-1">
                    <button
                        @click="closeDeleteModal"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button
                        @click="confirmDeleteUser"
                        :disabled="deleteSubmitting || !canDeleteSelectedUser"
                        class="px-3 py-2 rounded-lg bg-danger hover:bg-danger/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
                    >
                        {{ deleteSubmitting ? 'Deleting...' : 'Delete User' }}
                    </button>
                </div>
            </div>
        </div>

        <!-- Wipe User Modal -->
        <div
            v-if="showWipeModal && wipingUser"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
            @click.self="closeWipeModal"
        >
            <div class="w-full max-w-lg rounded-xl border border-border-1 bg-bg-surface-1 shadow-xl">
                <div class="flex items-start justify-between p-5 border-b border-border-1">
                    <div class="flex items-start gap-3">
                        <div class="rounded-lg bg-warning/10 p-2">
                            <Eraser class="w-4 h-4 text-warning" />
                        </div>
                        <div>
                            <h3 class="text-sm font-semibold text-text-1">Wipe User</h3>
                            <p class="text-[10px] text-text-3 mt-0.5">
                                Permanently delete this user from the database. This action cannot be undone.
                            </p>
                        </div>
                    </div>
                    <button @click="closeWipeModal" class="text-text-4 hover:text-text-2 p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4" />
                    </button>
                </div>

                <div class="p-5 space-y-4">
                    <div class="rounded-lg border border-warning/20 bg-warning/10 p-3 text-xs text-warning">
                        <p><strong>Warning:</strong> This will permanently remove the user record from the database. Only users with no messages can be wiped.</p>
                    </div>

                    <div class="text-xs text-text-2">
                        <p>You are about to wipe user:</p>
                        <div class="mt-2 rounded-md bg-bg-surface-2 px-3 py-2 font-mono text-[10px] break-all">
                            {{ wipingUser.username }} ({{ wipingUser.email }})
                        </div>
                        <p class="mt-2 text-[10px] text-text-3">Deleted at: {{ formatDate(wipingUser.deleted_at ?? null) }}</p>
                    </div>

                    <div v-if="wipeError" class="rounded-lg border border-danger/20 bg-danger/10 px-3 py-2 text-xs text-danger">
                        {{ wipeError }}
                    </div>
                </div>

                <div class="flex items-center justify-end gap-2 p-5 border-t border-border-1">
                    <button
                        @click="closeWipeModal"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button
                        @click="confirmWipeUser"
                        :disabled="wipeSubmitting"
                        class="px-3 py-2 rounded-lg bg-warning hover:bg-warning/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
                    >
                        {{ wipeSubmitting ? 'Wiping...' : 'Permanently Wipe User' }}
                    </button>
                </div>
            </div>
        </div>

        <!-- Re-sync Membership Modal -->
        <div
            v-if="showResyncModal && resyncingUser"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
            @click.self="closeResyncModal"
        >
            <div class="w-full max-w-lg rounded-xl border border-border-1 bg-bg-surface-1 shadow-xl">
                <div class="flex items-start justify-between p-5 border-b border-border-1">
                    <div class="flex items-start gap-3">
                        <div class="rounded-lg bg-brand/10 p-2">
                            <UserPlus class="w-4 h-4 text-brand" />
                        </div>
                        <div>
                            <h3 class="text-sm font-semibold text-text-1">Re-sync Memberships</h3>
                            <p class="text-[10px] text-text-3 mt-0.5">
                                Apply auto-membership policies for this user across all their teams.
                            </p>
                        </div>
                    </div>
                    <button @click="closeResyncModal" class="text-text-4 hover:text-text-2 p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4" />
                    </button>
                </div>

                <div class="p-5 space-y-4">
                    <!-- User Info -->
                    <div class="text-xs text-text-2">
                        <p class="text-[10px] text-text-3 mb-1">User:</p>
                        <div class="rounded-md bg-bg-surface-2 px-3 py-2 font-medium text-text-1">
                            @{{ resyncingUser.username }} ({{ resyncingUser.email }})
                        </div>
                    </div>

                    <!-- Result Display -->
                    <div v-if="resyncResult" class="rounded-lg border border-success/20 bg-success/10 p-4">
                        <h4 class="text-xs font-semibold text-success mb-3">Re-sync Complete</h4>
                        <div class="grid grid-cols-3 gap-3 text-center">
                            <div class="rounded-lg bg-bg-surface-1 p-2">
                                <div class="text-xl font-bold text-success">{{ resyncResult.teams_processed }}</div>
                                <div class="text-[10px] text-success/80">Teams Processed</div>
                            </div>
                            <div class="rounded-lg bg-bg-surface-1 p-2">
                                <div class="text-xl font-bold text-success">{{ resyncResult.memberships_applied }}</div>
                                <div class="text-[10px] text-success/80">Applied</div>
                            </div>
                            <div class="rounded-lg bg-bg-surface-1 p-2">
                                <div class="text-xl font-bold" :class="resyncResult.memberships_failed > 0 ? 'text-danger' : 'text-success'">{{ resyncResult.memberships_failed }}</div>
                                <div class="text-[10px]" :class="resyncResult.memberships_failed > 0 ? 'text-danger' : 'text-success'">Failed</div>
                            </div>
                        </div>
                    </div>

                    <div v-if="resyncError" class="rounded-lg border border-danger/20 bg-danger/10 px-3 py-2 text-xs text-danger">
                        {{ resyncError }}
                    </div>
                </div>

                <div class="flex items-center justify-end gap-2 p-5 border-t border-border-1">
                    <button
                        v-if="!resyncResult"
                        @click="closeResyncModal"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button
                        v-if="resyncResult"
                        @click="closeResyncModal"
                        class="px-3 py-2 rounded-lg bg-text-3 hover:bg-text-2 text-white text-xs font-medium transition-colors"
                    >
                        Close
                    </button>
                    <button
                        v-if="!resyncResult"
                        @click="confirmResyncUser"
                        :disabled="resyncSubmitting"
                        class="px-3 py-2 rounded-lg bg-brand hover:bg-brand/90 disabled:opacity-50 text-white text-xs font-medium flex items-center gap-1.5 transition-colors"
                    >
                        <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': resyncSubmitting }" />
                        {{ resyncSubmitting ? 'Processing...' : 'Run Re-sync' }}
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>
