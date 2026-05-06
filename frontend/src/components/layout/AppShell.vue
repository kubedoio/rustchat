<script setup lang="ts">
import { watch, computed } from 'vue';
import GlobalHeader from './GlobalHeader.vue';
import TeamRail from './TeamRail.vue';
import ChannelSidebar from './ChannelSidebar.vue';
import RightSidebar from './RightSidebar.vue';
import { useUIStore } from '../../stores/ui';
import { useChannelStore } from '../../stores/channels';
import { useBreakpoints } from '../../composables/useBreakpoints';

const emit = defineEmits<{
  (e: 'rhsJump', messageId: string): void;
  (e: 'openChannelSettings'): void;
}>();

const ui = useUIStore();
const channelStore = useChannelStore();
const { isMobile, isTablet, isMobileOrTablet } = useBreakpoints();

// Computed for mobile sidebar visibility
const showMobileSidebar = computed(() => ui.isLhsOpen && isMobile.value);

// Close mobile sidebar when switching to desktop
watch(isMobile, (mobile) => {
  if (!mobile && ui.isLhsOpen) {
    ui.closeLhs();
  }
});

// Close mobile sidebar when channel changes
watch(() => channelStore.currentChannelId, () => {
  if (isMobile.value && ui.isLhsOpen) {
    ui.closeLhs();
  }
});
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden bg-bg-app text-text-1 transition-standard">
    <!-- Top Header -->
    <GlobalHeader class="shrink-0" />

    <!-- Mobile Sidebar Overlay -->
    <Transition
      enter-active-class="transition-opacity duration-200"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-opacity duration-150"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div 
        v-if="showMobileSidebar" 
        class="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm"
        @click="ui.closeLhs()"
      />
    </Transition>

    <!-- Mobile Sidebar Drawer -->
    <Transition
      enter-active-class="transition-transform duration-250 ease-out"
      enter-from-class="-translate-x-full"
      enter-to-class="translate-x-0"
      leave-active-class="transition-transform duration-200 ease-in"
      leave-from-class="translate-x-0"
      leave-to-class="-translate-x-full"
    >
      <div 
        v-if="showMobileSidebar" 
        class="fixed top-0 left-0 bottom-0 z-50 flex shadow-2xl"
      >
        <TeamRail class="h-full border-r border-border-1 bg-bg-surface-2" />
        <ChannelSidebar class="h-full border-r border-border-1 bg-bg-surface-2 w-[var(--sidebar-width)]" />
      </div>
    </Transition>

    <!-- Main Layout -->
    <div class="relative flex flex-1 overflow-hidden bg-[radial-gradient(circle_at_top,_color-mix(in_srgb,_var(--brand)_5%,transparent),transparent_38%)]">
      <!-- Desktop Team Rail (Leftmost) -->
      <TeamRail 
        v-if="!isMobile" 
        class="border-r border-border-1 bg-bg-surface-2 shrink-0" 
      />

      <!-- Desktop Channel Sidebar (LHS) -->
      <ChannelSidebar 
        v-if="!isMobile" 
        class="border-r border-border-1 bg-bg-surface-2 shrink-0" 
      />

      <!-- Main Content (Center) -->
      <main 
        class="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-[linear-gradient(180deg,color-mix(in_srgb,var(--bg-surface-1)_94%,transparent),var(--bg-surface-1))]"
        :class="{ 'shadow-2': ui.isRhsOpen && isMobileOrTablet }"
      >
        <slot />
        
        <!-- Mobile RHS Overlay -->
        <Transition
          enter-active-class="transition-opacity duration-200"
          enter-from-class="opacity-0"
          enter-to-class="opacity-100"
          leave-active-class="transition-opacity duration-150"
          leave-from-class="opacity-100"
          leave-to-class="opacity-0"
        >
          <div 
            v-if="ui.isRhsOpen && isMobileOrTablet" 
            class="absolute inset-0 bg-black/40 backdrop-blur-sm z-30 lg:hidden"
            @click="ui.closeRhs()"
          />
        </Transition>
      </main>

      <!-- Right Sidebar (RHS) -->
      <Transition
        enter-active-class="transition-transform duration-250 ease-out"
        enter-from-class="translate-x-full"
        enter-to-class="translate-x-0"
        leave-active-class="transition-transform duration-200 ease-in"
        leave-from-class="translate-x-0"
        leave-to-class="translate-x-full"
      >
        <RightSidebar 
          v-if="ui.isRhsOpen" 
          class="fixed lg:relative top-0 right-0 h-full z-40 lg:z-10 shadow-2xl lg:shadow-none bg-bg-surface-1 border-l border-border-1 shrink-0"
          :class="[isMobile ? 'w-[85%]' : isTablet ? 'w-[360px]' : 'w-[var(--rhs-width)]']"
          @jump="emit('rhsJump', $event)"
          @openSettings="emit('openChannelSettings')"
        />
      </Transition>
    </div>
  </div>
</template>
