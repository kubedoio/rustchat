<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useChannelStore } from '../../stores/channels';
import { useMessageStore } from '../../stores/messages';
import { useUnreadStore } from '../../stores/unreads';
import { useWebSocket } from '../../composables/useWebSocket';
import AppShell from '../../components/layout/AppShell.vue';
import ChannelHeader from '../../components/channel/ChannelHeader.vue';
import MessageList from '../../components/channel/MessageList.vue';
import MessageComposer from '../../components/composer/MessageComposer.vue';
import SavedMessagesPanel from '../../components/channel/SavedMessagesPanel.vue';
import PinnedMessagesPanel from '../../components/channel/PinnedMessagesPanel.vue';
import SearchPanel from '../../components/channel/SearchPanel.vue';
import ChannelMembersPanel from '../../components/channel/ChannelMembersPanel.vue';
import ChannelSettingsModal from '../../components/modals/ChannelSettingsModal.vue';
import VideoCallModal from '../../components/modals/VideoCallModal.vue';
import TypingIndicator from '../../components/channel/TypingIndicator.vue';
import { useUIStore } from '../../stores/ui';

const channelStore = useChannelStore();
const messageStore = useMessageStore();
const unreadStore = useUnreadStore();
const uiStore = useUIStore();
const { sendTyping, sendMessage, subscribe, unsubscribe } = useWebSocket();

const currentChannel = computed(() => channelStore.currentChannel);
const channelId = computed(() => channelStore.currentChannelId);

const messageListRef = ref<any>(null);

// Channel settings modal
const showChannelSettings = ref(false);

// Mark as read when channel changes
watch(channelId, (newId) => {
    if (newId) {
        unreadStore.markAsRead(newId);
    }
});

// Fetch messages when channel changes
watch(channelId, (newId, oldId) => {
    if (oldId) {
        unsubscribe(oldId);
    }
    if (newId) {
        messageStore.fetchMessages(newId);
        subscribe(newId);
    }
    showChannelSettings.value = false;
}, { immediate: true });

async function onSendMessage(content: string) {
    if (channelId.value) {
        // Optimistic send via WebSocket
        await sendMessage(channelId.value, content);
    }
}

function onTyping() {
    if (channelId.value) {
        sendTyping(channelId.value);
    }
}

function handleMessageReply(messageId: string) {
    uiStore.openRhs('thread', messageId);
}

function handleMessageDelete(messageId: string) {
    // Remove from local state - the API call is made in MessageItem
    if (channelId.value) {
        const messages = messageStore.messagesByChannel[channelId.value];
        if (messages) {
            const index = messages.findIndex(m => m.id === messageId);
            if (index !== -1) {
                messages.splice(index, 1);
            }
        }
    }
}

function handleMessageJump(messageId: string) {
    if (messageListRef.value) {
        messageListRef.value.scrollToMessage(messageId);
    }
}

function handleChannelDeleted() {
    channelStore.removeChannel(currentChannel.value?.id || '');
}
</script>

<template>
  <AppShell>
      <div class="flex h-full relative overflow-hidden">
          <!-- Background Gradient -->
          <div class="absolute inset-0 bg-slate-900 pointer-events-none z-0">
             <div class="absolute inset-0 bg-gradient-to-br from-indigo-900/20 via-slate-900 to-slate-900"></div>
          </div>

          <!-- Main Channel Area -->
          <div class="relative flex flex-col flex-1 min-w-0 z-10 bg-transparent">
              <!-- No Channel Selected -->
              <div v-if="!currentChannel" class="flex-1 flex items-center justify-center text-slate-500">
                  <div class="text-center">
                      <p class="text-lg">Select a channel to start messaging</p>
                      <p class="text-sm mt-2">Choose a channel from the sidebar</p>
                  </div>
              </div>
              
              <!-- Channel View -->
              <template v-else>
                  <!-- Header -->
                  <ChannelHeader 
                      :name="currentChannel.display_name || currentChannel.name" 
                      :topic="currentChannel.purpose || currentChannel.header"
                      :channelType="currentChannel.channel_type"
                      :channelId="currentChannel.id"
                      @openSettings="showChannelSettings = true"
                  />
                  
                  <!-- Messages -->
                  <MessageList 
                    ref="messageListRef"
                    :channelId="currentChannel.id"
                    @reply="handleMessageReply"
                    @delete="handleMessageDelete"
                  />

                  <!-- Typing Indicator -->
                  <TypingIndicator :channelId="currentChannel.id" />

                  <!-- Composer -->
                  <MessageComposer @send="onSendMessage" @typing="onTyping" />
              </template>
          </div>

          <!-- RHS Panels -->
          <ThreadPanel 
            v-if="uiStore.isRhsOpen && uiStore.rhsView === 'thread'"
            @close="uiStore.closeRhs"
          />

          <SavedMessagesPanel 
            v-if="uiStore.isRhsOpen && uiStore.rhsView === 'saved'"
            :show="true"
            @close="uiStore.closeRhs"
            @jump="handleMessageJump"
          />

          <PinnedMessagesPanel 
            v-if="uiStore.isRhsOpen && uiStore.rhsView === 'pinned'"
            :show="true"
            @close="uiStore.closeRhs"
            @jump="handleMessageJump"
          />

          <SearchPanel 
            v-if="uiStore.isRhsOpen && uiStore.rhsView === 'search'"
            :show="true"
            @close="uiStore.closeRhs"
            @jump="handleMessageJump"
          />

          <ChannelMembersPanel 
            v-if="uiStore.isRhsOpen && uiStore.rhsView === 'members' && currentChannel"
            :channelId="currentChannel.id"
            @close="uiStore.closeRhs"
          />
      </div>

      <!-- Channel Settings Modal -->
      <ChannelSettingsModal
        :isOpen="showChannelSettings"
        :channel="currentChannel"
        @close="showChannelSettings = false"
        @deleted="handleChannelDeleted"
      />

      <!-- Video Call Modal (Global for ChannelView context) -->
      <VideoCallModal
        :is-open="uiStore.isVideoCallOpen"
        :url="uiStore.videoCallUrl"
        @close="uiStore.closeVideoCall"
      />
  </AppShell>
</template>
