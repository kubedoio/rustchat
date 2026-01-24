<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { X, Users, ArrowRight, Check } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import { useToast } from '../../composables/useToast';

const props = defineProps<{
    open: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const teamStore = useTeamStore();
const toast = useToast();
const joining = ref<string | null>(null);

onMounted(() => {
    teamStore.fetchPublicTeams();
});

// Teams the user hasn't joined yet
const availableTeams = computed(() => {
    const myTeamIds = new Set(teamStore.teams.map(t => t.id));
    return teamStore.publicTeams.filter(t => !myTeamIds.has(t.id));
});

// Teams the user has already joined
const myTeams = computed(() => {
    const myTeamIds = new Set(teamStore.teams.map(t => t.id));
    return teamStore.publicTeams.filter(t => myTeamIds.has(t.id));
});

async function joinTeam(teamId: string) {
    joining.value = teamId;
    try {
        await teamStore.joinTeam(teamId);
        emit('close');
        toast.success('Joined team', 'You have successfully joined the team');
    } catch (e) {
        console.error('Failed to join team:', e);
        toast.error('Failed to join', 'Could not join the team');
    } finally {
        joining.value = null;
    }
}
</script>

<template>
    <Teleport to="body">
        <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center">
            <!-- Backdrop -->
            <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="emit('close')"></div>
            
            <!-- Modal -->
            <div class="relative bg-slate-900 rounded-xl shadow-2xl border border-white/10 w-full max-w-lg max-h-[80vh] overflow-hidden">
                <!-- Header -->
                <div class="flex items-center justify-between px-6 py-4 border-b border-white/10">
                    <div class="flex items-center space-x-3">
                        <Users class="w-5 h-5 text-indigo-400" />
                        <h2 class="text-lg font-semibold text-white">Browse Teams</h2>
                    </div>
                    <button @click="emit('close')" class="p-1 hover:bg-white/10 rounded-lg transition-colors">
                        <X class="w-5 h-5 text-gray-400" />
                    </button>
                </div>

                <!-- Content -->
                <div class="p-6 overflow-y-auto max-h-[60vh]">
                    <div v-if="teamStore.loading" class="text-center py-8 text-gray-400">
                        Loading teams...
                    </div>

                    <div v-else-if="availableTeams.length === 0 && myTeams.length === 0" class="text-center py-8 text-gray-400">
                        <Users class="w-12 h-12 mx-auto mb-4 text-gray-600" />
                        <p>No public teams available</p>
                    </div>

                    <div v-else class="space-y-6">
                        <!-- Available to join -->
                        <div v-if="availableTeams.length > 0">
                            <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">Available to Join</h3>
                            <div class="space-y-2">
                                <div 
                                    v-for="team in availableTeams" 
                                    :key="team.id"
                                    class="flex items-center justify-between p-4 bg-slate-800 hover:bg-slate-700 rounded-lg transition-colors"
                                >
                                    <div class="flex-1 min-w-0">
                                        <p class="font-medium text-white truncate">
                                            {{ team.display_name || team.name }}
                                        </p>
                                        <p v-if="team.description" class="text-sm text-gray-400 truncate">
                                            {{ team.description }}
                                        </p>
                                    </div>
                                    <button
                                        @click="joinTeam(team.id)"
                                        :disabled="joining === team.id"
                                        class="ml-4 flex items-center px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
                                    >
                                        <span>{{ joining === team.id ? 'Joining...' : 'Join' }}</span>
                                        <ArrowRight class="w-4 h-4 ml-1" />
                                    </button>
                                </div>
                            </div>
                        </div>

                        <!-- Already a member -->
                        <div v-if="myTeams.length > 0">
                            <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">Already Joined</h3>
                            <div class="space-y-2">
                                <div 
                                    v-for="team in myTeams" 
                                    :key="team.id"
                                    class="flex items-center justify-between p-4 bg-slate-800/50 rounded-lg"
                                >
                                    <div class="flex-1 min-w-0">
                                        <p class="font-medium text-white truncate">
                                            {{ team.display_name || team.name }}
                                        </p>
                                    </div>
                                    <div class="flex items-center text-green-400 text-sm">
                                        <Check class="w-4 h-4 mr-1" />
                                        Member
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </Teleport>
</template>
