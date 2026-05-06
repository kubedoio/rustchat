<script setup lang="ts">
import { ref, computed, watch } from 'vue';
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

watch(() => props.open, (isOpen) => {
    if (isOpen) {
        teamStore.fetchPublicTeams();
    }
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
            <div class="absolute inset-0 bg-bg-app/70 backdrop-blur-sm" @click="emit('close')"></div>
            
            <!-- Modal -->
            <div class="relative mx-4 max-h-[80vh] w-full max-w-lg overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl">
                <!-- Header -->
                <div class="flex items-center justify-between border-b border-border-1 px-6 py-4">
                    <div class="flex items-center space-x-3">
                        <div class="flex h-10 w-10 items-center justify-center rounded-r-2 bg-brand/10 text-brand">
                            <Users class="h-5 w-5" />
                        </div>
                        <div>
                            <h2 class="text-lg font-semibold text-brand">Browse Teams</h2>
                            <p class="text-xs text-text-3">Join another workspace without leaving your current flow.</p>
                        </div>
                    </div>
                    <button @click="emit('close')" class="rounded-r-2 p-1 transition-standard hover:bg-bg-surface-2">
                        <X class="h-5 w-5 text-text-3" />
                    </button>
                </div>

                <!-- Content -->
                <div class="max-h-[60vh] overflow-y-auto p-6">
                    <div v-if="teamStore.loading" class="py-8 text-center text-text-3">
                        Loading teams...
                    </div>

                    <div v-else-if="availableTeams.length === 0 && myTeams.length === 0" class="py-8 text-center text-text-3">
                        <Users class="mx-auto mb-4 h-12 w-12 text-text-4" />
                        <p>No public teams available</p>
                    </div>

                    <div v-else class="space-y-6">
                        <!-- Available to join -->
                        <div v-if="availableTeams.length > 0">
                            <h3 class="mb-3 text-xs font-semibold uppercase tracking-[0.18em] text-text-3">Available to Join</h3>
                            <div class="space-y-2">
                                <div 
                                    v-for="team in availableTeams" 
                                    :key="team.id"
                                    class="flex items-center justify-between rounded-r-2 border border-border-1 bg-bg-surface-2/70 p-4 transition-standard hover:bg-bg-surface-2"
                                >
                                    <div class="flex-1 min-w-0">
                                        <p class="truncate font-medium text-text-1">
                                            {{ team.display_name || team.name }}
                                        </p>
                                        <p v-if="team.description" class="truncate text-sm text-text-3">
                                            {{ team.description }}
                                        </p>
                                    </div>
                                    <button
                                        @click="joinTeam(team.id)"
                                        :disabled="joining === team.id"
                                        class="ml-4 flex items-center rounded-r-2 bg-brand px-3 py-1.5 text-sm font-medium text-brand-foreground transition-standard hover:opacity-90 disabled:opacity-50"
                                    >
                                        <span>{{ joining === team.id ? 'Joining...' : 'Join' }}</span>
                                        <ArrowRight class="w-4 h-4 ml-1" />
                                    </button>
                                </div>
                            </div>
                        </div>

                        <!-- Already a member -->
                        <div v-if="myTeams.length > 0">
                            <h3 class="mb-3 text-xs font-semibold uppercase tracking-[0.18em] text-text-3">Already Joined</h3>
                            <div class="space-y-2">
                                <div 
                                    v-for="team in myTeams" 
                                    :key="team.id"
                                    class="flex items-center justify-between rounded-r-2 border border-border-1 bg-bg-surface-2/45 p-4"
                                >
                                    <div class="flex-1 min-w-0">
                                        <p class="truncate font-medium text-text-1">
                                            {{ team.display_name || team.name }}
                                        </p>
                                    </div>
                                    <div class="flex items-center text-sm text-success">
                                        <Check class="mr-1 h-4 w-4" />
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
