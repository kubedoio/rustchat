<script setup lang="ts">
import { ref, computed, onMounted, reactive } from 'vue'
import type { Component, Ref } from 'vue'
import SettingItemMin from '../SettingItemMin.vue'
import SettingItemMax from '../SettingItemMax.vue'
import ThemeEditor from './ThemeEditor.vue'
import RadioSettingEditor from './RadioSettingEditor.vue'
import ToggleSettingEditor from './ToggleSettingEditor.vue'
import SelectSettingEditor from './SelectSettingEditor.vue'
import { useThemeStore, THEME_OPTIONS, type Theme } from '../../../features/theme/stores/themeStore'
import { usePreferencesStore } from '../../../features/preferences/stores/preferencesStore'

const themeStore = useThemeStore()
const preferencesStore = usePreferencesStore()

const expandedRow = ref<string | null>(null)
const editingTheme = ref<Theme>(themeStore.theme)
const savingTheme = ref(false)
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

const optionCardClass =
  'flex items-center gap-3 rounded-lg border border-border-1 bg-bg-surface-1 p-3 text-text-1 transition-standard hover:bg-bg-surface-2'
const optionTitleClass = 'text-sm font-medium text-text-1'
const optionDescriptionClass = 'text-xs text-text-3'
const radioClass = 'h-4 w-4 cursor-pointer accent-brand'
const selectClass =
  'w-full rounded-lg border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard focus:border-brand focus:ring-2 focus:ring-brand/15'

const themeLabel = computed(
  () => THEME_OPTIONS.find(t => t.id === themeStore.theme)?.label || themeStore.theme
)

const NAME_LABELS: Record<string, string> = {
  username: 'Show username',
  nickname: 'Show nickname',
  full_name: 'Show full name',
}
const teammateNameDisplayLabel = computed(
  () =>
    NAME_LABELS[preferencesStore.preferences?.teammate_name_display || 'username'] ||
    'Show username'
)

const timezoneLabel = computed(() => {
  const tz = preferencesStore.preferences?.timezone
  return !tz || tz === 'auto' ? 'Auto' : tz
})

const messageDisplayLabel = computed(() =>
  preferencesStore.preferences?.message_display === 'compact' ? 'Compact' : 'Standard'
)
const channelDisplayLabel = computed(() =>
  preferencesStore.preferences?.channel_display_mode === 'centered' ? 'Centered' : 'Full width'
)

const LANGUAGE_LABELS: Record<string, string> = {
  en: 'English',
  es: 'Español',
  fr: 'Français',
  de: 'Deutsch',
  ja: '日本語',
  ko: '한국어',
  'pt-BR': 'Português (Brasil)',
  ru: 'Русский',
  'zh-CN': '中文 (简体)',
  'zh-TW': '中文 (繁體)',
}
const languageLabel = computed(
  () => LANGUAGE_LABELS[preferencesStore.preferences?.language || 'en'] || 'English'
)

const fontSizeLabel = computed(() => {
  const size = themeStore.chatFontSize
  return size <= 13 ? 'Small' : size <= 14 ? 'Medium' : 'Large'
})

onMounted(() => {
  preferencesStore.fetchPreferences().then(() => syncLocalState())
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
  const currentSize = themeStore.chatFontSize
  if (currentSize <= 13) localFontSize.value = 13
  else if (currentSize >= 16) localFontSize.value = 16
  else localFontSize.value = 14
}

function expandRow(rowId: string) {
  if (expandedRow.value === rowId) return
  if (rowId === 'theme') editingTheme.value = themeStore.theme
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

interface DisplaySetting {
  id: string
  label: string
  minDescription: string
  maxDescription: string
  minValue: () => string
  editor: Component
  editorProps: Record<string, unknown>
  model: Ref<any>
  save: () => void | Promise<void>
}

function radioSetting(
  id: string,
  label: string,
  minDesc: string,
  maxDesc: string,
  minValue: () => string,
  model: Ref<any>,
  options: unknown[],
  save: () => void | Promise<void>
): DisplaySetting {
  return {
    id,
    label,
    minDescription: minDesc,
    maxDescription: maxDesc,
    minValue,
    editor: RadioSettingEditor,
    editorProps: { options },
    model,
    save,
  }
}

function toggleSetting(
  id: string,
  label: string,
  minDesc: string,
  maxDesc: string,
  minValue: () => string,
  model: Ref<boolean>,
  title: string,
  description: string,
  save: () => void | Promise<void>
): DisplaySetting {
  return {
    id,
    label,
    minDescription: minDesc,
    maxDescription: maxDesc,
    minValue,
    editor: ToggleSettingEditor,
    editorProps: { title, description },
    model,
    save,
  }
}

function selectSetting(
  id: string,
  label: string,
  minDesc: string,
  maxDesc: string,
  minValue: () => string,
  model: Ref<string>,
  options: unknown[],
  save: () => void | Promise<void>
): DisplaySetting {
  return {
    id,
    label,
    minDescription: minDesc,
    maxDescription: maxDesc,
    minValue,
    editor: SelectSettingEditor,
    editorProps: { options },
    model,
    save,
  }
}

const displaySettings = reactive<DisplaySetting[]>([
  radioSetting(
    'font_size',
    'Font Size',
    'Adjust the text size for better readability',
    'Choose your preferred text size',
    () => fontSizeLabel.value,
    localFontSize,
    [
      {
        value: 13,
        label: 'Small',
        description: 'Compact text for more content on screen',
        style: { fontSize: '13px' },
      },
      {
        value: 14,
        label: 'Medium',
        description: 'Standard text size',
        style: { fontSize: '14px' },
      },
      {
        value: 16,
        label: 'Large',
        description: 'Larger text for improved readability',
        style: { fontSize: '16px' },
      },
    ],
    () => {
      themeStore.setChatFontSize(localFontSize.value)
      expandedRow.value = null
    }
  ),
  radioSetting(
    'threaded_discussions',
    'Threaded Discussions',
    'Display replies in threads',
    'Choose how to display thread replies',
    () => (localCollapsedReplyThreads.value ? 'Collapsed' : 'Expanded'),
    localCollapsedReplyThreads,
    [
      { value: false, label: 'Expanded', description: 'Show all replies in the channel' },
      {
        value: true,
        label: 'Collapsed',
        description: 'Show only the number of replies in the channel',
      },
    ],
    () =>
      savePreference('threaded_discussions', {
        collapsed_reply_threads: localCollapsedReplyThreads.value,
      })
  ),
  radioSetting(
    'clock_display',
    'Clock Display',
    'Select your preferred clock format',
    'Choose your preferred time format',
    () => (localUseMilitaryTime.value ? '24-hour clock' : '12-hour clock'),
    localUseMilitaryTime,
    [
      { value: false, label: '12-hour clock', description: 'Example: 4:00 PM' },
      { value: true, label: '24-hour clock', description: 'Example: 16:00' },
    ],
    () => savePreference('clock_display', { use_military_time: localUseMilitaryTime.value })
  ),
  radioSetting(
    'teammate_name',
    'Teammate Name Display',
    'Select how teammate names are displayed',
    'Choose how to display names for teammates',
    () => teammateNameDisplayLabel.value,
    localTeammateNameDisplay,
    [
      { value: 'username', label: 'Show username', description: '@username' },
      { value: 'nickname', label: 'Show nickname', description: 'If set, otherwise username' },
      { value: 'full_name', label: 'Show full name', description: 'First and last name, if set' },
    ],
    () => savePreference('teammate_name', { teammate_name_display: localTeammateNameDisplay.value })
  ),
  radioSetting(
    'availability_badges',
    'Online Availability Badges',
    'Show online availability badges on profile images',
    'Control visibility of online status indicators',
    () => (localAvailabilityVisible.value ? 'Show' : 'Hide'),
    localAvailabilityVisible,
    [
      { value: true, label: 'Show', description: 'Display online status on profile images' },
      { value: false, label: 'Hide', description: 'Do not show online status indicators' },
    ],
    () =>
      savePreference('availability_badges', {
        availability_status_visible: localAvailabilityVisible.value,
      })
  ),
  toggleSetting(
    'last_active',
    'Share Last Active Time',
    'Allow teammates to see when you were last active',
    'Control whether others can see your last activity',
    () => (localShowLastActive.value ? 'On' : 'Off'),
    localShowLastActive,
    'Share last active time',
    'Teammates can see when you were last online',
    () => savePreference('last_active', { show_last_active_time: localShowLastActive.value })
  ),
  toggleSetting(
    'link_previews',
    'Link Previews',
    'Show previews for links in messages',
    'Control link preview generation',
    () => (localLinkPreviews.value ? 'On' : 'Off'),
    localLinkPreviews,
    'Show link previews',
    'Display previews when links are posted',
    () => savePreference('link_previews', { link_previews_enabled: localLinkPreviews.value })
  ),
  toggleSetting(
    'image_previews',
    'Image Previews',
    'Show previews for images in messages',
    'Control image preview display',
    () => (localImagePreviews.value ? 'On' : 'Off'),
    localImagePreviews,
    'Show image previews',
    'Display image previews in messages',
    () => savePreference('image_previews', { image_previews_enabled: localImagePreviews.value })
  ),
  radioSetting(
    'message_display',
    'Message Display',
    'Select your message display mode',
    'Choose how messages appear in channels',
    () => messageDisplayLabel.value,
    localMessageDisplay,
    [
      { value: 'standard', label: 'Standard', description: 'Full message display with avatars' },
      {
        value: 'compact',
        label: 'Compact',
        description: 'Condensed view for more messages on screen',
      },
    ],
    () => savePreference('message_display', { message_display: localMessageDisplay.value })
  ),
  toggleSetting(
    'click_to_reply',
    'Click to Open Threads',
    'Click anywhere on a message to open the reply thread',
    'Control thread opening behavior',
    () => (localClickToReply.value ? 'On' : 'Off'),
    localClickToReply,
    'Click to open threads',
    'Click on any message to view its thread',
    () => savePreference('click_to_reply', { click_to_reply: localClickToReply.value })
  ),
  radioSetting(
    'channel_display',
    'Channel Display',
    'Select your channel display mode',
    'Choose how channel content is displayed',
    () => channelDisplayLabel.value,
    localChannelDisplay,
    [
      { value: 'full', label: 'Full width', description: 'Use the full width of the window' },
      { value: 'centered', label: 'Centered', description: 'Center content with fixed width' },
    ],
    () => savePreference('channel_display', { channel_display_mode: localChannelDisplay.value })
  ),
  toggleSetting(
    'quick_reactions',
    'Quick Reactions',
    'Show quick reaction buttons on messages',
    'Control quick reaction buttons',
    () => (localQuickReactions.value ? 'On' : 'Off'),
    localQuickReactions,
    'Show quick reactions',
    'Display emoji reaction buttons on hover',
    () => savePreference('quick_reactions', { quick_reactions_enabled: localQuickReactions.value })
  ),
  toggleSetting(
    'render_emoticons',
    'Render Emoticons',
    'Convert text emoticons to emoji',
    'Convert text emoticons like :) to emoji',
    () => (localRenderEmoticons.value ? 'On' : 'Off'),
    localRenderEmoticons,
    'Render emoticons',
    'Convert :) to 😊 and other text emoticons',
    () => savePreference('render_emoticons', { emoji_picker_enabled: localRenderEmoticons.value })
  ),
  selectSetting(
    'language',
    'Language',
    'Select your display language',
    'Choose your display language',
    () => languageLabel.value,
    localLanguage,
    [
      { value: 'en', label: 'English' },
      { value: 'es', label: 'Español' },
      { value: 'fr', label: 'Français' },
      { value: 'de', label: 'Deutsch' },
      { value: 'ja', label: '日本語' },
      { value: 'ko', label: '한국어' },
      { value: 'pt-BR', label: 'Português (Brasil)' },
      { value: 'ru', label: 'Русский' },
      { value: 'zh-CN', label: '中文 (简体)' },
      { value: 'zh-TW', label: '中文 (繁體)' },
    ],
    () => savePreference('language', { language: localLanguage.value })
  ),
])

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
      <ThemeEditor v-model="editingTheme" @save="handleSaveTheme" @cancel="handleCancelTheme" />
    </SettingItemMax>
    <template v-for="setting in displaySettings" :key="setting.id">
      <div v-if="expandedRow !== setting.id">
        <SettingItemMin
          :label="setting.label"
          :value="setting.minValue()"
          :description="setting.minDescription"
          @click="expandRow(setting.id)"
        />
      </div>
      <SettingItemMax
        v-else
        :label="setting.label"
        :description="setting.maxDescription"
        :loading="saving"
        @save="setting.save"
        @cancel="cancelEdit"
      >
        <component :is="setting.editor" v-model="setting.model" v-bind="setting.editorProps" />
      </SettingItemMax>
    </template>
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
      @save="
        savePreference('timezone', {
          timezone: localTimezoneMode === 'auto' ? 'auto' : localTimezone,
        })
      "
      @cancel="cancelEdit"
    >
      <div class="space-y-4">
        <label :class="optionCardClass">
          <input type="radio" v-model="localTimezoneMode" value="auto" :class="radioClass" />
          <div class="flex-1">
            <div :class="optionTitleClass">Auto</div>
            <div :class="optionDescriptionClass">Use your browser's timezone</div>
          </div>
        </label>
        <label :class="optionCardClass">
          <input type="radio" v-model="localTimezoneMode" value="manual" :class="radioClass" />
          <div class="flex-1">
            <div :class="optionTitleClass">Manual</div>
            <div :class="optionDescriptionClass">Select a specific timezone</div>
          </div>
        </label>
        <div v-if="localTimezoneMode === 'manual'" class="mt-3">
          <select v-model="localTimezone" :class="selectClass">
            <option v-for="tz in commonTimezones" :key="tz" :value="tz">{{ tz }}</option>
          </select>
        </div>
      </div>
    </SettingItemMax>
  </div>
</template>
