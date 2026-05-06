<script setup lang="ts">
import { X } from 'lucide-vue-next';
import { useUIStore } from '../../stores/ui';
import { useChannelStore } from '../../stores/channels';
import ThreadPanel from '../channel/ThreadPanel.vue';
import SearchPanel from '../channel/SearchPanel.vue';
import ChannelMembersPanel from '../channel/ChannelMembersPanel.vue';
import ChannelInfoPanel from '../channel/ChannelInfoPanel.vue';
import SavedMessagesPanel from '../channel/SavedMessagesPanel.vue';
import PinnedMessagesPanel from '../channel/PinnedMessagesPanel.vue';

const emit = defineEmits<{
  (e: 'jump', messageId: string): void;
  (e: 'openSettings'): void;
}>();

const ui = useUIStore();
const channelStore = useChannelStore();

function handleOpenSettings() {
  emit('openSettings');
}

function handleJump(messageId: string) {
  emit('jump', messageId);
}
</script>

<template>
  <aside 
    class="w-[400px] bg-surface border-l border-border-dim flex flex-col shadow-2xl z-20 shrink-0 transition-all duration-300 ease-out"
    :class="{ 'translate-x-0': ui.isRhsOpen, 'translate-x-full': !ui.isRhsOpen }"
  >
    <!-- Header -->
    <div
      v-if="ui.rhsView !== 'saved' && ui.rhsView !== 'pinned'"
      class="h-12 border-b border-border-dim flex items-center justify-between px-4 shrink-0 bg-surface-dim"
    >
        <h3 class="font-bold text-[15px] text-gray-900 uppercase tracking-wider">
            <span v-if="ui.rhsView === 'thread'">Thread</span>
            <span v-else-if="ui.rhsView === 'search'">Search</span>
            <span v-else-if="ui.rhsView === 'info'">Channel Info</span>
            <span v-else-if="ui.rhsView === 'members'">Members</span>
        </h3>
        <button 
          @click="ui.closeRhs()" 
          class="p-1.5 hover:bg-gray-200 rounded-lg text-gray-500 transition-standard focus-ring"
          aria-label="Close sidebar"
          title="Close sidebar"
        >
            <X class="w-5 h-5" />
        </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-hidden flex flex-col bg-surface relative">
         <ThreadPanel v-if="ui.rhsView === 'thread'" />
         <SavedMessagesPanel v-else-if="ui.rhsView === 'saved'" :show="true" @close="ui.closeRhs()" @jump="handleJump" />
         <PinnedMessagesPanel v-else-if="ui.rhsView === 'pinned'" :show="true" @close="ui.closeRhs()" @jump="handleJump" />
         <SearchPanel v-else-if="ui.rhsView === 'search' && channelStore.currentChannelId" :channelId="channelStore.currentChannelId" @close="ui.closeRhs()" @jump="handleJump" />
         <ChannelMembersPanel v-else-if="ui.rhsView === 'members' && channelStore.currentChannelId" :channelId="channelStore.currentChannelId" @close="ui.closeRhs()" />
         <ChannelInfoPanel v-else-if="ui.rhsView === 'info' && channelStore.currentChannelId" :channelId="channelStore.currentChannelId" @close="ui.closeRhs()" @openSettings="handleOpenSettings" />
         
         <div v-else-if="!ui.rhsView || !['thread', 'saved', 'pinned', 'search', 'members', 'info'].includes(ui.rhsView)" class="flex-1 flex items-center justify-center text-gray-400">
             <p class="text-sm">No content</p>
         </div>
    </div>
  </aside>
</template>
