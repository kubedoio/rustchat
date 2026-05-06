<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { Plus } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import { useUnreadStore } from '../../stores/unreads';
import { useAuthStore } from '../../stores/auth';
import { canCreateTeam as canCreateTeamForRole } from '../../features/permissions/capabilities';
import CreateTeamModal from '../modals/CreateTeamModal.vue';

const teamStore = useTeamStore();
const unreadStore = useUnreadStore();
const authStore = useAuthStore();
const showCreateModal = ref(false);
const canCreateTeam = computed(() => canCreateTeamForRole(authStore.user?.role));

onMounted(() => {
  teamStore.fetchTeams();
});

function selectTeam(teamId: string) {
  teamStore.selectTeam(teamId);
}

function getInitials(name: string): string {
  return name.split(' ').map(w => w[0]).join('').slice(0, 2).toUpperCase();
}
</script>

<template>
  <aside class="z-20 flex w-[var(--team-rail-width)] shrink-0 flex-col items-center border-r border-border-1 bg-bg-surface-2/95 py-3">
    <div class="mb-3 text-[10px] font-semibold uppercase tracking-[0.22em] text-text-3">
      Teams
    </div>

    <!-- Teams List -->
    <div class="flex w-full flex-1 flex-col items-center gap-2 px-2">
      <div 
        v-for="team in teamStore.teams" 
        :key="team.id"
        class="group relative flex w-full justify-center"
      >
        <!-- Active Indicator -->
        <div 
          v-if="teamStore.currentTeamId === team.id"
          class="absolute left-0 top-1/2 h-9 w-[4px] -translate-y-1/2 rounded-r-full bg-brand transition-standard"
        ></div>

        <!-- Team Button -->
        <button
          @click="selectTeam(team.id)"
          class="relative flex h-11 w-11 items-center justify-center overflow-hidden rounded-r-2 text-sm font-bold transition-standard"
          :class="{ 
            'bg-brand text-brand-foreground shadow-1': teamStore.currentTeamId === team.id,
            'border border-border-1 bg-bg-surface-1 text-text-1 hover:-translate-y-0.5 hover:border-border-2 hover:text-brand hover:shadow-1': teamStore.currentTeamId !== team.id
          }"
          :title="team.display_name || team.name"
        >
          {{ getInitials(team.display_name || team.name) }}
        </button>

        <!-- Unread Indicator -->
        <div 
          v-if="unreadStore.getTeamUnreadCount(team.id) > 0 && teamStore.currentTeamId !== team.id"
          class="absolute right-0.5 top-0 h-3 w-3 rounded-full border-2 border-bg-surface-2 bg-danger shadow-1"
        ></div>
      </div>

      <!-- Empty State -->
      <div v-if="teamStore.teams.length === 0 && !teamStore.loading" class="text-text-3 text-[10px] text-center px-2 uppercase font-bold tracking-tight mt-4">
        No teams
      </div>
    </div>

    <!-- Add Team Button -->
    <div v-if="canCreateTeam" class="mt-auto w-full border-t border-border-1 px-2 pt-3">
      <button 
        @click="showCreateModal = true"
        class="mx-auto flex h-11 w-11 items-center justify-center rounded-r-2 border border-dashed border-border-2 bg-bg-surface-1 text-text-3 transition-standard hover:border-brand hover:bg-brand/10 hover:text-brand"
        title="Create Team"
      >
        <Plus class="w-5 h-5" />
      </button>
    </div>

    <!-- Create Team Modal -->
    <CreateTeamModal v-if="canCreateTeam" :show="showCreateModal" @close="showCreateModal = false" />
  </aside>
</template>
