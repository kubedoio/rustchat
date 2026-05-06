<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import SettingItemMin from '../SettingItemMin.vue'
import SettingItemMax from '../SettingItemMax.vue'
import { usePreferencesStore } from '../../../stores/preferences'

const preferencesStore = usePreferencesStore()
const expandedRow = ref<string | null>(null)
const saving = ref(false)

// Local preference states for editing
const localSendOnCtrlEnter = ref(false)
const localEnableFormatting = ref(true)
const localEnableJoinLeave = ref(true)
const localEnablePerformanceDebug = ref(false)
const localUnreadScrollPosition = ref<'start' | 'last' | 'end'>('last')
const localSyncDrafts = ref(true)

// Display labels
const sendOnCtrlEnterLabel = computed(() => {
  return preferencesStore.preferences?.send_on_ctrl_enter ? 'On' : 'Off'
})

const enableFormattingLabel = computed(() => {
  return preferencesStore.preferences?.enable_post_formatting !== false ? 'On' : 'Off'
})

const enableJoinLeaveLabel = computed(() => {
  return preferencesStore.preferences?.enable_join_leave_messages !== false ? 'On' : 'Off'
})

const performanceDebugLabel = computed(() => {
  return preferencesStore.preferences?.enable_performance_debugging ? 'On' : 'Off'
})

const unreadScrollLabel = computed(() => {
  const labels: Record<string, string> = {
    start: 'Start of channel',
    last: 'Last viewed post',
    end: 'End of channel'
  }
  return labels[preferencesStore.preferences?.unread_scroll_position || 'last'] || 'Last viewed post'
})

const syncDraftsLabel = computed(() => {
  return preferencesStore.preferences?.sync_drafts !== false ? 'On' : 'Off'
})

// Initialize local states from preferences on mount
onMounted(() => {
  preferencesStore.fetchPreferences().then(() => {
    syncLocalState()
  })
})

function syncLocalState() {
  const prefs = preferencesStore.preferences
  if (!prefs) return

  localSendOnCtrlEnter.value = prefs.send_on_ctrl_enter ?? false
  localEnableFormatting.value = prefs.enable_post_formatting ?? true
  localEnableJoinLeave.value = prefs.enable_join_leave_messages ?? true
  localEnablePerformanceDebug.value = prefs.enable_performance_debugging ?? false
  localUnreadScrollPosition.value = prefs.unread_scroll_position || 'last'
  localSyncDrafts.value = prefs.sync_drafts ?? true
}

function expandRow(rowId: string) {
  if (expandedRow.value === rowId) {
    return
  }
  syncLocalState()
  expandedRow.value = rowId
}

async function savePreference(updates: Record<string, unknown>) {
  saving.value = true
  try {
    await preferencesStore.updatePreferences(updates)
    expandedRow.value = null
  } finally {
    saving.value = false
  }
}

function cancelEdit() {
  syncLocalState()
  expandedRow.value = null
}
</script>

<template>
  <div class="space-y-1">
    <!-- 1. Send messages on Ctrl+Enter -->
    <div v-if="expandedRow !== 'send_on_ctrl_enter'">
      <SettingItemMin
        label="Send messages on Ctrl+Enter"
        :value="sendOnCtrlEnterLabel"
        description="Use Ctrl+Enter to send messages (Enter for new line)"
        @click="expandRow('send_on_ctrl_enter')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Send messages on Ctrl+Enter"
      description="Control message sending behavior"
      :loading="saving"
      @save="savePreference({ send_on_ctrl_enter: localSendOnCtrlEnter })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center justify-between p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <div>
            <div class="text-sm font-medium text-text-1">Send messages on Ctrl+Enter</div>
            <div class="text-xs text-text-3">When enabled, press Ctrl+Enter to send. Press Enter alone for a new line.</div>
          </div>
          <input
            type="checkbox"
            v-model="localSendOnCtrlEnter"
            class="w-5 h-5 text-primary rounded"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 2. Enable post formatting -->
    <div v-if="expandedRow !== 'enable_formatting'">
      <SettingItemMin
        label="Enable post formatting"
        :value="enableFormattingLabel"
        description="Show formatting options in the message input"
        @click="expandRow('enable_formatting')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Enable post formatting"
      description="Control formatting toolbar visibility"
      :loading="saving"
      @save="savePreference({ enable_post_formatting: localEnableFormatting })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center justify-between p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <div>
            <div class="text-sm font-medium text-text-1">Enable post formatting</div>
            <div class="text-xs text-text-3">Show the formatting toolbar in the message input</div>
          </div>
          <input
            type="checkbox"
            v-model="localEnableFormatting"
            class="w-5 h-5 text-primary rounded"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 3. Enable join/leave messages -->
    <div v-if="expandedRow !== 'join_leave'">
      <SettingItemMin
        label="Enable join/leave messages"
        :value="enableJoinLeaveLabel"
        description="Show when users join or leave channels"
        @click="expandRow('join_leave')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Enable join/leave messages"
      description="Control visibility of join and leave messages"
      :loading="saving"
      @save="savePreference({ enable_join_leave_messages: localEnableJoinLeave })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center justify-between p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <div>
            <div class="text-sm font-medium text-text-1">Enable join/leave messages</div>
            <div class="text-xs text-text-3">Display system messages when users join or leave channels</div>
          </div>
          <input
            type="checkbox"
            v-model="localEnableJoinLeave"
            class="w-5 h-5 text-primary rounded"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 4. Enable performance debugging -->
    <div v-if="expandedRow !== 'performance_debug'">
      <SettingItemMin
        label="Enable performance debugging"
        :value="performanceDebugLabel"
        description="Show performance metrics for debugging"
        @click="expandRow('performance_debug')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Enable performance debugging"
      description="Enable client-side performance debugging"
      :loading="saving"
      @save="savePreference({ enable_performance_debugging: localEnablePerformanceDebug })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center justify-between p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <div>
            <div class="text-sm font-medium text-text-1">Enable performance debugging</div>
            <div class="text-xs text-text-3">Show performance metrics and debugging information</div>
          </div>
          <input
            type="checkbox"
            v-model="localEnablePerformanceDebug"
            class="w-5 h-5 text-primary rounded"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 5. Unread scroll position -->
    <div v-if="expandedRow !== 'unread_scroll'">
      <SettingItemMin
        label="Unread scroll position"
        :value="unreadScrollLabel"
        description="Where to scroll when viewing channels with unread messages"
        @click="expandRow('unread_scroll')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Unread scroll position"
      description="Choose where to scroll when opening a channel with unread messages"
      :loading="saving"
      @save="savePreference({ unread_scroll_position: localUnreadScrollPosition })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localUnreadScrollPosition"
            value="start"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">Start of channel</div>
            <div class="text-xs text-text-3">Always scroll to the beginning of the channel</div>
          </div>
        </label>
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localUnreadScrollPosition"
            value="last"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">Last viewed post</div>
            <div class="text-xs text-text-3">Scroll to the last message you read</div>
          </div>
        </label>
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localUnreadScrollPosition"
            value="end"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">End of channel</div>
            <div class="text-xs text-text-3">Always scroll to the most recent messages</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 6. Sync draft messages -->
    <div v-if="expandedRow !== 'sync_drafts'">
      <SettingItemMin
        label="Sync draft messages"
        :value="syncDraftsLabel"
        description="Sync message drafts across devices"
        @click="expandRow('sync_drafts')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Sync draft messages"
      description="Control draft message synchronization"
      :loading="saving"
      @save="savePreference({ sync_drafts: localSyncDrafts })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center justify-between p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <div>
            <div class="text-sm font-medium text-text-1">Sync draft messages</div>
            <div class="text-xs text-text-3">Synchronize message drafts across all your devices</div>
          </div>
          <input
            type="checkbox"
            v-model="localSyncDrafts"
            class="w-5 h-5 text-primary rounded"
          />
        </label>
      </div>
    </SettingItemMax>
  </div>
</template>
