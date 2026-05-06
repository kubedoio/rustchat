<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import SettingItemMin from '../SettingItemMin.vue'
import SettingItemMax from '../SettingItemMax.vue'
import { usePreferencesStore } from '../../../stores/preferences'

const preferencesStore = usePreferencesStore()
const expandedRow = ref<string | null>(null)
const saving = ref(false)

// Local preference states for editing
const localGroupUnread = ref<'never' | 'only_for_favorites' | 'always'>('never')
const localLimitDMs = ref<'all' | '10' | '20' | '40'>('all')

// Display labels
const groupUnreadLabel = computed(() => {
  const labels: Record<string, string> = {
    never: 'Off',
    only_for_favorites: 'For favorites only',
    always: 'On'
  }
  return labels[preferencesStore.preferences?.group_unread_channels || 'never'] || 'Off'
})

const limitDMsLabel = computed(() => {
  const limit = preferencesStore.preferences?.limit_visible_dms_gms
  if (limit === 'all' || !limit) return 'All direct messages'
  return `${limit} direct messages`
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

  localGroupUnread.value = prefs.group_unread_channels || 'never'
  localLimitDMs.value = prefs.limit_visible_dms_gms || 'all'
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
    <!-- 1. Group unread channels separately -->
    <div v-if="expandedRow !== 'group_unread'">
      <SettingItemMin
        label="Group unread channels separately"
        :value="groupUnreadLabel"
        description="Group unread channels above read channels in the channel sidebar"
        @click="expandRow('group_unread')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Group unread channels separately"
      description="Control how unread channels are grouped in the sidebar"
      :loading="saving"
      @save="savePreference({ group_unread_channels: localGroupUnread })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localGroupUnread"
            value="never"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">Off</div>
            <div class="text-xs text-text-3">Show channels in their normal order</div>
          </div>
        </label>
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localGroupUnread"
            value="only_for_favorites"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">For favorites only</div>
            <div class="text-xs text-text-3">Only group unread channels in Favorites</div>
          </div>
        </label>
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localGroupUnread"
            value="always"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">On</div>
            <div class="text-xs text-text-3">Group all unread channels at the top</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 2. Number of direct messages to show -->
    <div v-if="expandedRow !== 'limit_dms'">
      <SettingItemMin
        label="Number of direct messages to show"
        :value="limitDMsLabel"
        description="Limit the number of direct messages shown in the sidebar"
        @click="expandRow('limit_dms')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Number of direct messages to show"
      description="Control how many direct message conversations appear in the sidebar"
      :loading="saving"
      @save="savePreference({ limit_visible_dms_gms: localLimitDMs })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localLimitDMs"
            value="all"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">All direct messages</div>
            <div class="text-xs text-text-3">Show all your direct messages</div>
          </div>
        </label>
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localLimitDMs"
            value="40"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">40 direct messages</div>
            <div class="text-xs text-text-3">Show the 40 most recent conversations</div>
          </div>
        </label>
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localLimitDMs"
            value="20"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">20 direct messages</div>
            <div class="text-xs text-text-3">Show the 20 most recent conversations</div>
          </div>
        </label>
        <label class="flex items-center gap-3 p-3 border border-border-1 rounded-lg cursor-pointer hover:bg-bg-surface-2">
          <input
            type="radio"
            v-model="localLimitDMs"
            value="10"
            class="w-4 h-4 text-primary"
          />
          <div class="flex-1">
            <div class="text-sm font-medium text-text-1">10 direct messages</div>
            <div class="text-xs text-text-3">Show the 10 most recent conversations</div>
          </div>
        </label>
      </div>
    </SettingItemMax>
  </div>
</template>
