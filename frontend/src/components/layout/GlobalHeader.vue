<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { Bell, Search, HelpCircle, LogOut, Settings, Smile, Shield } from 'lucide-vue-next';
import { useAuthStore } from '../../stores/auth';
import { useUIStore } from '../../stores/ui';
import SearchModal from '../modals/SearchModal.vue';
import SetStatusModal from '../modals/SetStatusModal.vue';
import RcAvatar from '../ui/RcAvatar.vue';
import PresenceSelector from '../ui/PresenceSelector.vue';
import NotificationsDropdown from './NotificationsDropdown.vue';
import { useConfigStore } from '../../stores/config';
import { usePresenceStore } from '../../stores/presence';
import { useUnreadStore } from '../../stores/unreads';

const auth = useAuthStore();
const ui = useUIStore();
const configStore = useConfigStore();
const presenceStore = usePresenceStore();
const unreadStore = useUnreadStore();

const showSearch = ref(false);
const showUserMenu = ref(false);
const showSetStatus = ref(false);
const showNotifications = ref(false);

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
        showSearch.value = true;
    }
    if (e.key === 'Escape') {
        showSearch.value = false;
        showUserMenu.value = false;
    }
}

onMounted(() => {
    document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
    document.removeEventListener('keydown', handleKeydown);
});

async function setPresence(status: 'online' | 'away' | 'dnd' | 'offline') {
    await auth.updateStatus({ presence: status });
    presenceStore.updatePresenceFromEvent(auth.user?.id || '', status);
    showUserMenu.value = false;
}

const userPresence = computed(() => {
    return presenceStore.self?.presence || 'online';
});

const statusColor = computed(() => {
    switch (userPresence.value) {
        case 'online': return 'bg-green-500';
        case 'away': return 'bg-amber-500';
        case 'dnd': return 'bg-red-500';
        case 'offline': return 'bg-gray-400';
        default: return 'bg-green-500';
    }
});

const statusLabel = computed(() => {
     switch (userPresence.value) {
        case 'online': return 'Online';
        case 'away': return 'Away';
        case 'dnd': return 'Do not disturb';
        case 'offline': return 'Offline';
        default: return 'Online';
    }
});
</script>

<template>
  <header class="h-[60px] bg-gray-900 border-b border-gray-800 flex items-center justify-between px-3 text-white shrink-0 z-20 relative">
    <!-- Left: Logo & Team -->
    <div class="flex items-center min-w-[200px]">
      <div class="font-bold text-lg tracking-tight mr-4 flex items-center">
        <img v-if="configStore.siteConfig.logo_url" :src="configStore.siteConfig.logo_url" class="w-[50px] h-[50px] rounded mr-2 object-cover" alt="Logo" />
        <div v-else class="w-8 h-8 bg-primary rounded mr-2 flex items-center justify-center text-sm font-bold">{{ configStore.siteConfig.site_name.charAt(0).toUpperCase() }}</div>
        {{ configStore.siteConfig.site_name }}
      </div>
    </div>

    <!-- Center: Search -->
    <div class="flex-1 max-w-2xl px-4">
      <div 
        class="relative group cursor-pointer"
        @click="showSearch = true"
      >
        <div class="absolute inset-y-0 left-0 pl-2 flex items-center pointer-events-none text-gray-400">
          <Search class="w-4 h-4" />
        </div>
        <div 
          class="block w-full bg-gray-800 border-transparent rounded text-sm text-gray-400 pl-8 pr-3 py-1.5 transition-colors hover:bg-gray-700 flex items-center justify-between"
        >
          <span>Search {{ configStore.siteConfig.site_name }}</span>
          <kbd class="hidden sm:inline-flex px-1.5 py-0.5 bg-gray-700 text-gray-400 text-xs rounded">⌘K</kbd>
        </div>
      </div>
    </div>

    <!-- Right: Actions -->
    <div class="flex items-center space-x-3">
      <button class="text-gray-400 hover:text-white transition-colors">
        <HelpCircle class="w-5 h-5" />
      </button>
      
      <div class="relative">
        <button 
          @click="showNotifications = !showNotifications"
          class="relative text-gray-400 hover:text-white transition-colors p-1"
        >
          <Bell class="w-5 h-5" />
          <span 
            v-if="unreadStore.totalUnreadCount > 0"
            class="absolute top-0 right-0 block h-2.5 w-2.5 rounded-full ring-2 ring-gray-900 bg-red-500 animate-pulse"
          ></span>
        </button>
        <NotificationsDropdown v-if="showNotifications" @close="showNotifications = false" />
        <div v-if="showNotifications" class="fixed inset-0 z-40" @click="showNotifications = false"></div>
      </div>
      
      <!-- Presence Switcher -->
      <div class="mr-2">
        <PresenceSelector />
      </div>

      <!-- User Menu -->
      <div class="ml-2 relative">
        <div class="cursor-pointer relative" @click="showUserMenu = !showUserMenu">
           <RcAvatar 
             :userId="auth.user?.id"
             :src="auth.user?.avatar_url" 
             :username="auth.user?.username" 
             size="md"
           />
        </div>

        <!-- Dropdown -->
        <div v-if="showUserMenu" class="absolute top-full right-0 mt-2 w-64 bg-gray-800 border border-gray-700 rounded-lg shadow-xl py-1 z-50 origin-top-right focus:outline-none">
            <!-- User Info -->
            <div class="px-4 py-3 border-b border-gray-700">
                <p class="text-sm font-medium text-white truncate">{{ auth.user?.display_name || auth.user?.username }}</p>
                <div class="flex items-center mt-1">
                    <div class="h-2 w-2 rounded-full mr-2" :class="statusColor"></div>
                    <p class="text-xs text-gray-400">{{ statusLabel }}</p>
                </div>
            </div>

            <!-- Custom Status -->
            <div class="px-1 py-1">
                <button 
                    @click="showSetStatus = true; showUserMenu = false"
                    class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center transition-colors border border-gray-700 bg-gray-700/30"
                >
                    <span v-if="auth.user?.status_emoji" class="mr-2">{{ auth.user.status_emoji }}</span>
                    <Smile v-else class="w-4 h-4 mr-2" />
                    <span class="truncate">{{ auth.user?.status_text || 'Update your status' }}</span>
                </button>
            </div>

            <div class="border-t border-gray-700 my-1"></div>

            <!-- Presence Options -->
            <div class="px-1 py-1">
                 <button @click="setPresence('online')" class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center justify-between group">
                    <div class="flex items-center">
                        <div class="h-2 w-2 rounded-full bg-green-500 mr-2"></div>
                        Online
                    </div>
                </button>
                <button @click="setPresence('away')" class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center justify-between group">
                    <div class="flex items-center">
                        <div class="h-2 w-2 rounded-full bg-yellow-500 mr-2"></div>
                        Away
                    </div>
                </button>
                 <button @click="setPresence('dnd')" class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center justify-between group">
                    <div class="flex items-center">
                        <div class="h-2 w-2 rounded-full bg-red-500 mr-2"></div>
                        Do not disturb
                    </div>
                </button>
                 <button @click="setPresence('offline')" class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center justify-between group">
                    <div class="flex items-center">
                        <div class="h-2 w-2 rounded-full ring-1 ring-gray-400 mr-2"></div>
                        Offline
                    </div>
                </button>
            </div>

            <div class="border-t border-gray-700 my-1"></div>

            <!-- Links -->
            <div class="px-1 py-1">
                <button 
                  v-if="auth.user?.role === 'system_admin' || auth.user?.role === 'org_admin'"
                  @click="$router.push('/admin'); showUserMenu = false"
                  class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center transition-colors"
                >
                    <Shield class="w-4 h-4 mr-2" />
                    System Console
                </button>
                <button 
                  @click="ui.openSettings(); showUserMenu = false"
                  class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center transition-colors"
                >
                    <Settings class="w-4 h-4 mr-2" />
                    Profile & Preferences
                </button>
                <button 
                    @click="auth.logout()"
                     class="w-full text-left px-3 py-2 text-sm text-gray-300 hover:bg-gray-700 hover:text-white rounded-md flex items-center transition-colors"
                >
                    <LogOut class="w-4 h-4 mr-2" />
                    Sign out
                </button>
            </div>
        </div>
        
        <!-- Click outside -->
        <div v-if="showUserMenu" class="fixed inset-0 z-40" @click="showUserMenu = false"></div>
      </div>
    </div>

    <!-- Search Modal -->
    <SearchModal :show="showSearch" @close="showSearch = false" />
    <!-- Set Status Modal -->
    <SetStatusModal :show="showSetStatus" @close="showSetStatus = false" />
  </header>
</template>
