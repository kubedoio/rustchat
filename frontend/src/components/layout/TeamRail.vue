<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { Plus } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import CreateTeamModal from '../modals/CreateTeamModal.vue';

const teamStore = useTeamStore();
const showCreateModal = ref(false);

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
  <aside class="w-[64px] bg-gray-900 flex flex-col items-center py-3 space-y-3 z-20 shrink-0">
    <div 
      v-for="team in teamStore.teams" 
      :key="team.id"
      class="group relative"
    >
      <!-- Active Indicator -->
      <div 
        v-if="teamStore.currentTeamId === team.id"
        class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-white rounded-r full transition-all"
      ></div>

      <!-- Team Icon -->
      <button
        @click="selectTeam(team.id)"
        class="w-10 h-10 rounded-xl bg-gray-700 hover:bg-primary transition-all cursor-pointer flex items-center justify-center text-white font-bold text-sm overflow-hidden border-2 "
        :class="teamStore.currentTeamId === team.id ? 'border-white bg-primary' : 'border-transparent group-hover:border-gray-500'"
        :title="team.display_name || team.name"
      >
        {{ getInitials(team.display_name || team.name) }}
      </button>
    </div>

    <!-- Empty state -->
    <div v-if="teamStore.teams.length === 0 && !teamStore.loading" class="text-gray-500 text-xs text-center px-2">
        No teams yet
    </div>

    <!-- Add Team Button -->
    <button 
      @click="showCreateModal = true"
      class="w-10 h-10 rounded-full bg-gray-800 hover:bg-green-600 transition-colors cursor-pointer flex items-center justify-center text-gray-400 hover:text-white group"
      title="Create Team"
    >
      <Plus class="w-5 h-5 group-hover:scale-110 transition-transform" />
    </button>

    <!-- Create Team Modal -->
    <CreateTeamModal :show="showCreateModal" @close="showCreateModal = false" />
  </aside>
</template>
