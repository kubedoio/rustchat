<script setup lang="ts">
import GlobalHeader from './GlobalHeader.vue';
import TeamRail from './TeamRail.vue';
import ChannelSidebar from './ChannelSidebar.vue';
import RightSidebar from './RightSidebar.vue';
import { useUIStore } from '../../stores/ui';

const ui = useUIStore();
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden bg-white dark:bg-gray-900 text-gray-800 dark:text-gray-100">
    <!-- Top Header -->
    <GlobalHeader />

    <div class="flex flex-1 overflow-hidden">
        <!-- Team Rail (Leftmost) -->
        <TeamRail />

        <!-- Channel Sidebar (LHS) -->
        <ChannelSidebar />

        <!-- Main Content (Center) -->
        <main class="flex-1 flex flex-col min-w-0 bg-white dark:bg-gray-900 relative">
            <slot />
        </main>

        <!-- Right Sidebar (RHS) -->
        <RightSidebar v-if="ui.isRhsOpen" />
    </div>
  </div>
</template>
