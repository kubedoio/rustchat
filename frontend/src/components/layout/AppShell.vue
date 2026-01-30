<script setup lang="ts">
import { computed } from 'vue';
import GlobalHeader from './GlobalHeader.vue';
import TeamRail from './TeamRail.vue';
import ChannelSidebar from './ChannelSidebar.vue';
import RightSidebar from './RightSidebar.vue';
import { useUIStore } from '../../stores/ui';
import { Menu, X } from 'lucide-vue-next';

const ui = useUIStore();

const showMobileOverlay = computed(() => 
    ui.isMobile && (ui.isSidebarOpen || ui.isTeamRailOpen)
);

function closeMobileDrawers() {
    ui.closeSidebar();
    ui.closeTeamRail();
}
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden bg-white dark:bg-gray-900 text-gray-800 dark:text-gray-100">
    <!-- Top Header -->
    <GlobalHeader />

    <div class="flex flex-1 overflow-hidden relative">
        <!-- Mobile Overlay -->
        <div 
            v-if="showMobileOverlay"
            class="fixed inset-0 bg-black/50 z-40 lg:hidden"
            @click="closeMobileDrawers"
        />

        <!-- Team Rail (Leftmost) -->
        <!-- Desktop: always visible, Mobile: hidden by default, shown as drawer when open -->
        <div 
            class="transition-transform duration-300 ease-in-out z-30"
            :class="[
                // Desktop: always show with fixed width
                'hidden lg:flex lg:flex-col lg:w-16 lg:shrink-0',
                // Mobile: absolute positioning when open
                ui.isMobile ? 'fixed inset-y-0 left-0 w-16' : '',
                // Mobile: translate based on open state
                ui.isMobile && !ui.isTeamRailOpen ? '-translate-x-full' : 'translate-x-0'
            ]"
        >
            <TeamRail />
        </div>

        <!-- Channel Sidebar (LHS) -->
        <!-- Desktop: always visible with fixed width, Mobile: full-width drawer -->
        <div 
            class="transition-transform duration-300 ease-in-out z-30"
            :class="[
                // Desktop: always show with fixed width
                'hidden md:flex md:flex-col md:w-64 md:shrink-0',
                // Mobile: fixed positioning when open, full width
                ui.isMobile ? 'fixed inset-y-0 left-0 w-[280px]' : '',
                // Mobile: if team rail is also open on mobile, offset by team rail width
                ui.isMobile && ui.isTeamRailOpen ? 'left-16' : 'left-0',
                // Mobile: translate based on open state
                ui.isMobile && !ui.isSidebarOpen ? '-translate-x-full' : 'translate-x-0'
            ]"
        >
            <ChannelSidebar />
        </div>

        <!-- Main Content (Center) -->
        <main class="flex-1 flex flex-col min-w-0 bg-white dark:bg-gray-900 relative w-full">
            <!-- Mobile Sidebar Toggle Button (shown when sidebar is closed) -->
            <button
                v-if="ui.isMobile && !ui.isSidebarOpen"
                @click="ui.toggleSidebar"
                class="fixed top-20 left-4 z-20 p-2 bg-white dark:bg-slate-800 rounded-full shadow-lg border border-gray-200 dark:border-slate-700 lg:hidden"
                aria-label="Open channels"
            >
                <Menu class="w-5 h-5 text-gray-600 dark:text-gray-300" />
            </button>

            <slot />
        </main>

        <!-- Right Sidebar (RHS) -->
        <!-- Desktop: slide over, Mobile: full width -->
        <div
            v-if="ui.isRhsOpen"
            class="absolute inset-y-0 right-0 z-40 w-full md:w-80 lg:w-96 bg-white dark:bg-slate-900 shadow-xl border-l border-gray-200 dark:border-slate-700"
        >
            <RightSidebar />
        </div>
    </div>
  </div>
</template>
