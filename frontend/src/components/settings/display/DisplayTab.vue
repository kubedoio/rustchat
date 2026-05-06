<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import SettingItemMin from '../SettingItemMin.vue'
import SettingItemMax from '../SettingItemMax.vue'
import ThemeEditor from './ThemeEditor.vue'
import { useThemeStore, THEME_OPTIONS, type Theme } from '../../../stores/theme'
import { usePreferencesStore } from '../../../stores/preferences'


const themeStore = useThemeStore()
const preferencesStore = usePreferencesStore()

// Expanded row state
const expandedRow = ref<string | null>(null)

// Local theme state for edit mode
const editingTheme = ref<Theme>(themeStore.theme)
const savingTheme = ref(false)

// Local preference states for editing
const localCollapsedReplyThreads = ref(false)
const localUseMilitaryTime = ref(false)
const localTeammateNameDisplay = ref<'username' | 'nickname' | 'full_name'>('username')
const localAvailabilityVisible = ref(true)
const localShowLastActive = ref(true)
const localTimezoneMode = ref<'auto' | 'manual'>('auto')
const localTimezone = ref('UTC')
const localLinkPreviews = ref(true)
const localImagePreviews = ref(true)
const localMessageDisplay = ref<'standard' | 'compact'>('standard')
const localClickToReply = ref(true)
const localChannelDisplay = ref<'full' | 'centered'>('full')
const localQuickReactions = ref(true)
const localRenderEmoticons = ref(true)
const localLanguage = ref('en')
const localFontSize = ref<13 | 14 | 16>(14)

const saving = ref(false)

const optionCardClass = 'flex items-center gap-3 rounded-lg border border-border-1 bg-bg-surface-1 p-3 text-text-1 transition-standard hover:bg-bg-surface-2'
const optionToggleCardClass = 'flex items-center justify-between rounded-lg border border-border-1 bg-bg-surface-1 p-3 text-text-1 transition-standard hover:bg-bg-surface-2'
const optionTitleClass = 'text-sm font-medium text-text-1'
const optionDescriptionClass = 'text-xs text-text-3'
const radioClass = 'h-4 w-4 cursor-pointer accent-brand'
const checkboxClass = 'h-5 w-5 cursor-pointer rounded accent-brand'
const selectClass = 'w-full rounded-lg border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard focus:border-brand focus:ring-2 focus:ring-brand/15'

// Theme display label
const themeLabel = computed(() => {
  const option = THEME_OPTIONS.find(t => t.id === themeStore.theme)
  return option?.label || themeStore.theme
})

// Display labels for values
const teammateNameDisplayLabel = computed(() => {
  const labels: Record<string, string> = {
    username: 'Show username',
    nickname: 'Show nickname',
    full_name: 'Show full name'
  }
  return labels[preferencesStore.preferences?.teammate_name_display || 'username'] || 'Show username'
})

const timezoneLabel = computed(() => {
  const tz = preferencesStore.preferences?.timezone
  if (!tz || tz === 'auto') return 'Auto'
  return tz
})

const messageDisplayLabel = computed(() => {
  return preferencesStore.preferences?.message_display === 'compact' ? 'Compact' : 'Standard'
})

const channelDisplayLabel = computed(() => {
  return preferencesStore.preferences?.channel_display_mode === 'centered' ? 'Centered' : 'Full width'
})

const languageLabel = computed(() => {
  const labels: Record<string, string> = {
    en: 'English',
    es: 'Español',
    fr: 'Français',
    de: 'Deutsch',
    ja: '日本語',
    ko: '한국어',
    'pt-BR': 'Português (Brasil)',
    ru: 'Русский',
    'zh-CN': '中文 (简体)',
    'zh-TW': '中文 (繁體)'
  }
  return labels[preferencesStore.preferences?.language || 'en'] || 'English'
})

const fontSizeLabel = computed(() => {
  const size = themeStore.chatFontSize
  if (size <= 13) return 'Small'
  if (size <= 14) return 'Medium'
  return 'Large'
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

  localCollapsedReplyThreads.value = prefs.collapsed_reply_threads ?? false
  localUseMilitaryTime.value = prefs.use_military_time ?? false
  localTeammateNameDisplay.value = prefs.teammate_name_display || 'username'
  localAvailabilityVisible.value = prefs.availability_status_visible ?? true
  localShowLastActive.value = prefs.show_last_active_time ?? true
  const tz = prefs.timezone
  if (tz && tz !== 'auto') {
    localTimezoneMode.value = 'manual'
    localTimezone.value = tz
  } else {
    localTimezoneMode.value = 'auto'
    localTimezone.value = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
  }
  localLinkPreviews.value = prefs.link_previews_enabled ?? true
  localImagePreviews.value = prefs.image_previews_enabled ?? true
  localMessageDisplay.value = prefs.message_display === 'compact' ? 'compact' : 'standard'
  localClickToReply.value = prefs.click_to_reply ?? true
  localChannelDisplay.value = prefs.channel_display_mode === 'centered' ? 'centered' : 'full'
  localQuickReactions.value = prefs.quick_reactions_enabled ?? true
  localRenderEmoticons.value = prefs.emoji_picker_enabled ?? true
  localLanguage.value = prefs.language || 'en'
  
  // Sync font size from theme store
  const currentSize = themeStore.chatFontSize
  if (currentSize <= 13) localFontSize.value = 13
  else if (currentSize >= 16) localFontSize.value = 16
  else localFontSize.value = 14
}

function expandRow(rowId: string) {
  if (expandedRow.value === rowId) {
    return
  }
  
  // Initialize edit state when expanding
  if (rowId === 'theme') {
    editingTheme.value = themeStore.theme
  }
  syncLocalState()
  
  expandedRow.value = rowId
}

async function handleSaveTheme(theme: Theme) {
  savingTheme.value = true
  try {
    themeStore.setTheme(theme)
    expandedRow.value = null
  } finally {
    savingTheme.value = false
  }
}

function handleCancelTheme() {
  editingTheme.value = themeStore.theme
  expandedRow.value = null
}

// Generic save handler for preference rows
async function savePreference(_rowId: string, updates: Record<string, unknown>) {
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

// Timezone options
const commonTimezones = [
  'UTC',
  'America/New_York',
  'America/Chicago',
  'America/Denver',
  'America/Los_Angeles',
  'America/Toronto',
  'America/Vancouver',
  'America/Mexico_City',
  'America/Sao_Paulo',
  'Europe/London',
  'Europe/Paris',
  'Europe/Berlin',
  'Europe/Madrid',
  'Europe/Rome',
  'Europe/Amsterdam',
  'Europe/Moscow',
  'Asia/Tokyo',
  'Asia/Shanghai',
  'Asia/Hong_Kong',
  'Asia/Singapore',
  'Asia/Seoul',
  'Asia/Mumbai',
  'Asia/Dubai',
  'Australia/Sydney',
  'Australia/Melbourne',
  'Pacific/Auckland',
]
</script>

<template>
  <div class="space-y-1">
    <!-- 1. Theme Row -->
    <div v-if="expandedRow !== 'theme'">
      <SettingItemMin
        label="Theme"
        :value="themeLabel"
        description="Choose a color theme for the application"
        @click="expandRow('theme')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Theme"
      description="Select a premade theme tuned for contrast and readability"
      :loading="savingTheme"
      @save="() => {}"
      @cancel="handleCancelTheme"
    >
      <ThemeEditor
        v-model="editingTheme"
        @save="handleSaveTheme"
        @cancel="handleCancelTheme"
      />
    </SettingItemMax>

    <!-- 2. Font Size -->
    <div v-if="expandedRow !== 'font_size'">
      <SettingItemMin
        label="Font Size"
        :value="fontSizeLabel"
        description="Adjust the text size for better readability"
        @click="expandRow('font_size')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Font Size"
      description="Choose your preferred text size"
      :loading="saving"
      @save="themeStore.setChatFontSize(localFontSize); expandedRow = null"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localFontSize"
            :value="13"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Small</div>
            <div :class="optionDescriptionClass" style="font-size: 13px;">Compact text for more content on screen</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localFontSize"
            :value="14"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Medium</div>
            <div :class="optionDescriptionClass" style="font-size: 14px;">Standard text size</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localFontSize"
            :value="16"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Large</div>
            <div :class="optionDescriptionClass" style="font-size: 16px;">Larger text for improved readability</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 3. Threaded Discussions -->
    <div v-if="expandedRow !== 'threaded_discussions'">
      <SettingItemMin
        label="Threaded Discussions"
        :value="localCollapsedReplyThreads ? 'Collapsed' : 'Expanded'"
        description="Display replies in threads"
        @click="expandRow('threaded_discussions')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Threaded Discussions"
      description="Choose how to display thread replies"
      :loading="saving"
      @save="savePreference('threaded_discussions', { collapsed_reply_threads: localCollapsedReplyThreads })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localCollapsedReplyThreads"
            :value="false"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Expanded</div>
            <div :class="optionDescriptionClass">Show all replies in the channel</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localCollapsedReplyThreads"
            :value="true"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Collapsed</div>
            <div :class="optionDescriptionClass">Show only the number of replies in the channel</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 3. Clock Display -->
    <div v-if="expandedRow !== 'clock_display'">
      <SettingItemMin
        label="Clock Display"
        :value="localUseMilitaryTime ? '24-hour clock' : '12-hour clock'"
        description="Select your preferred clock format"
        @click="expandRow('clock_display')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Clock Display"
      description="Choose your preferred time format"
      :loading="saving"
      @save="savePreference('clock_display', { use_military_time: localUseMilitaryTime })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localUseMilitaryTime"
            :value="false"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">12-hour clock</div>
            <div :class="optionDescriptionClass">Example: 4:00 PM</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localUseMilitaryTime"
            :value="true"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">24-hour clock</div>
            <div :class="optionDescriptionClass">Example: 16:00</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 4. Teammate Name Display -->
    <div v-if="expandedRow !== 'teammate_name'">
      <SettingItemMin
        label="Teammate Name Display"
        :value="teammateNameDisplayLabel"
        description="Select how teammate names are displayed"
        @click="expandRow('teammate_name')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Teammate Name Display"
      description="Choose how to display names for teammates"
      :loading="saving"
      @save="savePreference('teammate_name', { teammate_name_display: localTeammateNameDisplay })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localTeammateNameDisplay"
            value="username"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Show username</div>
            <div :class="optionDescriptionClass">@username</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localTeammateNameDisplay"
            value="nickname"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Show nickname</div>
            <div :class="optionDescriptionClass">If set, otherwise username</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localTeammateNameDisplay"
            value="full_name"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Show full name</div>
            <div :class="optionDescriptionClass">First and last name, if set</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 5. Online Availability Badges -->
    <div v-if="expandedRow !== 'availability_badges'">
      <SettingItemMin
        label="Online Availability Badges"
        :value="localAvailabilityVisible ? 'Show' : 'Hide'"
        description="Show online availability badges on profile images"
        @click="expandRow('availability_badges')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Online Availability Badges"
      description="Control visibility of online status indicators"
      :loading="saving"
      @save="savePreference('availability_badges', { availability_status_visible: localAvailabilityVisible })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localAvailabilityVisible"
            :value="true"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Show</div>
            <div :class="optionDescriptionClass">Display online status on profile images</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localAvailabilityVisible"
            :value="false"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Hide</div>
            <div :class="optionDescriptionClass">Do not show online status indicators</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 6. Share Last Active Time -->
    <div v-if="expandedRow !== 'last_active'">
      <SettingItemMin
        label="Share Last Active Time"
        :value="localShowLastActive ? 'On' : 'Off'"
        description="Allow teammates to see when you were last active"
        @click="expandRow('last_active')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Share Last Active Time"
      description="Control whether others can see your last activity"
      :loading="saving"
      @save="savePreference('last_active', { show_last_active_time: localShowLastActive })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionToggleCardClass">
          <div>
            <div :class="optionTitleClass">Share last active time</div>
            <div :class="optionDescriptionClass">Teammates can see when you were last online</div>
          </div>
          <input
            type="checkbox"
            v-model="localShowLastActive"
            :class="checkboxClass"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 7. Timezone -->
    <div v-if="expandedRow !== 'timezone'">
      <SettingItemMin
        label="Timezone"
        :value="timezoneLabel"
        description="Select your timezone"
        @click="expandRow('timezone')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Timezone"
      description="Set your timezone for accurate time display"
      :loading="saving"
      @save="savePreference('timezone', { timezone: localTimezoneMode === 'auto' ? 'auto' : localTimezone })"
      @cancel="cancelEdit"
    >
      <div class="space-y-4">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localTimezoneMode"
            value="auto"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Auto</div>
            <div :class="optionDescriptionClass">Use your browser's timezone</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localTimezoneMode"
            value="manual"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Manual</div>
            <div :class="optionDescriptionClass">Select a specific timezone</div>
          </div>
        </label>
        
        <div v-if="localTimezoneMode === 'manual'" class="mt-3">
          <select
            v-model="localTimezone"
            :class="selectClass"
          >
            <option v-for="tz in commonTimezones" :key="tz" :value="tz">{{ tz }}</option>
          </select>
        </div>
      </div>
    </SettingItemMax>

    <!-- 8. Link Previews -->
    <div v-if="expandedRow !== 'link_previews'">
      <SettingItemMin
        label="Link Previews"
        :value="localLinkPreviews ? 'On' : 'Off'"
        description="Show previews for links in messages"
        @click="expandRow('link_previews')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Link Previews"
      description="Control link preview generation"
      :loading="saving"
      @save="savePreference('link_previews', { link_previews_enabled: localLinkPreviews })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionToggleCardClass">
          <div>
            <div :class="optionTitleClass">Show link previews</div>
            <div :class="optionDescriptionClass">Display previews when links are posted</div>
          </div>
          <input
            type="checkbox"
            v-model="localLinkPreviews"
            :class="checkboxClass"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 9. Image Previews -->
    <div v-if="expandedRow !== 'image_previews'">
      <SettingItemMin
        label="Image Previews"
        :value="localImagePreviews ? 'On' : 'Off'"
        description="Show previews for images in messages"
        @click="expandRow('image_previews')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Image Previews"
      description="Control image preview display"
      :loading="saving"
      @save="savePreference('image_previews', { image_previews_enabled: localImagePreviews })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionToggleCardClass">
          <div>
            <div :class="optionTitleClass">Show image previews</div>
            <div :class="optionDescriptionClass">Display image previews in messages</div>
          </div>
          <input
            type="checkbox"
            v-model="localImagePreviews"
            :class="checkboxClass"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 10. Message Display -->
    <div v-if="expandedRow !== 'message_display'">
      <SettingItemMin
        label="Message Display"
        :value="messageDisplayLabel"
        description="Select your message display mode"
        @click="expandRow('message_display')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Message Display"
      description="Choose how messages appear in channels"
      :loading="saving"
      @save="savePreference('message_display', { message_display: localMessageDisplay })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localMessageDisplay"
            value="standard"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Standard</div>
            <div :class="optionDescriptionClass">Full message display with avatars</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localMessageDisplay"
            value="compact"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Compact</div>
            <div :class="optionDescriptionClass">Condensed view for more messages on screen</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 11. Click to Open Threads -->
    <div v-if="expandedRow !== 'click_to_reply'">
      <SettingItemMin
        label="Click to Open Threads"
        :value="localClickToReply ? 'On' : 'Off'"
        description="Click anywhere on a message to open the reply thread"
        @click="expandRow('click_to_reply')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Click to Open Threads"
      description="Control thread opening behavior"
      :loading="saving"
      @save="savePreference('click_to_reply', { click_to_reply: localClickToReply })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionToggleCardClass">
          <div>
            <div :class="optionTitleClass">Click to open threads</div>
            <div :class="optionDescriptionClass">Click on any message to view its thread</div>
          </div>
          <input
            type="checkbox"
            v-model="localClickToReply"
            :class="checkboxClass"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 12. Channel Display -->
    <div v-if="expandedRow !== 'channel_display'">
      <SettingItemMin
        label="Channel Display"
        :value="channelDisplayLabel"
        description="Select your channel display mode"
        @click="expandRow('channel_display')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Channel Display"
      description="Choose how channel content is displayed"
      :loading="saving"
      @save="savePreference('channel_display', { channel_display_mode: localChannelDisplay })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localChannelDisplay"
            value="full"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Full width</div>
            <div :class="optionDescriptionClass">Use the full width of the window</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input
            type="radio"
            v-model="localChannelDisplay"
            value="centered"
            :class="radioClass"
          />
          <div class="flex-1">
            <div :class="optionTitleClass">Centered</div>
            <div :class="optionDescriptionClass">Center content with fixed width</div>
          </div>
        </label>
      </div>
    </SettingItemMax>

    <!-- 13. Quick Reactions -->
    <div v-if="expandedRow !== 'quick_reactions'">
      <SettingItemMin
        label="Quick Reactions"
        :value="localQuickReactions ? 'On' : 'Off'"
        description="Show quick reaction buttons on messages"
        @click="expandRow('quick_reactions')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Quick Reactions"
      description="Control quick reaction buttons"
      :loading="saving"
      @save="savePreference('quick_reactions', { quick_reactions_enabled: localQuickReactions })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionToggleCardClass">
          <div>
            <div :class="optionTitleClass">Show quick reactions</div>
            <div :class="optionDescriptionClass">Display emoji reaction buttons on hover</div>
          </div>
          <input
            type="checkbox"
            v-model="localQuickReactions"
            :class="checkboxClass"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 14. Render Emoticons -->
    <div v-if="expandedRow !== 'render_emoticons'">
      <SettingItemMin
        label="Render Emoticons"
        :value="localRenderEmoticons ? 'On' : 'Off'"
        description="Convert text emoticons to emoji"
        @click="expandRow('render_emoticons')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Render Emoticons"
      description="Convert text emoticons like :) to emoji"
      :loading="saving"
      @save="savePreference('render_emoticons', { emoji_picker_enabled: localRenderEmoticons })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <label :class="optionToggleCardClass">
          <div>
            <div :class="optionTitleClass">Render emoticons</div>
            <div :class="optionDescriptionClass">Convert :) to 😊 and other text emoticons</div>
          </div>
          <input
            type="checkbox"
            v-model="localRenderEmoticons"
            :class="checkboxClass"
          />
        </label>
      </div>
    </SettingItemMax>

    <!-- 15. Language -->
    <div v-if="expandedRow !== 'language'">
      <SettingItemMin
        label="Language"
        :value="languageLabel"
        description="Select your display language"
        @click="expandRow('language')"
      />
    </div>
    
    <SettingItemMax
      v-else
      label="Language"
      description="Choose your display language"
      :loading="saving"
      @save="savePreference('language', { language: localLanguage })"
      @cancel="cancelEdit"
    >
      <div class="space-y-3">
        <select
          v-model="localLanguage"
          :class="selectClass"
        >
          <option value="en">English</option>
          <option value="es">Español</option>
          <option value="fr">Français</option>
          <option value="de">Deutsch</option>
          <option value="ja">日本語</option>
          <option value="ko">한국어</option>
          <option value="pt-BR">Português (Brasil)</option>
          <option value="ru">Русский</option>
          <option value="zh-CN">中文 (简体)</option>
          <option value="zh-TW">中文 (繁體)</option>
        </select>
      </div>
    </SettingItemMax>
  </div>
</template>
