<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { 
    Users, 
    MessageSquare, 
    Search, 
    Eye, 
    Trash2, 
    ChevronLeft,
    ChevronRight,
    Hash,
    Lock,
    Building2,
    ArrowLeft,
    Plus,
    Edit2,
    UserPlus,
    UserMinus,
    X,
    CheckCircle,
    Globe,
    MoreHorizontal
} from 'lucide-vue-next';
import adminApi, { type AdminTeam, type AdminChannel, type AdminUser } from '../../api/admin';
import BaseInput from '../../components/atomic/BaseInput.vue';

// State
const teams = ref<AdminTeam[]>([]);
const totalTeams = ref(0);
const loading = ref(true);
const search = ref('');
const page = ref(1);
const perPage = 10;

const selectedTeam = ref<AdminTeam | null>(null);
const activeTab = ref<'channels' | 'members'>('channels');

// Team Form State
const showCreateTeamModal = ref(false);
const showEditTeamModal = ref(false);
const editingTeam = ref<AdminTeam | null>(null);
const teamForm = ref({
    name: '',
    display_name: '',
    description: '',
    is_public: true,
    allow_open_invite: false
});

// Channels State
const teamChannels = ref<AdminChannel[]>([]);
const channelsLoading = ref(false);
const showCreateChannelModal = ref(false);
const showEditChannelModal = ref(false);
const editingChannel = ref<AdminChannel | null>(null);
const channelForm = ref({
    name: '',
    display_name: '',
    purpose: '',
    type: 'public' as 'public' | 'private'
});

// Members State
const teamMembers = ref<any[]>([]);
const membersLoading = ref(false);
const showAddMemberModal = ref(false);
const memberSearch = ref('');
const memberSearchResults = ref<AdminUser[]>([]);
const searchingMembers = ref(false);
const activeMenuMemberId = ref<string | null>(null);

// Actions
async function fetchTeams() {
    loading.value = true;
    try {
        const { data } = await adminApi.listTeams({
            page: page.value,
            per_page: perPage,
            search: search.value
        });
        teams.value = data.teams;
        totalTeams.value = data.total;
    } catch (e) {
        console.error('Failed to fetch teams', e);
    } finally {
        loading.value = false;
    }
}

async function viewTeamDetails(team: AdminTeam) {
    selectedTeam.value = team;
    activeTab.value = 'channels';
    await fetchChannels();
}

async function fetchChannels() {
    if (!selectedTeam.value) return;
    channelsLoading.value = true;
    try {
        const { data } = await adminApi.listChannels({
            team_id: selectedTeam.value.id,
            per_page: 100
        });
        teamChannels.value = data.channels;
    } catch (e) {
        console.error('Failed to fetch channels', e);
    } finally {
        channelsLoading.value = false;
    }
}

async function fetchMembers() {
    if (!selectedTeam.value) return;
    membersLoading.value = true;
    try {
        const { data } = await adminApi.listTeamMembers(selectedTeam.value.id);
        teamMembers.value = data;
    } catch (e) {
        console.error('Failed to fetch members', e);
    } finally {
        membersLoading.value = false;
    }
}

function closeDetails() {
    selectedTeam.value = null;
    teamChannels.value = [];
    teamMembers.value = [];
}

// Team Actions
function openCreateTeam() {
    teamForm.value = {
        name: '',
        display_name: '',
        description: '',
        is_public: true,
        allow_open_invite: false
    };
    showCreateTeamModal.value = true;
}

function openEditTeam(team: AdminTeam) {
    editingTeam.value = team;
    teamForm.value = {
        name: team.name,
        display_name: team.display_name || '',
        description: team.description || '',
        is_public: team.is_public,
        allow_open_invite: team.allow_open_invite
    };
    showEditTeamModal.value = true;
}

async function createTeam() {
    try {
        await adminApi.createTeam({
            name: teamForm.value.name,
            display_name: teamForm.value.display_name,
            description: teamForm.value.description,
            is_public: teamForm.value.is_public,
            allow_open_invite: teamForm.value.allow_open_invite
        });
        showCreateTeamModal.value = false;
        await fetchTeams();
    } catch (e: any) {
        console.error('Failed to create team', e);
        alert(e?.response?.data?.message || 'Failed to create team');
    }
}

async function updateTeam() {
    if (!editingTeam.value) return;
    try {
        await adminApi.updateTeam(editingTeam.value.id, {
            display_name: teamForm.value.display_name,
            description: teamForm.value.description,
            is_public: teamForm.value.is_public,
            allow_open_invite: teamForm.value.allow_open_invite
        });
        showEditTeamModal.value = false;
        editingTeam.value = null;
        await fetchTeams();
        if (selectedTeam.value?.id) {
            const { data } = await adminApi.getTeam(selectedTeam.value.id);
            selectedTeam.value = data;
        }
    } catch (e: any) {
        console.error('Failed to update team', e);
        alert(e?.response?.data?.message || 'Failed to update team');
    }
}

async function deleteTeam(team: AdminTeam) {
    if (!confirm(`Are you sure you want to delete the team "${team.display_name || team.name}"? This will permanently delete all channels, messages, and files.`)) {
        return;
    }

    try {
        await adminApi.deleteTeam(team.id);
        await fetchTeams();
    } catch (e) {
        console.error('Failed to delete team', e);
        alert('Failed to delete team');
    }
}

// Channel Actions
function openCreateChannel() {
    channelForm.value = { name: '', display_name: '', purpose: '', type: 'public' };
    showCreateChannelModal.value = true;
}

async function createChannel() {
    if (!selectedTeam.value) return;
    try {
        await adminApi.createChannel({
            team_id: selectedTeam.value.id,
            name: channelForm.value.name,
            display_name: channelForm.value.display_name,
            purpose: channelForm.value.purpose,
            channel_type: channelForm.value.type
        });
        showCreateChannelModal.value = false;
        await fetchChannels();
    } catch (e) {
        console.error('Failed to create channel', e);
        alert('Failed to create channel');
    }
}

function openEditChannel(channel: AdminChannel) {
    editingChannel.value = channel;
    channelForm.value = {
        name: channel.name,
        display_name: channel.display_name || '',
        purpose: channel.purpose || '',
        type: channel.channel_type === 'public' || channel.channel_type === 'private' ? channel.channel_type : 'public'
    };
    showEditChannelModal.value = true;
}

async function updateChannel() {
    if (!editingChannel.value) return;
    try {
        await adminApi.updateChannel(editingChannel.value.id, {
            display_name: channelForm.value.display_name,
            purpose: channelForm.value.purpose
        });
        showEditChannelModal.value = false;
        await fetchChannels();
    } catch (e) {
        console.error('Failed to update channel', e);
        alert('Failed to update channel');
    }
}

async function deleteChannel(channel: AdminChannel) {
    if (!confirm(`Are you sure you want to delete the channel "#${channel.name}"?`)) {
        return;
    }

    try {
        await adminApi.deleteChannel(channel.id);
        await fetchChannels();
    } catch (e) {
        console.error('Failed to delete channel', e);
        alert('Failed to delete channel');
    }
}

// Member Actions
async function searchUsers() {
    if (!memberSearch.value) {
        memberSearchResults.value = [];
        return;
    }
    searchingMembers.value = true;
    try {
        const { data } = await adminApi.listUsers({ search: memberSearch.value, per_page: 5 });
        // Filter out existing members
        const currentIds = new Set(teamMembers.value.map(m => m.user_id));
        memberSearchResults.value = data.users.filter(u => !currentIds.has(u.id));
    } catch (e) {
        console.error('Failed to search users', e);
    } finally {
        searchingMembers.value = false;
    }
}

async function addMember(user: AdminUser) {
    if (!selectedTeam.value) return;
    try {
        await adminApi.addTeamMember(selectedTeam.value.id, user.id);
        memberSearchResults.value = memberSearchResults.value.filter(u => u.id !== user.id);
        await fetchMembers();
    } catch (e) {
        console.error('Failed to add member', e);
        alert('Failed to add member');
    }
}

async function removeMember(member: any) {
    if (!selectedTeam.value) return;
    if (!confirm(`Remove ${member.username} from the team?`)) return;
    try {
        await adminApi.removeTeamMember(selectedTeam.value.id, member.user_id);
        await fetchMembers();
    } catch (e) {
        console.error('Failed to remove member', e);
        alert('Failed to remove member');
    }
}

async function changeMemberRole(member: any, newRole: string) {
    if (!selectedTeam.value) return;
    try {
        await adminApi.updateTeamMemberRole(selectedTeam.value.id, member.user_id, newRole);
        await fetchMembers();
    } catch (e) {
        console.error('Failed to change member role', e);
        alert('Failed to change member role');
    }
}

function toggleMemberMenu(userId: string) {
    if (activeMenuMemberId.value === userId) {
        activeMenuMemberId.value = null;
    } else {
        activeMenuMemberId.value = userId;
    }
}

// Watchers
watch([search, page], () => {
    fetchTeams();
});

watch(activeTab, (tab) => {
    if (tab === 'members') fetchMembers();
    else fetchChannels();
});

let searchTimeout: ReturnType<typeof setTimeout>;
watch(memberSearch, () => {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(searchUsers, 300);
});

onMounted(fetchTeams);
</script>

<template>
    <div class="space-y-5">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <div v-if="selectedTeam" class="flex items-center gap-2 mb-1">
                    <button @click="closeDetails" class="text-text-3 hover:text-text-1 p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <ArrowLeft class="w-3.5 h-3.5" />
                    </button>
                    <span class="text-text-4 text-xs">Teams</span>
                </div>
                <h1 class="text-lg font-semibold text-text-1">
                    {{ selectedTeam ? (selectedTeam.display_name || selectedTeam.name) : 'Teams & Channels' }}
                </h1>
                <p class="text-text-3 text-xs mt-0.5">
                    {{ selectedTeam ? 'Manage channels and members for this team' : 'Manage teams, channels, and memberships' }}
                </p>
            </div>
            <button 
                v-if="!selectedTeam"
                @click="openCreateTeam"
                class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
            >
                <Plus class="w-3.5 h-3.5" />
                Create Team
            </button>
        </div>

        <!-- Team List View -->
        <div v-if="!selectedTeam" class="space-y-4">
            <!-- Search & Filters -->
            <div class="flex items-center gap-3">
                <div class="relative flex-1 max-w-sm">
                    <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-4" />
                    <input 
                        v-model="search"
                        type="text" 
                        placeholder="Search teams..." 
                        class="w-full pl-9 pr-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                    />
                </div>
            </div>

            <!-- Teams Table -->
            <div class="bg-bg-surface-1 rounded-xl border border-border-1 overflow-hidden shadow-sm">
                <div class="overflow-x-auto">
                    <table class="w-full text-left">
                        <thead>
                            <tr class="bg-bg-surface-2 border-b border-border-1">
                                <th class="px-4 py-3 text-[10px] font-semibold text-text-3 uppercase tracking-wider">Team</th>
                                <th class="px-4 py-3 text-[10px] font-semibold text-text-3 uppercase tracking-wider">Visibility</th>
                                <th class="px-4 py-3 text-[10px] font-semibold text-text-3 uppercase tracking-wider text-center">Members</th>
                                <th class="px-4 py-3 text-[10px] font-semibold text-text-3 uppercase tracking-wider text-center">Channels</th>
                                <th class="px-4 py-3 text-[10px] font-semibold text-text-3 uppercase tracking-wider">Created</th>
                                <th class="px-4 py-3 text-[10px] font-semibold text-text-3 uppercase tracking-wider text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-border-1">
                            <tr v-if="loading" v-for="i in 3" :key="i" class="animate-pulse">
                                <td v-for="j in 6" :key="j" class="px-4 py-3">
                                    <div class="h-3 bg-bg-surface-2 rounded w-2/3"></div>
                                </td>
                            </tr>
                            <tr v-else-if="teams.length === 0" class="text-center">
                                <td colspan="6" class="px-4 py-12">
                                    <div class="flex flex-col items-center justify-center">
                                        <Building2 class="w-10 h-10 text-text-4 mb-3" />
                                        <p class="text-text-3 text-xs">No teams found</p>
                                    </div>
                                </td>
                            </tr>
                            <tr v-for="team in teams" :key="team.id" class="hover:bg-bg-surface-2/50 transition-colors">
                                <td class="px-4 py-3">
                                    <div class="flex items-center gap-3">
                                        <div class="w-8 h-8 rounded-lg bg-brand/10 flex items-center justify-center text-brand text-xs font-bold">
                                            {{ (team.display_name || team.name).charAt(0).toUpperCase() }}
                                        </div>
                                        <div class="min-w-0">
                                            <div class="text-xs font-medium text-text-1 truncate">{{ team.display_name || team.name }}</div>
                                            <div class="text-[10px] text-text-3">@{{ team.name }}</div>
                                        </div>
                                    </div>
                                </td>
                                <td class="px-4 py-3">
                                    <span v-if="team.is_public" class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-success/10 text-success border border-success/20">
                                        <Globe class="w-3 h-3" />
                                        Public
                                    </span>
                                    <span v-else class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-primary/10 text-primary border border-primary/20">
                                        <Lock class="w-3 h-3" />
                                        Private
                                    </span>
                                </td>
                                <td class="px-4 py-3 text-center">
                                    <div class="flex items-center justify-center gap-1 text-text-2 text-xs">
                                        <Users class="w-3.5 h-3.5" />
                                        <span>{{ team.members_count }}</span>
                                    </div>
                                </td>
                                <td class="px-4 py-3 text-center">
                                    <div class="flex items-center justify-center gap-1 text-text-2 text-xs">
                                        <MessageSquare class="w-3.5 h-3.5" />
                                        <span>{{ team.channels_count }}</span>
                                    </div>
                                </td>
                                <td class="px-4 py-3 text-xs text-text-3">
                                    {{ new Date(team.created_at).toLocaleDateString() }}
                                </td>
                                <td class="px-4 py-3 text-right">
                                    <div class="flex items-center justify-end gap-1">
                                        <button 
                                            @click="openEditTeam(team)"
                                            class="p-1.5 text-text-4 hover:text-brand hover:bg-brand/10 rounded-lg transition-colors"
                                            title="Edit Team"
                                        >
                                            <Edit2 class="w-3.5 h-3.5" />
                                        </button>
                                        <button 
                                            @click="viewTeamDetails(team)"
                                            class="p-1.5 text-text-4 hover:text-primary hover:bg-primary/10 rounded-lg transition-colors"
                                            title="View Details"
                                        >
                                            <Eye class="w-3.5 h-3.5" />
                                        </button>
                                        <button 
                                            @click="deleteTeam(team)"
                                            class="p-1.5 text-text-4 hover:text-danger hover:bg-danger/10 rounded-lg transition-colors"
                                            title="Delete Team"
                                        >
                                            <Trash2 class="w-3.5 h-3.5" />
                                        </button>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <!-- Pagination -->
                <div class="px-4 py-3 flex items-center justify-between border-t border-border-1 bg-bg-surface-2/30">
                    <div class="text-xs text-text-3">
                        Showing {{ ((page - 1) * perPage) + 1 }} to {{ Math.min(page * perPage, totalTeams) }} of {{ totalTeams }} teams
                    </div>
                    <div class="flex items-center gap-1.5">
                        <button 
                            @click="page--"
                            :disabled="page === 1"
                            class="p-1.5 border border-border-1 rounded-lg hover:bg-bg-surface-1 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <ChevronLeft class="w-3.5 h-3.5" />
                        </button>
                        <button 
                            @click="page++"
                            :disabled="page * perPage >= totalTeams"
                            class="p-1.5 border border-border-1 rounded-lg hover:bg-bg-surface-1 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <ChevronRight class="w-3.5 h-3.5" />
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Team Details View -->
        <div v-else class="space-y-5">
            <div class="bg-bg-surface-1 rounded-xl border border-border-1 shadow-sm overflow-hidden">
                <!-- Details Header with Tabs -->
                <div class="border-b border-border-1">
                    <div class="px-5 py-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <h2 class="text-base font-semibold text-text-1">{{ selectedTeam.display_name || selectedTeam.name }}</h2>
                                <div class="flex items-center gap-3 mt-1 text-xs text-text-3">
                                    <span>@{{ selectedTeam.name }}</span>
                                    <span class="w-px h-3 bg-border-1"></span>
                                    <span class="flex items-center gap-1">
                                        <Users class="w-3 h-3" />
                                        {{ selectedTeam.members_count }} members
                                    </span>
                                    <span class="w-px h-3 bg-border-1"></span>
                                    <span class="flex items-center gap-1">
                                        <MessageSquare class="w-3 h-3" />
                                        {{ selectedTeam.channels_count }} channels
                                    </span>
                                </div>
                            </div>
                            <button 
                                @click="openEditTeam(selectedTeam)"
                                class="flex items-center gap-1.5 px-3 py-2 bg-bg-surface-2 hover:bg-border-1 text-text-2 rounded-lg text-xs font-medium transition-colors"
                            >
                                <Edit2 class="w-3.5 h-3.5" />
                                Edit Team
                            </button>
                        </div>
                    </div>
                    <div class="flex px-5 gap-6">
                        <button 
                            @click="activeTab = 'channels'"
                            class="pb-3 text-xs font-medium border-b-2 transition-colors"
                            :class="activeTab === 'channels' ? 'border-brand text-brand' : 'border-transparent text-text-3 hover:text-text-2'"
                        >
                            Channels
                        </button>
                        <button 
                            @click="activeTab = 'members'"
                            class="pb-3 text-xs font-medium border-b-2 transition-colors"
                            :class="activeTab === 'members' ? 'border-brand text-brand' : 'border-transparent text-text-3 hover:text-text-2'"
                        >
                            Members
                        </button>
                    </div>
                </div>

                <!-- Channels Tab -->
                <div v-if="activeTab === 'channels'" class="p-5">
                    <div class="flex justify-between items-center mb-4">
                        <h3 class="font-semibold text-text-1 text-sm">Team Channels</h3>
                        <button 
                            @click="openCreateChannel"
                            class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
                        >
                            <Plus class="w-3.5 h-3.5" />
                            Create Channel
                        </button>
                    </div>

                    <div v-if="channelsLoading" class="p-12 text-center">
                        <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-brand mx-auto"></div>
                    </div>
                    
                    <div v-else-if="teamChannels.length === 0" class="p-12 text-center text-text-3 text-xs">
                        No channels in this team.
                    </div>

                    <div v-else class="divide-y divide-border-1 border border-border-1 rounded-lg">
                        <div v-for="channel in teamChannels" :key="channel.id" class="px-4 py-3 flex items-center justify-between hover:bg-bg-surface-2/50 transition-colors">
                            <div class="flex items-center gap-3">
                                <div v-if="channel.channel_type === 'public'" class="text-text-4">
                                    <Hash class="w-4 h-4" />
                                </div>
                                <div v-else class="text-primary">
                                    <Lock class="w-4 h-4" />
                                </div>
                                <div>
                                    <div class="font-medium text-text-1 text-xs flex items-center gap-2">
                                        {{ channel.display_name || channel.name }}
                                        <span v-if="channel.is_archived" class="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase bg-bg-surface-2 text-text-3">Archived</span>
                                    </div>
                                    <p class="text-[10px] text-text-3">{{ channel.purpose || 'No purpose set' }}</p>
                                </div>
                            </div>
                            <div class="flex items-center gap-1">
                                <button 
                                    @click="openEditChannel(channel)"
                                    class="p-1.5 text-text-4 hover:text-brand hover:bg-brand/10 rounded-lg transition-colors"
                                    title="Edit Channel"
                                >
                                    <Edit2 class="w-3.5 h-3.5" />
                                </button>
                                <button 
                                    @click="deleteChannel(channel)"
                                    class="p-1.5 text-text-4 hover:text-danger hover:bg-danger/10 rounded-lg transition-colors"
                                    title="Delete Channel"
                                >
                                    <Trash2 class="w-3.5 h-3.5" />
                                </button>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Members Tab -->
                <div v-if="activeTab === 'members'" class="p-5">
                    <div class="flex justify-between items-center mb-4">
                        <h3 class="font-semibold text-text-1 text-sm">Team Members</h3>
                        <button 
                            @click="showAddMemberModal = true"
                            class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
                        >
                            <UserPlus class="w-3.5 h-3.5" />
                            Add Member
                        </button>
                    </div>

                    <div v-if="membersLoading" class="p-12 text-center">
                        <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-brand mx-auto"></div>
                    </div>

                    <div v-else-if="teamMembers.length === 0" class="p-12 text-center text-text-3 text-xs">
                        No members found.
                    </div>

                    <div v-else class="divide-y divide-border-1 border border-border-1 rounded-lg">
                        <div v-for="member in teamMembers" :key="member.user_id" class="px-4 py-3 flex items-center justify-between hover:bg-bg-surface-2/50 transition-colors">
                            <div class="flex items-center gap-3">
                                <div class="w-8 h-8 rounded-full bg-bg-surface-2 flex items-center justify-center text-text-2 text-xs font-bold">
                                    {{ (member.display_name || member.username).charAt(0).toUpperCase() }}
                                </div>
                                <div>
                                    <div class="font-medium text-text-1 text-xs">{{ member.display_name || member.username }}</div>
                                    <div class="text-[10px] text-text-3">@{{ member.username }} • {{ member.role }}</div>
                                </div>
                            </div>
                            <div class="flex items-center gap-1">
                                <div class="relative">
                                    <button 
                                        @click="toggleMemberMenu(member.user_id)"
                                        class="p-1.5 text-text-4 hover:text-text-2 hover:bg-bg-surface-2 rounded-lg transition-colors"
                                    >
                                        <MoreHorizontal class="w-3.5 h-3.5" />
                                    </button>
                                    <div 
                                        v-if="activeMenuMemberId === member.user_id"
                                        class="absolute right-0 mt-1 w-36 bg-bg-surface-1 rounded-lg shadow-lg py-1 z-10 border border-border-1"
                                    >
                                        <button
                                            @click="changeMemberRole(member, 'admin'); activeMenuMemberId = null"
                                            class="flex w-full items-center gap-2 px-3 py-2 text-xs text-text-2 hover:bg-bg-surface-2 transition-colors"
                                            :class="{ 'text-brand': member.role === 'admin' }"
                                        >
                                            <CheckCircle v-if="member.role === 'admin'" class="w-3.5 h-3.5" />
                                            <span v-else class="w-3.5 h-3.5"></span>
                                            Admin
                                        </button>
                                        <button
                                            @click="changeMemberRole(member, 'member'); activeMenuMemberId = null"
                                            class="flex w-full items-center gap-2 px-3 py-2 text-xs text-text-2 hover:bg-bg-surface-2 transition-colors"
                                            :class="{ 'text-brand': member.role === 'member' }"
                                        >
                                            <CheckCircle v-if="member.role === 'member'" class="w-3.5 h-3.5" />
                                            <span v-else class="w-3.5 h-3.5"></span>
                                            Member
                                        </button>
                                        <div class="h-px bg-border-1 my-1"></div>
                                        <button
                                            @click="removeMember(member); activeMenuMemberId = null"
                                            class="flex w-full items-center gap-2 px-3 py-2 text-xs text-danger hover:bg-danger/10 transition-colors"
                                        >
                                            <UserMinus class="w-3.5 h-3.5" />
                                            Remove
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Create Team Modal -->
        <div v-if="showCreateTeamModal" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showCreateTeamModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-md p-5 space-y-4">
                <div class="flex justify-between items-center">
                    <h3 class="text-sm font-bold text-text-1">Create Team</h3>
                    <button @click="showCreateTeamModal = false" class="p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                <BaseInput v-model="teamForm.name" label="Team URL Name" placeholder="e.g. engineering" required />
                <BaseInput v-model="teamForm.display_name" label="Display Name" placeholder="e.g. Engineering Team" />
                <BaseInput v-model="teamForm.description" label="Description" placeholder="Team description" />
                <div class="space-y-2">
                    <label class="flex items-center gap-2 text-xs text-text-2">
                        <input v-model="teamForm.is_public" type="checkbox" class="w-4 h-4 text-brand rounded border-border-1" />
                        <span>Public team (visible to all users)</span>
                    </label>
                    <label class="flex items-center gap-2 text-xs text-text-2">
                        <input v-model="teamForm.allow_open_invite" type="checkbox" class="w-4 h-4 text-brand rounded border-border-1" />
                        <span>Allow open invite (any member can invite)</span>
                    </label>
                </div>
                <div class="flex justify-end gap-2 pt-2">
                    <button 
                        @click="showCreateTeamModal = false"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button 
                        @click="createTeam"
                        :disabled="!teamForm.name"
                        class="px-3 py-2 rounded-lg bg-brand hover:bg-brand/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
                    >
                        Create Team
                    </button>
                </div>
            </div>
        </div>

        <!-- Edit Team Modal -->
        <div v-if="showEditTeamModal" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showEditTeamModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-md p-5 space-y-4">
                <div class="flex justify-between items-center">
                    <h3 class="text-sm font-bold text-text-1">Edit Team</h3>
                    <button @click="showEditTeamModal = false" class="p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                <BaseInput v-model="teamForm.name" label="Team URL Name" disabled />
                <BaseInput v-model="teamForm.display_name" label="Display Name" placeholder="e.g. Engineering Team" />
                <BaseInput v-model="teamForm.description" label="Description" placeholder="Team description" />
                <div class="space-y-2">
                    <label class="flex items-center gap-2 text-xs text-text-2">
                        <input v-model="teamForm.is_public" type="checkbox" class="w-4 h-4 text-brand rounded border-border-1" />
                        <span>Public team (visible to all users)</span>
                    </label>
                    <label class="flex items-center gap-2 text-xs text-text-2">
                        <input v-model="teamForm.allow_open_invite" type="checkbox" class="w-4 h-4 text-brand rounded border-border-1" />
                        <span>Allow open invite (any member can invite)</span>
                    </label>
                </div>
                <div class="flex justify-end gap-2 pt-2">
                    <button 
                        @click="showEditTeamModal = false"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button 
                        @click="updateTeam"
                        class="px-3 py-2 rounded-lg bg-brand hover:bg-brand/90 text-white text-xs font-medium transition-colors"
                    >
                        Save Changes
                    </button>
                </div>
            </div>
        </div>

        <!-- Create Channel Modal -->
        <div v-if="showCreateChannelModal" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showCreateChannelModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-md p-5 space-y-4">
                <div class="flex justify-between items-center">
                    <h3 class="text-sm font-bold text-text-1">Create Channel</h3>
                    <button @click="showCreateChannelModal = false" class="p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                <BaseInput v-model="channelForm.name" label="Name" placeholder="e.g. general" required />
                <BaseInput v-model="channelForm.display_name" label="Display Name" placeholder="e.g. General" />
                <BaseInput v-model="channelForm.purpose" label="Purpose" placeholder="Channel purpose" />
                <div>
                    <label class="block text-xs font-medium text-text-2 mb-1.5">Type</label>
                    <select v-model="channelForm.type" class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-2 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none">
                        <option value="public">Public</option>
                        <option value="private">Private</option>
                    </select>
                </div>
                <div class="flex justify-end gap-2 pt-2">
                    <button 
                        @click="showCreateChannelModal = false"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button 
                        @click="createChannel"
                        :disabled="!channelForm.name"
                        class="px-3 py-2 rounded-lg bg-brand hover:bg-brand/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
                    >
                        Create
                    </button>
                </div>
            </div>
        </div>

        <!-- Edit Channel Modal -->
        <div v-if="showEditChannelModal" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showEditChannelModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-md p-5 space-y-4">
                <div class="flex justify-between items-center">
                    <h3 class="text-sm font-bold text-text-1">Edit Channel</h3>
                    <button @click="showEditChannelModal = false" class="p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                <BaseInput v-model="channelForm.name" label="Name" disabled />
                <BaseInput v-model="channelForm.display_name" label="Display Name" />
                <BaseInput v-model="channelForm.purpose" label="Purpose" />
                <div class="flex justify-end gap-2 pt-2">
                    <button 
                        @click="showEditChannelModal = false"
                        class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
                    >
                        Cancel
                    </button>
                    <button 
                        @click="updateChannel"
                        class="px-3 py-2 rounded-lg bg-brand hover:bg-brand/90 text-white text-xs font-medium transition-colors"
                    >
                        Save
                    </button>
                </div>
            </div>
        </div>

        <!-- Add Member Modal -->
        <div v-if="showAddMemberModal" class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="fixed inset-0 bg-black/50" @click="showAddMemberModal = false"></div>
            <div class="relative bg-bg-surface-1 rounded-xl shadow-xl w-full max-w-md p-5 space-y-4">
                <div class="flex justify-between items-center">
                    <h3 class="text-sm font-bold text-text-1">Add Team Member</h3>
                    <button @click="showAddMemberModal = false" class="p-1 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <X class="w-4 h-4 text-text-3" />
                    </button>
                </div>
                <div class="relative">
                    <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-4" />
                    <input 
                        v-model="memberSearch"
                        type="text"
                        placeholder="Search users..."
                        class="w-full pl-9 pr-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-2 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none"
                    />
                </div>
                <div class="max-h-60 overflow-y-auto space-y-1">
                    <div v-if="searchingMembers" class="text-center py-4 text-xs text-text-3">Searching...</div>
                    <div v-else-if="memberSearchResults.length === 0" class="text-center py-4 text-xs text-text-3">
                        {{ memberSearch ? 'No users found' : 'Type to search users' }}
                    </div>
                    <div v-else v-for="user in memberSearchResults" :key="user.id" class="flex items-center justify-between p-2 hover:bg-bg-surface-2 rounded-lg transition-colors">
                        <div class="flex items-center gap-2.5">
                            <div class="w-7 h-7 rounded-full bg-bg-surface-2 flex items-center justify-center text-text-2 font-bold text-xs">
                                {{ (user.display_name || user.username).charAt(0).toUpperCase() }}
                            </div>
                            <div>
                                <div class="text-xs font-medium text-text-1">{{ user.display_name || user.username }}</div>
                                <div class="text-[10px] text-text-3">@{{ user.username }}</div>
                            </div>
                        </div>
                        <button @click="addMember(user)" class="p-1.5 bg-brand/10 text-brand rounded-lg hover:bg-brand/20 transition-colors">
                            <Plus class="w-3.5 h-3.5" />
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
