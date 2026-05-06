<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { AlertTriangle, ExternalLink, Lightbulb, Pencil } from 'lucide-vue-next'
import api from '../../../api/client'
import SettingItemMax from '../SettingItemMax.vue'
import { useAuthStore } from '../../../stores/auth'
import { usePreferencesStore } from '../../../stores/preferences'
import { useToast } from '../../../composables/useToast'

const preferencesStore = usePreferencesStore()
const authStore = useAuthStore()
const toast = useToast()

const expandedRow = ref<string | null>(null)
const saving = ref(false)
const sendingTestNotification = ref(false)
const notificationPermission = ref<NotificationPermission>('default')

const HIGHLIGHT_KEYWORDS_STORAGE_KEY = 'notifications_highlight_keywords'
const AUTO_RESPONDER_ENABLED_STORAGE_KEY = 'notifications_auto_responder_enabled'
const AUTO_RESPONDER_MESSAGE_STORAGE_KEY = 'notifications_auto_responder_message'

const localNotifyDesktop = ref<'all' | 'mentions' | 'none'>('mentions')
const localNotifyPush = ref<'all' | 'mentions' | 'none'>('mentions')
const localNotifySounds = ref(true)
const localNotifyEmail = ref(true)
const localMentionKeywords = ref<string[]>([])

const localHighlightKeywords = ref<string[]>([])
const localAutoResponderEnabled = ref(false)
const localAutoResponderMessage = ref('')

const mentionKeywordsInput = ref('')
const highlightKeywordsInput = ref('')

const defaultMentionKeywords = computed(() => {
  const username = authStore.user?.username ? `@${authStore.user.username}` : '@username'
  return [username, '@channel', '@all', '@here']
})

const desktopAndMobileLabel = computed(() => {
  if (localNotifyDesktop.value === 'none' && localNotifyPush.value === 'none') {
    return 'Never'
  }

  if (localNotifyDesktop.value === 'all' || localNotifyPush.value === 'all') {
    return 'All new messages'
  }

  return 'Mentions, direct messages, and group messages'
})

const desktopNotificationSoundsLabel = computed(() => {
  if (!localNotifySounds.value) {
    return 'Off'
  }
  return '"Bing" for messages'
})

const emailNotificationsLabel = computed(() => {
  return localNotifyEmail.value ? 'On' : 'Off'
})

const keywordsTriggerLabel = computed(() => {
  const keywords = localMentionKeywords.value.length > 0 ? localMentionKeywords.value : defaultMentionKeywords.value
  return formatKeywordList(keywords)
})

const highlightKeywordsLabel = computed(() => {
  if (localHighlightKeywords.value.length === 0) {
    return 'None'
  }
  return formatKeywordList(localHighlightKeywords.value)
})

const autoResponderLabel = computed(() => {
  return localAutoResponderEnabled.value ? 'Enabled' : 'Disabled'
})

const permissionRequired = computed(() => {
  return notificationPermission.value !== 'granted'
})

const pageEyebrowClass = 'text-[11px] font-semibold uppercase tracking-[0.24em] text-brand'
const titleClass = 'text-3xl font-semibold tracking-[-0.04em] text-text-1 sm:text-[2rem]'
const summaryClass = 'mt-2 max-w-2xl text-sm text-text-3'
const linkClass = 'inline-flex items-center gap-2 rounded-full border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm font-medium text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 hover:text-text-1'
const panelClass = 'overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1'
const rowButtonClass = 'flex w-full items-start justify-between gap-4 border-b border-border-1 px-5 py-4 text-left transition-standard hover:bg-bg-surface-2'
const rowTitleClass = 'text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg'
const rowValueClass = 'mt-1 text-sm text-text-3'
const editChipClass = 'inline-flex items-center gap-1 rounded-full border border-border-1 bg-bg-surface-2 px-2.5 py-1 text-xs font-semibold text-text-2'
const permissionChipClass = 'inline-flex items-center gap-1 rounded-full border border-secondary/20 bg-secondary/10 px-2.5 py-1 text-xs font-semibold text-secondary'
const optionCardClass = 'flex items-start gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2'
const toggleCardClass = 'flex items-center justify-between rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-1'
const fieldLabelClass = 'mb-1 block text-sm font-medium text-text-1'
const fieldInputClass = 'w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15'
const helperCardClass = 'rounded-r-2 border border-secondary/20 bg-secondary/10 p-4'
const helperButtonClass = 'mt-3 rounded-r-2 bg-brand px-3 py-2 text-sm font-medium text-brand-foreground transition-standard hover:bg-brand-hover'
const ghostButtonClass = 'inline-flex items-center gap-1 rounded-r-2 border border-border-1 bg-bg-surface-1 px-4 py-2 text-sm font-medium text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 hover:text-text-1'

onMounted(async () => {
  if (typeof Notification !== 'undefined') {
    notificationPermission.value = Notification.permission
  }

  await preferencesStore.fetchPreferences()
  syncLocalState()
})

function syncLocalState() {
  const prefs = preferencesStore.preferences
  if (prefs) {
    localNotifyDesktop.value = (prefs.notify_desktop as 'all' | 'mentions' | 'none') || 'mentions'
    localNotifyPush.value = (prefs.notify_push as 'all' | 'mentions' | 'none') || 'mentions'
    localNotifySounds.value = prefs.notify_sounds !== false

    const notifyEmail = prefs.notify_email
    localNotifyEmail.value = notifyEmail === 'true' || notifyEmail === 'all' || notifyEmail === 'mentions'

    localMentionKeywords.value = prefs.mention_keywords && prefs.mention_keywords.length > 0
      ? prefs.mention_keywords
      : [...defaultMentionKeywords.value]
  }

  localHighlightKeywords.value = readJsonArray(HIGHLIGHT_KEYWORDS_STORAGE_KEY)
  localAutoResponderEnabled.value = localStorage.getItem(AUTO_RESPONDER_ENABLED_STORAGE_KEY) === 'true'
  localAutoResponderMessage.value = localStorage.getItem(AUTO_RESPONDER_MESSAGE_STORAGE_KEY) || ''

  mentionKeywordsInput.value = localMentionKeywords.value.join(', ')
  highlightKeywordsInput.value = localHighlightKeywords.value.join(', ')
}

function expandRow(rowId: string) {
  if (expandedRow.value === rowId) {
    return
  }

  syncLocalState()
  expandedRow.value = rowId
}

function cancelEdit() {
  syncLocalState()
  expandedRow.value = null
}

async function saveDesktopAndMobileSettings() {
  await savePreference({
    notify_desktop: localNotifyDesktop.value,
    notify_push: localNotifyPush.value,
  })
}

async function saveDesktopNotificationSounds() {
  await savePreference({
    notify_sounds: localNotifySounds.value,
  })
}

async function saveEmailNotifications() {
  await savePreference({
    notify_email: localNotifyEmail.value ? 'true' : 'false',
  })
}

async function saveKeywordsThatTriggerNotifications() {
  const parsed = parseKeywords(mentionKeywordsInput.value)
  localMentionKeywords.value = parsed.length > 0 ? parsed : [...defaultMentionKeywords.value]
  await savePreference({ mention_keywords: localMentionKeywords.value })
}

async function saveHighlightedKeywords() {
  localHighlightKeywords.value = parseKeywords(highlightKeywordsInput.value)
  localStorage.setItem(HIGHLIGHT_KEYWORDS_STORAGE_KEY, JSON.stringify(localHighlightKeywords.value))
  expandedRow.value = null
}

async function saveAutomaticDirectMessageReplies() {
  localStorage.setItem(AUTO_RESPONDER_ENABLED_STORAGE_KEY, String(localAutoResponderEnabled.value))
  localStorage.setItem(AUTO_RESPONDER_MESSAGE_STORAGE_KEY, localAutoResponderMessage.value.trim())
  expandedRow.value = null
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

async function requestNotificationPermission() {
  if (typeof Notification === 'undefined') {
    toast.error('Permission unavailable', 'Notifications are not supported in this browser')
    return
  }

  try {
    notificationPermission.value = await Notification.requestPermission()

    if (notificationPermission.value === 'granted') {
      toast.success('Permission granted', 'Desktop notifications are now enabled')
      return
    }

    toast.error('Permission required', 'Please enable notifications in your browser settings')
  } catch (error) {
    console.error('Failed to request notification permission', error)
    toast.error('Permission request failed', 'Could not request notification permission')
  }
}

function testNotificationSound() {
  try {
    const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)()
    const oscillator = audioContext.createOscillator()
    const gainNode = audioContext.createGain()

    oscillator.connect(gainNode)
    gainNode.connect(audioContext.destination)

    oscillator.frequency.value = 800
    oscillator.type = 'sine'
    gainNode.gain.value = 0.3

    oscillator.start()
    oscillator.stop(audioContext.currentTime + 0.12)

    toast.success('Sound test', 'Played notification sound')
  } catch (error) {
    console.error('Failed to play notification sound', error)
    toast.error('Sound test failed', 'Could not play notification sound')
  }
}

async function sendTestNotification() {
  sendingTestNotification.value = true
  try {
    const response = await api.post('/api/v4/notifications/test')
    if (response.data?.status === 'OK') {
      toast.success('Test notification sent', 'Check your devices for a test notification')
      return
    }

    toast.error('Failed to send test notification', 'Unexpected response from server')
  } catch (error) {
    console.error('Failed to send test notification', error)
    toast.error('Failed to send test notification', 'Please check your notification configuration and try again')
  } finally {
    sendingTestNotification.value = false
  }
}

function openTroubleshootingDocs() {
  window.open('https://mattermost.com/pl/troubleshoot-notifications', '_blank', 'noopener,noreferrer')
}

function parseKeywords(value: string): string[] {
  return value
    .split(',')
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
}

function readJsonArray(key: string): string[] {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) {
      return []
    }

    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) {
      return []
    }

    return parsed.filter((item) => typeof item === 'string')
  } catch {
    return []
  }
}

function formatKeywordList(keywords: string[]): string {
  return keywords.map((keyword) => `"${keyword}"`).join(', ')
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-col gap-3 px-1 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <p :class="pageEyebrowClass">Delivery Controls</p>
        <h3 :class="titleClass">Notifications</h3>
        <p :class="summaryClass">Tune desktop, push, email, and keyword behavior without losing readability across themes.</p>
      </div>
      <a
        href="https://mattermost.com/pl/about-notifications"
        target="_blank"
        rel="noopener noreferrer"
        :class="linkClass"
      >
        <Lightbulb class="h-4 w-4" />
        Learn more about notifications
      </a>
    </div>

    <div :class="panelClass">
      <div v-if="expandedRow !== 'desktop_mobile'">
        <button type="button" :class="rowButtonClass" @click="expandRow('desktop_mobile')">
          <div class="min-w-0">
            <div :class="rowTitleClass">Desktop and mobile notifications</div>
            <div :class="rowValueClass">{{ desktopAndMobileLabel }}</div>
          </div>
          <div class="mt-0.5 flex items-center gap-2">
            <span v-if="permissionRequired" :class="permissionChipClass">
              <AlertTriangle class="h-3.5 w-3.5" />
              Permission required
            </span>
            <span :class="editChipClass">
              <Pencil class="h-4 w-4" />
              Edit
            </span>
          </div>
        </button>
      </div>

      <SettingItemMax
        v-else
        label="Desktop and mobile notifications"
        description="Choose how desktop and mobile notifications are delivered"
        :loading="saving"
        @save="saveDesktopAndMobileSettings"
        @cancel="cancelEdit"
      >
        <div class="space-y-5">
          <div v-if="permissionRequired" :class="helperCardClass">
            <div class="text-sm font-semibold text-secondary">Permission required</div>
            <p class="mt-1 text-xs text-text-2">Allow browser notifications to receive desktop alerts.</p>
            <button type="button" :class="helperButtonClass" @click="requestNotificationPermission">
              Allow notifications
            </button>
          </div>

          <div>
            <div class="mb-2 text-sm font-semibold text-text-1">Desktop</div>
            <div class="space-y-2">
              <label :class="optionCardClass">
                <input v-model="localNotifyDesktop" type="radio" value="all" class="mt-0.5 h-4 w-4 accent-brand" />
                <span>For all activity</span>
              </label>
              <label :class="optionCardClass">
                <input v-model="localNotifyDesktop" type="radio" value="mentions" class="mt-0.5 h-4 w-4 accent-brand" />
                <span>For mentions and direct messages</span>
              </label>
              <label :class="optionCardClass">
                <input v-model="localNotifyDesktop" type="radio" value="none" class="mt-0.5 h-4 w-4 accent-brand" />
                <span>Never</span>
              </label>
            </div>
          </div>

          <div>
            <div class="mb-2 text-sm font-semibold text-text-1">Mobile</div>
            <div class="space-y-2">
              <label :class="optionCardClass">
                <input v-model="localNotifyPush" type="radio" value="all" class="mt-0.5 h-4 w-4 accent-brand" />
                <span>For all activity</span>
              </label>
              <label :class="optionCardClass">
                <input v-model="localNotifyPush" type="radio" value="mentions" class="mt-0.5 h-4 w-4 accent-brand" />
                <span>For mentions and direct messages</span>
              </label>
              <label :class="optionCardClass">
                <input v-model="localNotifyPush" type="radio" value="none" class="mt-0.5 h-4 w-4 accent-brand" />
                <span>Never</span>
              </label>
            </div>
          </div>
        </div>
      </SettingItemMax>

      <div v-if="expandedRow !== 'desktop_sounds'">
        <button type="button" :class="rowButtonClass" @click="expandRow('desktop_sounds')">
          <div class="min-w-0">
            <div :class="rowTitleClass">Desktop notification sounds</div>
            <div :class="rowValueClass">{{ desktopNotificationSoundsLabel }}</div>
          </div>
          <span :class="editChipClass">
            <Pencil class="h-4 w-4" />
            Edit
          </span>
        </button>
      </div>

      <SettingItemMax
        v-else
        label="Desktop notification sounds"
        description="Control desktop notification sounds"
        :loading="saving"
        @save="saveDesktopNotificationSounds"
        @cancel="cancelEdit"
      >
        <div class="space-y-3">
          <label :class="toggleCardClass">
            <span>Enable desktop notification sounds</span>
            <input v-model="localNotifySounds" type="checkbox" class="h-4 w-4 accent-brand" />
          </label>
          <button type="button" :class="ghostButtonClass" @click="testNotificationSound">
            Test sound
          </button>
        </div>
      </SettingItemMax>

      <div v-if="expandedRow !== 'email_notifications'">
        <button type="button" :class="rowButtonClass" @click="expandRow('email_notifications')">
          <div class="min-w-0">
            <div :class="rowTitleClass">Email notifications</div>
            <div :class="rowValueClass">{{ emailNotificationsLabel }}</div>
          </div>
          <span :class="editChipClass">
            <Pencil class="h-4 w-4" />
            Edit
          </span>
        </button>
      </div>

      <SettingItemMax
        v-else
        label="Email notifications"
        description="Enable or disable email notifications"
        :loading="saving"
        @save="saveEmailNotifications"
        @cancel="cancelEdit"
      >
        <label :class="toggleCardClass">
          <span>Send email notifications</span>
          <input v-model="localNotifyEmail" type="checkbox" class="h-4 w-4 accent-brand" />
        </label>
      </SettingItemMax>

      <div v-if="expandedRow !== 'trigger_keywords'">
        <button type="button" :class="rowButtonClass" @click="expandRow('trigger_keywords')">
          <div class="min-w-0">
            <div :class="rowTitleClass">Keywords that trigger notifications</div>
            <div :class="`${rowValueClass} break-words`">{{ keywordsTriggerLabel }}</div>
          </div>
          <span :class="editChipClass">
            <Pencil class="h-4 w-4" />
            Edit
          </span>
        </button>
      </div>

      <SettingItemMax
        v-else
        label="Keywords that trigger notifications"
        description="Messages containing these keywords will trigger notifications"
        :loading="saving"
        @save="saveKeywordsThatTriggerNotifications"
        @cancel="cancelEdit"
      >
        <div>
          <label :class="fieldLabelClass">Keywords (comma-separated)</label>
          <input
            v-model="mentionKeywordsInput"
            type="text"
            :class="fieldInputClass"
            placeholder="@username, @channel, @all, @here"
          />
        </div>
      </SettingItemMax>

      <div v-if="expandedRow !== 'highlight_keywords'">
        <button type="button" :class="rowButtonClass" @click="expandRow('highlight_keywords')">
          <div class="min-w-0">
            <div :class="rowTitleClass">Keywords that get highlighted (without notifications)</div>
            <div :class="`${rowValueClass} break-words`">{{ highlightKeywordsLabel }}</div>
          </div>
          <span :class="editChipClass">
            <Pencil class="h-4 w-4" />
            Edit
          </span>
        </button>
      </div>

      <SettingItemMax
        v-else
        label="Keywords that get highlighted (without notifications)"
        description="Messages containing these keywords are highlighted only"
        :loading="saving"
        @save="saveHighlightedKeywords"
        @cancel="cancelEdit"
      >
        <div>
          <label :class="fieldLabelClass">Keywords (comma-separated)</label>
          <input
            v-model="highlightKeywordsInput"
            type="text"
            :class="fieldInputClass"
            placeholder="release, incident, urgent"
          />
        </div>
      </SettingItemMax>

      <div v-if="expandedRow !== 'auto_responder'">
        <button
          type="button"
          class="flex w-full items-start justify-between gap-4 px-5 py-4 text-left transition-standard hover:bg-bg-surface-2"
          @click="expandRow('auto_responder')"
        >
          <div class="min-w-0">
            <div :class="rowTitleClass">Automatic direct message replies</div>
            <div :class="rowValueClass">{{ autoResponderLabel }}</div>
          </div>
          <span :class="editChipClass">
            <Pencil class="h-4 w-4" />
            Edit
          </span>
        </button>
      </div>

      <SettingItemMax
        v-else
        label="Automatic direct message replies"
        description="Automatically send a message when you receive a direct message"
        :loading="saving"
        @save="saveAutomaticDirectMessageReplies"
        @cancel="cancelEdit"
      >
        <div class="space-y-3">
          <label :class="toggleCardClass">
            <span>Enable automatic direct message replies</span>
            <input v-model="localAutoResponderEnabled" type="checkbox" class="h-4 w-4 accent-brand" />
          </label>
          <div>
            <label :class="fieldLabelClass">Auto-reply message</label>
            <textarea
              v-model="localAutoResponderMessage"
              rows="3"
              :class="fieldInputClass"
              placeholder="I'm away right now and will reply as soon as possible."
            />
          </div>
        </div>
      </SettingItemMax>
    </div>

    <div class="rounded-r-3 border border-border-1 bg-bg-surface-2 p-5 shadow-1">
      <div class="flex items-start gap-3">
        <Lightbulb class="mt-0.5 h-5 w-5 text-secondary" />
        <div class="flex-1">
          <h4 class="text-base font-semibold text-text-1">Troubleshooting notifications</h4>
          <p class="mt-1 text-sm text-text-2">
            Not receiving notifications? Start by sending a test notification to all your devices to check if they are working as expected. If issues persist, explore ways to solve them with troubleshooting steps.
          </p>
          <div class="mt-4 flex flex-wrap gap-2">
            <button
              type="button"
              class="rounded-r-2 bg-brand px-4 py-2 text-sm font-medium text-brand-foreground transition-standard hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-60"
              :disabled="sendingTestNotification"
              @click="sendTestNotification"
            >
              {{ sendingTestNotification ? 'Sending a test notification' : 'Send a test notification' }}
            </button>
            <button type="button" :class="ghostButtonClass" @click="openTroubleshootingDocs">
              Troubleshooting docs
              <ExternalLink class="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
