<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import {
  Hash, Lock, ChevronDown, ChevronRight, Plus, MessageCircle, Settings, Compass, Shield, Check, LogOut
} from 'lucide-vue-next';import { useTeamStore } from '../../stores/teams';
import { useChannelStore } from '../../stores/channels';
import { useAuthStore } from '../../stores/auth';
import { usePresenceStore } from '../../stores/presence';
import { useUnreadStore } from '../../stores/unreads';
import CreateChannelModal from '../modals/CreateChannelModal.vue';
import DirectMessageModal from '../modals/DirectMessageModal.vue';
import TeamSettingsModal from '../modals/TeamSettingsModal.vue';
import BrowseTeamsModal from '../modals/BrowseTeamsModal.vue';
import BrowseChannelsModal from '../modals/BrowseChannelsModal.vue';

const teamStore = useTeamStore();
const channelStore = useChannelStore();
const authStore = useAuthStore();
const presenceStore = usePresenceStore();
const unreadStore = useUnreadStore();

const showCreateModal = ref(false);
const showDirectMessageModal = ref(false);
const showTeamSettings = ref(false);
const showTeamMenu = ref(false);
const showBrowseTeams = ref(false);
const showBrowseChannels = ref(false);

// Reload channels when team changes
watch(() => teamStore.currentTeamId, (teamId) => {
    if (teamId) {
        channelStore.fetchChannels(teamId);
        teamStore.fetchMembers(teamId);
    } else {
        channelStore.clearChannels();
    }
}, { immediate: true });

const categories = computed(() => {
    // Helper to deduplicate channels by ID
    const dedupe = (channels: any[]) => {
        const unique = new Map();
        channels.forEach(c => {
            if (!unique.has(c.id)) {
                unique.set(c.id, c);
            }
        });
        const result = Array.from(unique.values());
        console.log('Deduped DMs:', result.map(c => ({ id: c.id, name: c.name })));
        return result;
    };

    return [
        {
            id: 'channels',
            name: 'Channels',
            collapsed: false,
            channels: dedupe(channelStore.publicChannels).map(c => ({
                id: c.id,
                name: c.display_name || c.name,
                type: 'public',
                unread: unreadStore.getChannelUnreadCount(c.id),
                mention: (unreadStore.channelMentions[c.id] || 0) > 0,
            })),
        },
        {
            id: 'private',
            name: 'Private Channels',
            collapsed: false,
            channels: dedupe(channelStore.privateChannels).map(c => ({
                id: c.id,
                name: c.display_name || c.name,
                type: 'private',
                unread: unreadStore.getChannelUnreadCount(c.id),
                mention: (unreadStore.channelMentions[c.id] || 0) > 0,
            })),
        },
        {
            id: 'dms',
            name: 'Direct Messages',
            collapsed: false,
            channels: dedupe(channelStore.directMessages).map(c => {
                let DisplayName = c.display_name || c.name;
                let otherId = '';
                
                // If it's a DM channel with a generated name, try to resolve the other user's name
                if (c.name.startsWith('dm_')) {
                    const parts = c.name.split('_');
                    if (parts.length === 3) {
                        otherId = parts[1] === authStore.user?.id ? parts[2] : parts[1];
                        const member = teamStore.members.find(m => m.user_id === otherId);
                        if (member) {
                            DisplayName = member.display_name || member.username;
                        }
                    }
                }
                
                // Get status from presence store
                const status = otherId ? (presenceStore.presenceMap.get(otherId)?.presence || 'offline') : 'offline';
                
                return {
                    id: c.id,
                    name: DisplayName,
                    type: 'dm',
                    status: status,
                    unread: unreadStore.getChannelUnreadCount(c.id),
                    mention: (unreadStore.channelMentions[c.id] || 0) > 0,
                } as any;
            }),
        }
    ];
});

function selectChannel(channelId: string) {
    channelStore.selectChannel(channelId);
    // Mark as read when selecting
    unreadStore.markAsRead(channelId);
}

const collapsedCategories = ref(new Set<string>());

function toggleCategory(catId: string) {
    if (collapsedCategories.value.has(catId)) {
        collapsedCategories.value.delete(catId);
    } else {
        collapsedCategories.value.add(catId);
    }
}

function isCategoryCollapsed(catId: string) {
    return collapsedCategories.value.has(catId);
}

function handleTeamDeleted() {
    teamStore.removeTeam(teamStore.currentTeamId || '');
}

function handleAddCategory(catId: string) {
    if (catId === 'dms') {
        showDirectMessageModal.value = true;
    } else {
        showCreateModal.value = true;
    }
}

async function handleLeaveTeam() {
    if (!teamStore.currentTeam) return;
    if (!confirm(`Are you sure you want to leave ${teamStore.currentTeam.display_name || teamStore.currentTeam.name}?`)) return;
    
    try {
        await teamStore.leaveTeam(teamStore.currentTeam.id);
        showTeamMenu.value = false;
    } catch (e) {
        console.error('Failed to leave team', e);
    }
}
</script>

<template>
  <aside class="w-[260px] flex flex-col shrink-0 select-none relative z-20">
    <!-- Glass Background Layer -->
    <!-- Background Layer -->
    <div class="absolute inset-0 bg-[#3F0E40] border-r border-[#522653]"></div>

    <!-- Content Layer -->
    <div class="relative flex-1 flex flex-col text-slate-300">
    <!-- Team Header -->
    <div 
      class="h-12 flex items-center justify-between px-4 hover:bg-[#350d36] cursor-pointer transition-all duration-200 group relative border-b border-[#522653]"
      @click="showTeamMenu = !showTeamMenu"
    >
      <h2 class="font-bold truncate text-white text-[15px] tracking-tight">
        {{ teamStore.currentTeam?.display_name || teamStore.currentTeam?.name || 'Select Team' }}
      </h2>
      <ChevronDown class="w-4 h-4 text-slate-400 group-hover:text-white transition-transform duration-200" :class="{ 'rotate-180': showTeamMenu }" />
      
      <!-- Team Dropdown Menu -->
      <div 
        v-if="showTeamMenu"
        class="absolute top-full left-0 right-0 mt-1 bg-gray-800 rounded-lg shadow-xl border border-gray-700 py-1 z-50"
        @click.stop
      >
        <button
          v-if="authStore.user?.role === 'system_admin' || authStore.user?.role === 'org_admin'"
          @click="$router.push('/admin')"
          class="w-full flex items-center px-4 py-2 text-sm text-gray-300 hover:bg-gray-700 transition-colors"
        >
          <Shield class="w-4 h-4 mr-3" />
          System Console
        </button>
        <div class="h-px bg-gray-700 my-1 font-medium"></div>
        <button
          @click="showBrowseTeams = true; showTeamMenu = false"
          class="w-full flex items-center px-4 py-2 text-sm text-gray-300 hover:bg-gray-700 transition-colors"
        >
          <Compass class="w-4 h-4 mr-3" />
          Browse Teams
        </button>
        <button
          @click="showTeamSettings = true; showTeamMenu = false"
          class="w-full flex items-center px-4 py-2 text-sm text-gray-300 hover:bg-gray-700 transition-colors"
        >
          <Settings class="w-4 h-4 mr-3" />
          Team Settings
        </button>
        <div class="h-px bg-gray-700 my-1 font-medium"></div>
        <button
          @click="handleLeaveTeam"
          class="w-full flex items-center px-4 py-2 text-sm text-red-400 hover:bg-red-900/20 transition-colors"
        >
          <LogOut class="w-4 h-4 mr-3" />
          Leave Team
        </button>
      </div>
    </div>

    <!-- Click outside to close menu -->
    <div v-if="showTeamMenu" class="fixed inset-0 z-40" @click="showTeamMenu = false"></div>

    <!-- Scrollable Content -->
    <div class="flex-1 overflow-y-auto p-2 space-y-4 custom-scrollbar">
      
      <!-- Loading State -->
      <div v-if="channelStore.loading" class="text-center py-4 text-gray-500 text-sm">
          Loading channels...
      </div>

      <!-- Categories -->
      <div v-for="cat in categories" :key="cat.id">
        <!-- Category Header -->
        <div 
          class="flex items-center justify-between px-3 py-1 text-[#BEABBE] hover:text-white cursor-pointer group mb-1"
          @click="toggleCategory(cat.id)"
        >
          <div class="flex items-center text-xs font-semibold uppercase tracking-wide">
            <component :is="isCategoryCollapsed(cat.id) ? ChevronRight : ChevronDown" class="w-3 h-3 mr-1" />
            {{ cat.name }}
          </div>
          <button 
            @click.stop="handleAddCategory(cat.id)"
            class="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-gray-700 rounded"
          >
            <Plus class="w-3 h-3" />
          </button>
        </div>

        <!-- Channels List -->
        <div v-if="!isCategoryCollapsed(cat.id)" class="space-y-0.5 mt-1">
          <div 
            v-for="channel in cat.channels" 
            :key="channel.id"
            @click="selectChannel(channel.id)"
            class="group/item relative px-3 py-1.5 mx-2 rounded flex items-center justify-between cursor-pointer transition-all duration-200"
            :class="{ 'bg-[#1164A3] text-white': channelStore.currentChannelId === channel.id, 'text-[#BEABBE] hover:bg-[#350d36] hover:text-white': channelStore.currentChannelId !== channel.id }"
          >
            <!-- Active Pill Indicator (Removing sidebar pill for Slack style active row) -->

            <div class="flex items-center min-w-0" :class="{ 'opacity-100': channelStore.currentChannelId === channel.id, 'opacity-70 group-hover/item:opacity-100': channelStore.currentChannelId !== channel.id }">
              <!-- Icon -->
              <span class="mr-2 shrink-0 transition-opacity" :class="channelStore.currentChannelId === channel.id ? 'text-white' : 'text-[#BEABBE] opacity-70 group-hover/item:opacity-100'">
                <Hash v-if="channel.type === 'public'" class="w-4 h-4" />
                <Lock v-else-if="channel.type === 'private'" class="w-3.5 h-3.5" />
                <div v-else-if="channel.type === 'dm'" class="relative flex items-center justify-center w-3.5 h-3.5">
                    <div 
                        class="w-2.5 h-2.5 rounded-full border border-[#3F0E40]"
                        :class="{ 'bg-green-500': channel.status === 'online', 'bg-transparent border-2 border-[#BEABBE]': channel.status === 'offline', 'bg-yellow-500': channel.status === 'away' }"
                    ></div>
                </div>
                <MessageCircle v-else class="w-4 h-4" />
              </span>
              
              <!-- Name -->
              <span class="truncate text-[15px]" :class="{ 'font-bold': channel.unread > 0, 'opacity-100': channelStore.currentChannelId === channel.id, 'opacity-90': channelStore.currentChannelId !== channel.id }">
                {{ channel.name }}
              </span>
            </div>

            <!-- Status Indicator for DM (Right Side) -->
            <div v-if="channel.type === 'dm'" class="ml-2 shrink-0">
                 <div 
                    class="w-2 h-2 rounded-full"
                    :class="{ 'bg-green-500': channel.status === 'online', 'border border-[#BEABBE]': channel.status === 'offline', 'bg-yellow-500': channel.status === 'away' }"
                ></div>
            </div>

            <!-- Badges & Actions -->
            <div class="flex items-center ml-2 space-x-1.5 min-w-0">
               <!-- Mark as read on hover -->
               <button 
                 v-if="channel.unread > 0"
                 @click.stop="unreadStore.markAsRead(channel.id)"
                 class="opacity-0 group-hover/item:opacity-100 flex items-center justify-center w-5 h-5 hover:bg-slate-700/50 rounded transition-opacity"
                 title="Mark as read"
               >
                 <Check class="w-3.5 h-3.5 text-slate-300" />
               </button>

               <div v-if="channel.mention" class="shrink-0 w-2.5 h-2.5 rounded-full bg-rose-500 ring-2 ring-slate-900 shadow-[0_0_8px_rgba(244,63,94,0.6)]"></div>
               <div v-if="channel.unread > 0" class="shrink-0 px-1.5 h-5 flex items-center justify-center rounded-md bg-slate-700/80 text-[10px] font-bold text-white min-w-[20px]">
                 {{ channel.unread > 99 ? '99+' : channel.unread }}
               </div>
            </div>
          </div>
          
          <!-- Empty Category State -->
          <div v-if="cat.channels.length === 0" class="px-8 py-2 text-xs text-gray-600 italic">
            No channels
          </div>
        </div>
      </div>
    </div>

    <!-- Sidebar Footer -->
    <div class="p-2 border-t border-gray-800 space-y-1">
        <button
          v-if="Object.values(unreadStore.channelUnreads).some(c => c > 0)"
          @click="unreadStore.markAllAsRead()"
          class="w-full flex items-center justify-start px-2 py-2 text-sm text-gray-400 hover:bg-gray-800 rounded transition-colors text-left"
        >
            <div class="w-6 h-6 rounded bg-gray-700/50 flex items-center justify-center mr-2">
                <Check class="w-4 h-4 text-gray-300" />
            </div>
            <span>Mark all as read</span>
        </button>
        <button 
          v-if="Object.values(unreadStore.channelUnreads).some(c => c > 0)"
          @click="unreadStore.markAllAsRead()"
          class="w-full flex items-center justify-start px-2 py-2 text-sm text-gray-400 hover:bg-gray-800 rounded transition-colors text-left"
        >
            <div class="w-6 h-6 rounded bg-gray-700/50 flex items-center justify-center mr-2">
                <Check class="w-4 h-4 text-gray-300" />
            </div>
            <span>Mark all as read</span>
        </button>
        <button 
          @click="showBrowseChannels = true"
          class="w-full flex items-center justify-start px-2 py-2 text-sm text-gray-400 hover:bg-gray-800 rounded transition-colors text-left"
        >
            <div class="w-6 h-6 rounded bg-gray-700/50 flex items-center justify-center mr-2">
                <Compass class="w-4 h-4 text-gray-300" />
            </div>
            <span>Browse channels</span>
        </button>
        <button 
          @click="showCreateModal = true"
          class="w-full flex items-center justify-start px-2 py-2 text-sm text-gray-400 hover:bg-gray-800 rounded transition-colors text-left"
        >
            <div class="w-6 h-6 rounded bg-gray-700 flex items-center justify-center mr-2">
                <Plus class="w-4 h-4 text-gray-300" />
            </div>
            <span>Create channel</span>
        </button>
    </div>

    <!-- Create Channel Modal -->
    <CreateChannelModal :show="showCreateModal" @close="showCreateModal = false" />
    
    <!-- Direct Message Modal -->
    <DirectMessageModal :show="showDirectMessageModal" @close="showDirectMessageModal = false" />
    
    <!-- Team Settings Modal -->
    <TeamSettingsModal 
      :isOpen="showTeamSettings" 
      :team="teamStore.currentTeam"
      @close="showTeamSettings = false"
      @deleted="handleTeamDeleted"
    />

    <BrowseTeamsModal 
      v-if="showBrowseTeams"
      :open="showBrowseTeams"
      @close="showBrowseTeams = false"
    />

    <BrowseChannelsModal 
      v-if="showBrowseChannels"
      :open="showBrowseChannels"
      @close="showBrowseChannels = false"
    />
    </div>
  </aside>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #374151;
  border-radius: 4px;
}
</style>
