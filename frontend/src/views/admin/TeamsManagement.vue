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
    ArrowLeft
} from 'lucide-vue-next';
import adminApi, { type AdminTeam, type AdminChannel } from '../../api/admin';

// State
const teams = ref<AdminTeam[]>([]);
const totalTeams = ref(0);
const loading = ref(true);
const search = ref('');
const page = ref(1);
const perPage = 10;

const selectedTeam = ref<AdminTeam | null>(null);
const teamChannels = ref<AdminChannel[]>([]);
const channelsLoading = ref(false);

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
    channelsLoading.value = true;
    try {
        const { data } = await adminApi.listChannels({
            team_id: team.id,
            per_page: 100 // Load all channels for the team
        });
        teamChannels.value = data.channels;
    } catch (e) {
        console.error('Failed to fetch channels', e);
    } finally {
        channelsLoading.value = false;
    }
}

function closeDetails() {
    selectedTeam.value = null;
    teamChannels.value = [];
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

async function deleteChannel(channel: AdminChannel) {
    if (!confirm(`Are you sure you want to delete the channel "#${channel.name}"?`)) {
        return;
    }

    try {
        await adminApi.deleteChannel(channel.id);
        if (selectedTeam.value) {
            await viewTeamDetails(selectedTeam.value);
        }
    } catch (e) {
        console.error('Failed to delete channel', e);
        alert('Failed to delete channel');
    }
}

// Watchers
watch([search, page], () => {
    fetchTeams();
});

onMounted(fetchTeams);
</script>

<template>
    <div class="space-y-6">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div>
                <div v-if="selectedTeam" class="flex items-center space-x-2 mb-2">
                    <button @click="closeDetails" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                        <ArrowLeft class="w-4 h-4" />
                    </button>
                    <span class="text-gray-400">Teams</span>
                </div>
                <h1 class="text-2xl font-bold text-gray-900 dark:text-white">
                    {{ selectedTeam ? (selectedTeam.display_name || selectedTeam.name) : 'Teams & Channels' }}
                </h1>
                <p class="text-gray-500 dark:text-gray-400 mt-1">
                    {{ selectedTeam ? 'Manage channels and visibility for this team' : 'Manage teams, channels, and memberships' }}
                </p>
            </div>
        </div>

        <!-- Team List View -->
        <div v-if="!selectedTeam" class="space-y-4">
            <!-- Search & Filters -->
            <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 bg-white dark:bg-slate-800 p-4 rounded-xl border border-gray-200 dark:border-slate-700">
                <div class="relative flex-1 max-w-md">
                    <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
                    <input 
                        v-model="search"
                        type="text" 
                        placeholder="Search teams..." 
                        class="w-full pl-10 pr-4 py-2 bg-gray-50 dark:bg-slate-900 border border-gray-200 dark:border-slate-700 rounded-lg focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none text-sm transition-all dark:text-white"
                    />
                </div>
            </div>

            <!-- Teams Table -->
            <div class="bg-white dark:bg-slate-800 rounded-xl border border-gray-200 dark:border-slate-700 overflow-hidden shadow-sm">
                <div class="overflow-x-auto">
                    <table class="w-full text-left">
                        <thead>
                            <tr class="bg-gray-50 dark:bg-slate-900/50 border-b border-gray-200 dark:border-slate-700">
                                <th class="px-6 py-4 text-xs font-semibold text-gray-500 uppercase tracking-wider">Team</th>
                                <th class="px-6 py-4 text-xs font-semibold text-gray-500 uppercase tracking-wider">Visibility</th>
                                <th class="px-6 py-4 text-xs font-semibold text-gray-500 uppercase tracking-wider text-center">Members</th>
                                <th class="px-6 py-4 text-xs font-semibold text-gray-500 uppercase tracking-wider text-center">Channels</th>
                                <th class="px-6 py-4 text-xs font-semibold text-gray-500 uppercase tracking-wider">Created</th>
                                <th class="px-6 py-4 text-xs font-semibold text-gray-500 uppercase tracking-wider text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200 dark:divide-slate-700">
                            <tr v-if="loading" v-for="i in 3" :key="i" class="animate-pulse">
                                <td v-for="j in 6" :key="j" class="px-6 py-4">
                                    <div class="h-4 bg-gray-100 dark:bg-slate-700 rounded w-2/3"></div>
                                </td>
                            </tr>
                            <tr v-else-if="teams.length === 0" class="text-center py-12">
                                <td colspan="6" class="px-6 py-12">
                                    <div class="flex flex-col items-center justify-center">
                                        <Building2 class="w-12 h-12 text-gray-300 dark:text-gray-600 mb-4" />
                                        <p class="text-gray-500 dark:text-gray-400">No teams found</p>
                                    </div>
                                </td>
                            </tr>
                            <tr v-for="team in teams" :key="team.id" class="hover:bg-gray-50 dark:hover:bg-slate-700/50 transition-colors">
                                <td class="px-6 py-4">
                                    <div class="flex items-center space-x-3">
                                        <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary font-bold">
                                            {{ (team.display_name || team.name).charAt(0).toUpperCase() }}
                                        </div>
                                        <div>
                                            <div class="font-medium text-gray-900 dark:text-white">{{ team.display_name || team.name }}</div>
                                            <div class="text-xs text-gray-500 dark:text-gray-400">@{{ team.name }}</div>
                                        </div>
                                    </div>
                                </td>
                                <td class="px-6 py-4">
                                    <span v-if="team.is_public" class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400 border border-green-200 dark:border-green-800">
                                        Public
                                    </span>
                                    <span v-else class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400 border border-purple-200 dark:border-purple-800">
                                        Private
                                    </span>
                                </td>
                                <td class="px-6 py-4 text-center">
                                    <div class="flex items-center justify-center space-x-1 text-gray-600 dark:text-gray-300">
                                        <Users class="w-4 h-4" />
                                        <span>{{ team.members_count }}</span>
                                    </div>
                                </td>
                                <td class="px-6 py-4 text-center">
                                    <div class="flex items-center justify-center space-x-1 text-gray-600 dark:text-gray-300">
                                        <MessageSquare class="w-4 h-4" />
                                        <span>{{ team.channels_count }}</span>
                                    </div>
                                </td>
                                <td class="px-6 py-4 text-sm text-gray-500 dark:text-gray-400">
                                    {{ new Date(team.created_at).toLocaleDateString() }}
                                </td>
                                <td class="px-6 py-4 text-right">
                                    <div class="flex items-center justify-end space-x-2">
                                        <button 
                                            @click="viewTeamDetails(team)"
                                            class="p-2 text-gray-400 hover:text-primary transition-colors"
                                            title="View Details"
                                        >
                                            <Eye class="w-4 h-4" />
                                        </button>
                                        <button 
                                            @click="deleteTeam(team)"
                                            class="p-2 text-gray-400 hover:text-red-500 transition-colors"
                                            title="Delete Team"
                                        >
                                            <Trash2 class="w-4 h-4" />
                                        </button>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <!-- Pagination -->
                <div class="px-6 py-4 flex items-center justify-between border-t border-gray-200 dark:border-slate-700 bg-gray-50/50 dark:bg-slate-900/30">
                    <div class="text-sm text-gray-500 dark:text-gray-400">
                        Showing {{ ((page - 1) * perPage) + 1 }} to {{ Math.min(page * perPage, totalTeams) }} of {{ totalTeams }} teams
                    </div>
                    <div class="flex items-center space-x-2">
                        <button 
                            @click="page--"
                            :disabled="page === 1"
                            class="p-2 border border-gray-200 dark:border-slate-700 rounded-lg hover:bg-white dark:hover:bg-slate-800 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <ChevronLeft class="w-4 h-4 dark:text-white" />
                        </button>
                        <button 
                            @click="page++"
                            :disabled="page * perPage >= totalTeams"
                            class="p-2 border border-gray-200 dark:border-slate-700 rounded-lg hover:bg-white dark:hover:bg-slate-800 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <ChevronRight class="w-4 h-4 dark:text-white" />
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Team Details View (Channels) -->
        <div v-else class="space-y-6">
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <!-- Info Sidebar -->
                <div class="space-y-6">
                    <div class="bg-white dark:bg-slate-800 rounded-xl border border-gray-200 dark:border-slate-700 p-6 shadow-sm">
                        <h3 class="text-sm font-semibold text-gray-900 dark:text-white uppercase tracking-wider mb-4">Team Info</h3>
                        <div class="space-y-4">
                            <div>
                                <label class="text-xs text-gray-500 dark:text-gray-400 block mb-1">Internal Name</label>
                                <div class="text-sm font-medium dark:text-white">@{{ selectedTeam.name }}</div>
                            </div>
                            <div>
                                <label class="text-xs text-gray-500 dark:text-gray-400 block mb-1">Description</label>
                                <div class="text-sm dark:text-gray-300 italic">
                                    {{ selectedTeam.description || 'No description provided' }}
                                </div>
                            </div>
                            <div>
                                <label class="text-xs text-gray-500 dark:text-gray-400 block mb-1">Stats</label>
                                <div class="flex items-center space-x-4">
                                    <div class="flex items-center space-x-1 text-sm dark:text-gray-300">
                                        <Users class="w-4 h-4 text-gray-400" />
                                        <span>{{ selectedTeam.members_count }} members</span>
                                    </div>
                                    <div class="flex items-center space-x-1 text-sm dark:text-gray-300">
                                        <MessageSquare class="w-4 h-4 text-gray-400" />
                                        <span>{{ selectedTeam.channels_count }} channels</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Channels List -->
                <div class="lg:col-span-2">
                    <div class="bg-white dark:bg-slate-800 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm overflow-hidden">
                        <div class="px-6 py-4 border-b border-gray-200 dark:border-slate-700 flex items-center justify-between">
                            <h3 class="font-semibold text-gray-900 dark:text-white">Team Channels</h3>
                        </div>
                        
                        <div v-if="channelsLoading" class="p-12 text-center">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
                        </div>
                        
                        <div v-else-if="teamChannels.length === 0" class="p-12 text-center text-gray-500">
                            No channels in this team.
                        </div>

                        <div v-else class="divide-y divide-gray-200 dark:divide-slate-700">
                            <div v-for="channel in teamChannels" :key="channel.id" class="px-6 py-4 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-slate-700/50">
                                <div class="flex items-center space-x-3">
                                    <div v-if="channel.channel_type === 'public'" class="text-gray-400">
                                        <Hash class="w-5 h-5" />
                                    </div>
                                    <div v-else class="text-purple-400">
                                        <Lock class="w-5 h-5" />
                                    </div>
                                    <div>
                                        <div class="font-medium text-gray-900 dark:text-white">
                                            {{ channel.display_name || channel.name }}
                                            <span v-if="channel.is_archived" class="ml-2 px-1.5 py-0.5 rounded text-[10px] font-bold uppercase bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400">Archived</span>
                                        </div>
                                        <p class="text-xs text-gray-500 dark:text-gray-400">{{ channel.purpose || 'No purpose set' }}</p>
                                    </div>
                                </div>
                                <div class="flex items-center space-x-4">
                                    <div class="text-xs text-gray-500 dark:text-gray-400 flex items-center">
                                        <Users class="w-3 h-3 mr-1" />
                                        {{ channel.members_count }}
                                    </div>
                                    <button 
                                        @click="deleteChannel(channel)"
                                        class="p-1.5 text-gray-400 hover:text-red-500 transition-colors"
                                    >
                                        <Trash2 class="w-4 h-4" />
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
