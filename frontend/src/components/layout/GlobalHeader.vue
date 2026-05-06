<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { useRouter } from 'vue-router';
import { Bell, Search, HelpCircle, LogOut, Smile, Shield, User, Check, Menu, ChevronDown, ChevronUp, ClipboardList } from 'lucide-vue-next';
import { useAuthStore } from '../../stores/auth';
import { useUIStore } from '../../stores/ui';
import SearchModal from '../modals/SearchModal.vue';
import QuickSwitcherModal from '../navigation/QuickSwitcherModal.vue';
import { useQuickSwitcher } from '../../composables/useQuickSwitcher';
import type { QuickSwitcherItem } from '../../composables/useQuickSwitcher';
import SetStatusModal from '../modals/SetStatusModal.vue';
import RcAvatar from '../ui/RcAvatar.vue';
import NotificationsDropdown from './NotificationsDropdown.vue';
import ActivityFeed from '../activity/ActivityFeed.vue';
import { useConfigStore } from '../../stores/config';
import { useTeamStore } from '../../stores/teams';
import { usePresenceStore } from '../../features/presence';
import { useUnreadStore } from '../../stores/unreads';
import { useBreakpoints } from '../../composables/useBreakpoints';
import { activityService } from '../../features/activity/services/activityService';
import { useActivityStore } from '../../features/activity/stores/activityStore';
import { useWebSocket } from '../../composables/useWebSocket'

const auth = useAuthStore();
const ui = useUIStore();
const configStore = useConfigStore();
const teamStore = useTeamStore();
const presenceStore = usePresenceStore();
const unreadStore = useUnreadStore();
const activityStore = useActivityStore();
const activityUnreadCount = computed(() => activityStore.unreadCount);
const router = useRouter();
const { isMobile } = useBreakpoints();

const { connectionStatus } = useWebSocket()

const connectionDotClass = computed(() => {
  switch (connectionStatus.value) {
    case 'connected': return 'bg-green-500'
    case 'reconnecting': return 'bg-amber-500 animate-pulse'
    case 'disconnected': return 'bg-orange-500 animate-pulse'
    case 'failed': return 'bg-red-500'
    default: return 'bg-gray-400'
  }
})

const showSearch = ref(false);
const quickSwitcher = useQuickSwitcher();
const showUserMenu = ref(false);
const showSetStatus = ref(false);
const showNotifications = ref(false);
const showDndSubmenu = ref(false);

// Initialize self presence
if (auth.user) {
  presenceStore.setSelfPresence({
    userId: auth.user.id,
    username: auth.user.username,
    presence: (auth.user.presence as any) || 'online'
  });
}

// Keyboard shortcut for search
function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
    e.preventDefault();
    quickSwitcher.toggle();
  }
  if (e.key === 'Escape') {
    if (quickSwitcher.isOpen.value) {
      e.stopPropagation();
      quickSwitcher.close();
      return;
    }
    showSearch.value = false;
    showUserMenu.value = false;
    showDndSubmenu.value = false;
    showNotifications.value = false;
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown);
});

async function setPresence(status: 'online' | 'away' | 'dnd' | 'offline') {
  await auth.updateStatus({ status });
  presenceStore.updatePresenceFromEvent(auth.user?.id || '', status);
  showUserMenu.value = false;
  showDndSubmenu.value = false;
}

function calculateDndEndTime(duration: string): number | undefined {
  const now = new Date();
  switch (duration) {
    case 'thirty_minutes':
      return now.getTime() + 30 * 60 * 1000;
    case 'one_hour':
      return now.getTime() + 60 * 60 * 1000;
    case 'four_hours':
      return now.getTime() + 4 * 60 * 60 * 1000;
    case 'tomorrow': {
      const midnight = new Date(now);
      midnight.setHours(24, 0, 0, 0);
      return midnight.getTime();
    }
    case 'this_week': {
      const endOfWeek = new Date(now);
      const daysUntilSunday = (7 - endOfWeek.getDay()) % 7;
      endOfWeek.setDate(endOfWeek.getDate() + daysUntilSunday + 1);
      endOfWeek.setHours(0, 0, 0, 0);
      return endOfWeek.getTime();
    }
    default:
      return undefined;
  }
}

async function setDndWithDuration(duration: string) {
  const dndEndTime = calculateDndEndTime(duration);
  await auth.updateStatus({ 
    status: 'dnd',
    dnd_end_time: dndEndTime
  });
  presenceStore.updatePresenceFromEvent(auth.user?.id || '', 'dnd');
  showUserMenu.value = false;
  showDndSubmenu.value = false;
}

function openCustomStatus() {
  showSetStatus.value = true;
  showUserMenu.value = false;
}

function openProfile() {
  ui.openSettings('profile');
  showUserMenu.value = false;
}

const userPresence = computed(() => {
  return presenceStore.self?.presence || 'online';
});

const dndDurations = [
  { label: '30 minutes', value: 'thirty_minutes' },
  { label: '1 hour', value: 'one_hour' },
  { label: '4 hours', value: 'four_hours' },
  { label: 'Tomorrow', value: 'tomorrow' },
  { label: 'This week', value: 'this_week' },
];

const siteInitial = computed(() => {
  return configStore.siteConfig.site_name?.charAt(0).toUpperCase() || 'R';
});

const currentTeamLabel = computed(() => {
  return teamStore.currentTeam?.display_name || teamStore.currentTeam?.name || '';
});

function openActivityFeed() {
  activityService.openFeed();
}

function handleQuickSwitcherSelect(item: QuickSwitcherItem) {
  quickSwitcher.addRecentItem(item.id);
  router.push(item.to);
  quickSwitcher.close();
}
</script>

<template>
  <header 
    class="relative z-30 flex h-[var(--header-height)] shrink-0 items-center justify-between border-b border-border-1 bg-bg-surface-1/95 px-3 backdrop-blur-sm sm:px-4"
  >
    <!-- Left: Mobile Menu + Logo -->
    <div class="flex min-w-0 items-center gap-3">
      <!-- Mobile Menu Button -->
      <button
        v-if="isMobile"
        @click="ui.toggleLhs()"
        class="flex items-center justify-center w-11 h-11 rounded-r-2 hover:bg-bg-surface-2 text-text-2 transition-standard focus-ring"
        :class="{ 'bg-bg-surface-2 text-brand': ui.isLhsOpen }"
        aria-label="Toggle navigation menu"
        title="Menu"
      >
        <Menu class="w-5 h-5" />
      </button>

      <!-- Logo -->
      <div class="flex min-w-0 items-center gap-3">
        <img 
          v-if="configStore.siteConfig.logo_url" 
          :src="configStore.siteConfig.logo_url" 
          class="h-9 w-9 shrink-0 rounded-r-1 object-cover shadow-1"
          alt="Logo" 
        />
        <div 
          v-else 
          class="flex h-9 w-9 shrink-0 items-center justify-center rounded-r-1 bg-brand text-sm font-semibold text-brand-foreground shadow-1"
        >
          {{ siteInitial }}
        </div>
        <div class="hidden min-w-0 sm:block">
          <div class="truncate text-base font-semibold tracking-[-0.03em] text-brand">
            {{ configStore.siteConfig.site_name }}
          </div>
          <div class="mt-0.5 truncate text-[11px] font-medium tracking-[0.12em] text-text-3">
            {{ currentTeamLabel || 'Team workspace' }}
          </div>
        </div>
      </div>
    </div>

    <!-- Center: Search (hidden on mobile) -->
    <div class="hidden flex-1 px-4 md:block">
      <button
        @click="showSearch = true"
        class="group mx-auto flex w-full max-w-md items-center gap-3 rounded-r-3 border border-border-1 bg-bg-surface-2/75 px-3.5 py-2.5 text-left transition-standard hover:border-border-2 hover:bg-bg-surface-1 focus-ring"
      >
        <Search class="h-4 w-4 shrink-0 text-text-3 group-hover:text-text-2" />
        <span class="min-w-0 flex-1 truncate text-sm font-medium text-text-2 group-hover:text-text-1">
          Search
        </span>
        <kbd class="hidden items-center gap-0.5 text-[10px] font-medium text-text-4 lg:flex">
          <span class="px-1.5 py-0.5 bg-bg-app border border-border-1 rounded">⌘</span>
          <span class="px-1.5 py-0.5 bg-bg-app border border-border-1 rounded">K</span>
        </kbd>
      </button>
    </div>

    <!-- Right: Actions -->
    <div class="flex items-center gap-1 rounded-r-3 border border-border-1 bg-bg-surface-2/75 p-1 sm:gap-1.5">
      <!-- Mobile Search Button -->
      <button 
        @click="showSearch = true"
        class="md:hidden flex items-center justify-center w-11 h-11 rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
        aria-label="Search"
      >
        <Search class="w-5 h-5" />
      </button>

      <!-- Help Button -->
      <button 
        class="hidden sm:flex items-center justify-center w-11 h-11 rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
        aria-label="Help"
      >
        <HelpCircle class="w-5 h-5" />
      </button>
      
      <!-- Notifications -->
      <div class="relative">
        <button 
          @click="showNotifications = !showNotifications"
          class="relative flex items-center justify-center w-11 h-11 rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
          :class="{ 'bg-bg-surface-2 text-text-1': showNotifications }"
          aria-label="Notifications"
        >
          <Bell class="w-5 h-5" />
          <span 
            v-if="unreadStore.totalUnreadCount > 0"
            class="absolute top-1.5 right-1.5 w-2 h-2 rounded-full bg-danger ring-2 ring-bg-surface-1"
          />
        </button>
        
        <NotificationsDropdown 
          v-if="showNotifications" 
          @close="showNotifications = false" 
        />
        
        <!-- Click outside backdrop -->
        <div
          v-if="showNotifications"
          class="fixed inset-0 z-40"
          @click="showNotifications = false"
        />
      </div>

      <!-- Activity Feed button -->
      <div class="relative">
        <button
          class="relative flex h-11 w-11 items-center justify-center rounded-r-2 text-text-3 transition-standard hover:bg-bg-surface-1 hover:text-text-1 focus-ring"
          title="Activity Feed"
          @click="openActivityFeed"
        >
          <ClipboardList class="w-5 h-5" />
          <span
            v-if="activityUnreadCount > 0"
            class="absolute -top-0.5 -right-0.5 bg-red-500 text-white text-[10px] font-bold rounded-full min-w-[16px] h-4 flex items-center justify-center px-0.5"
          >
            {{ activityUnreadCount > 99 ? '99+' : activityUnreadCount }}
          </span>
        </button>
      </div>

      <!-- Connection indicator -->
      <div 
        :class="connectionDotClass"
        class="h-2 w-2 rounded-full"
        :title="`Connection: ${connectionStatus}`"
      />

      <!-- User Menu -->
      <div class="relative ml-1">
        <button
          data-testid="user-menu-trigger"
          @click="showUserMenu = !showUserMenu"
          class="relative flex min-h-11 items-center gap-2 rounded-r-2 py-1 pl-1.5 pr-2.5 transition-standard focus-ring hover:bg-bg-surface-1"
          :class="{ 'bg-bg-surface-1': showUserMenu }"
        >
          <RcAvatar 
            :userId="auth.user?.id"
            :src="auth.user?.avatar_url" 
            :username="auth.user?.username" 
            size="sm"
            class="w-7 h-7"
          />
          <span class="hidden max-w-[120px] truncate text-sm font-medium text-text-1 lg:block">
            {{ auth.user?.username }}
          </span>
        </button>

        <!-- User Dropdown Menu -->
        <Transition
          enter-active-class="transition-all duration-200 ease-out"
          enter-from-class="opacity-0 scale-95 -translate-y-1"
          enter-to-class="opacity-100 scale-100 translate-y-0"
          leave-active-class="transition-all duration-150 ease-in"
          leave-from-class="opacity-100 scale-100 translate-y-0"
          leave-to-class="opacity-0 scale-95 -translate-y-1"
        >
          <div 
            v-if="showUserMenu" 
            class="absolute top-full right-0 mt-2 w-64 bg-bg-surface-1 border border-border-1 rounded-r-3 shadow-2xl py-1 z-50 origin-top-right"
          >
            <!-- Custom Status -->
            <button 
              @click="openCustomStatus"
              class="w-full flex items-center gap-3 px-4 py-2.5 text-sm text-text-2 hover:bg-bg-surface-2 transition-colors"
            >
              <span v-if="auth.user?.status_emoji" class="text-base">{{ auth.user.status_emoji }}</span>
              <Smile v-else class="w-4 h-4 text-text-3" />
              <span class="truncate">{{ auth.user?.status_text || 'Set custom status' }}</span>
            </button>

            <div class="h-px bg-border-1 my-1" />

            <!-- Presence Options -->
            <button 
              @click="setPresence('online')"
              class="w-full flex items-center justify-between px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-colors"
            >
              <div class="flex items-center gap-3">
                <span class="w-2 h-2 rounded-full bg-success" />
                <span>Online</span>
              </div>
              <Check v-if="userPresence === 'online'" class="w-4 h-4 text-brand" />
            </button>

            <button 
              @click="setPresence('away')"
              class="w-full flex items-center justify-between px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-colors"
            >
              <div class="flex items-center gap-3">
                <span class="w-2 h-2 rounded-full bg-warning" />
                <span>Away</span>
              </div>
              <Check v-if="userPresence === 'away'" class="w-4 h-4 text-brand" />
            </button>

            <!-- DND with Submenu -->
            <button 
              @click="showDndSubmenu = !showDndSubmenu"
              class="w-full flex items-center justify-between px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-colors"
            >
              <div class="flex items-center gap-3">
                <span class="w-2 h-2 rounded-full bg-danger" />
                <div class="flex flex-col items-start">
                  <span>Do not disturb</span>
                  <span class="text-xs text-text-3">Pause notifications</span>
                </div>
              </div>
              <div class="flex items-center">
                <Check v-if="userPresence === 'dnd' && !showDndSubmenu" class="w-4 h-4 text-brand mr-2" />
                <ChevronDown v-if="!showDndSubmenu" class="w-4 h-4 text-text-3" />
                <ChevronUp v-else class="w-4 h-4 text-text-3" />
              </div>
            </button>

            <!-- DND Duration Submenu -->
            <div v-if="showDndSubmenu" class="bg-bg-surface-2/50 py-1">
              <button 
                v-for="duration in dndDurations" 
                :key="duration.value"
                @click="setDndWithDuration(duration.value)"
                class="w-full text-left pl-11 pr-4 py-1.5 text-sm text-text-3 hover:bg-bg-surface-2 hover:text-text-1 transition-colors"
              >
                {{ duration.label }}
              </button>
            </div>

            <button 
              @click="setPresence('offline')"
              class="w-full flex items-center justify-between px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-colors"
            >
              <div class="flex items-center gap-3">
                <span class="w-2 h-2 rounded-full border-2 border-text-3" />
                <span>Offline</span>
              </div>
              <Check v-if="userPresence === 'offline'" class="w-4 h-4 text-brand" />
            </button>

            <div class="h-px bg-border-1 my-1" />

            <!-- Profile -->
            <button 
              @click="openProfile"
              class="w-full flex items-center gap-3 px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-colors"
            >
              <User class="w-4 h-4 text-text-3" />
              <span>Profile</span>
            </button>

            <!-- Admin Console -->
            <button
              v-if="['system_admin', 'org_admin', 'admin', 'administrator'].includes(auth.user?.role || '')"
              @click="router.push('/admin'); showUserMenu = false"
              class="w-full flex items-center gap-3 px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-colors"
            >
              <Shield class="w-4 h-4 text-text-3" />
              <span>Admin Console</span>
            </button>

            <div class="h-px bg-border-1 my-1" />

            <!-- Log out -->
            <button 
              @click="auth.logout(); showUserMenu = false"
              class="w-full flex items-center gap-3 px-4 py-2 text-sm text-danger hover:bg-danger/5 transition-colors"
            >
              <LogOut class="w-4 h-4" />
              <span>Log out</span>
            </button>
          </div>
        </Transition>
        
        <!-- Click outside backdrop -->
        <div 
          v-if="showUserMenu" 
          class="fixed inset-0 z-40" 
          @click="showUserMenu = false"
        />
      </div>
    </div>

    <!-- Search Modal -->
    <SearchModal :show="showSearch" @close="showSearch = false" />

    <!-- Quick Switcher Modal -->
    <QuickSwitcherModal
      :is-open="quickSwitcher.isOpen.value"
      :items="quickSwitcher.allItems.value"
      :recent-items="quickSwitcher.recentItems.value"
      @select="handleQuickSwitcherSelect"
      @close="quickSwitcher.close()"
    />

    <!-- Set Status Modal -->
    <SetStatusModal :show="showSetStatus" @close="showSetStatus = false" />
  </header>

  <!-- Activity Feed Panel -->
  <ActivityFeed />
</template>
