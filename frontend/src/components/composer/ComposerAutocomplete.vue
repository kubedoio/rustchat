<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import MentionAutocomplete from './MentionAutocomplete.vue'
import EmojiAutocomplete from './autocomplete/EmojiAutocomplete.vue'
import ChannelAutocomplete from './autocomplete/ChannelAutocomplete.vue'
import CommandAutocomplete from './autocomplete/CommandAutocomplete.vue'
import { useTeamStore } from '@/features/teams/stores/teamStore'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { searchEmojis } from '../../utils/emoji'

const props = defineProps<{
  content: string
}>()

const emit = defineEmits<{
  'update:content': [value: string]
  'focus-cursor': [pos: number]
  'save-draft': []
  'auto-resize': []
}>()

const teamStore = useTeamStore()
const channelStore = useChannelStore()

const showMentionMenu = ref(false)
const showEmojiAutocomplete = ref(false)
const showChannelAutocomplete = ref(false)
const showCommandAutocomplete = ref(false)
const mentionQuery = ref('')
const emojiQuery = ref('')
const channelQuery = ref('')
const commandQuery = ref('')
const autocompleteStartPos = ref(0)
const lastCursorPos = ref(0)

const commandEntries = ['call start', 'call join', 'call leave', 'call end']

const hasMentionSuggestions = computed(() => {
  if (!showMentionMenu.value) return false
  const query = mentionQuery.value.toLowerCase()
  return teamStore.members.some((member) => member.username.toLowerCase().includes(query))
})

const hasEmojiSuggestions = computed(() => {
  if (!showEmojiAutocomplete.value || !emojiQuery.value) return false
  return searchEmojis(emojiQuery.value, 1).length > 0
})

const hasChannelSuggestions = computed(() => {
  if (!showChannelAutocomplete.value || !channelQuery.value) return false
  const query = channelQuery.value.toLowerCase()
  return channelStore.channels.some((channel) => {
    const channelName = channel.name?.toLowerCase() ?? ''
    const displayName = channel.display_name?.toLowerCase() ?? ''
    return channelName.includes(query) || displayName.includes(query)
  })
})

const hasCommandSuggestions = computed(() => {
  if (!showCommandAutocomplete.value) return false
  const query = commandQuery.value.trim().toLowerCase().replace(/^\^k\s*/, '')
  if (!query) return commandEntries.length > 0
  return commandEntries.some((command) => command.toLowerCase().startsWith(query))
})

const commandRef = ref<InstanceType<typeof CommandAutocomplete> | null>(null)
const emojiRef = ref<InstanceType<typeof EmojiAutocomplete> | null>(null)
const channelRef = ref<InstanceType<typeof ChannelAutocomplete> | null>(null)

function selectPrevious() {
  if (showCommandAutocomplete.value) commandRef.value?.selectPrevious()
  else if (showEmojiAutocomplete.value) emojiRef.value?.selectPrevious()
  else if (showChannelAutocomplete.value) channelRef.value?.selectPrevious()
}

function selectNext() {
  if (showCommandAutocomplete.value) commandRef.value?.selectNext()
  else if (showEmojiAutocomplete.value) emojiRef.value?.selectNext()
  else if (showChannelAutocomplete.value) channelRef.value?.selectNext()
}

function selectCurrent() {
  if (showCommandAutocomplete.value) commandRef.value?.selectCurrent()
  else if (showEmojiAutocomplete.value) emojiRef.value?.selectCurrent()
  else if (showChannelAutocomplete.value) channelRef.value?.selectCurrent()
}

function onInput(newContent: string, cursorPos: number) {
  lastCursorPos.value = cursorPos
  const textBefore = newContent.substring(0, cursorPos)

  const commandPrefixMatch = !textBefore.includes('\n') ? textBefore.match(/^\^k\s*(.*)$/i) : null
  if (commandPrefixMatch) {
    commandQuery.value = commandPrefixMatch[1] ?? ''
    autocompleteStartPos.value = 0
    showCommandAutocomplete.value = true
    showMentionMenu.value = false
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
    return
  }

  const mentionMatch = textBefore.match(/@([^\s@]*)$/)
  if (mentionMatch) {
    mentionQuery.value = mentionMatch[1] ?? ''
    autocompleteStartPos.value = cursorPos - mentionMatch[0].length
    showMentionMenu.value = true
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
    showCommandAutocomplete.value = false
    if (teamStore.members.length === 0 && teamStore.currentTeamId) {
      teamStore.fetchMembers(teamStore.currentTeamId as string)
    }
    return
  }

  const emojiMatch = textBefore.match(/:([^\s:]*)$/)
  const emojiToken = emojiMatch?.[1] ?? ''
  if (emojiMatch && emojiToken.length > 0) {
    emojiQuery.value = emojiToken
    autocompleteStartPos.value = cursorPos - emojiMatch[0].length
    showEmojiAutocomplete.value = true
    showMentionMenu.value = false
    showChannelAutocomplete.value = false
    showCommandAutocomplete.value = false
    return
  }

  const channelMatch = textBefore.match(/~([^\s~]*)$/)
  if (channelMatch) {
    channelQuery.value = channelMatch[1] ?? ''
    autocompleteStartPos.value = cursorPos - channelMatch[0].length
    showChannelAutocomplete.value = true
    showMentionMenu.value = false
    showEmojiAutocomplete.value = false
    showCommandAutocomplete.value = false
    return
  }

  showMentionMenu.value = false
  showEmojiAutocomplete.value = false
  showChannelAutocomplete.value = false
  showCommandAutocomplete.value = false
}

function openCommandMenu(currentContent: string): string {
  const COMMAND_PREFIX = '^k'
  const existing = currentContent.trim()
  let newContent = existing
  if (!existing.toLowerCase().startsWith(COMMAND_PREFIX)) {
    newContent = existing ? `${COMMAND_PREFIX} ${existing}` : `${COMMAND_PREFIX} `
  }
  commandQuery.value = newContent.replace(/^\^k\s*/i, '')
  showCommandAutocomplete.value = true
  showMentionMenu.value = false
  showEmojiAutocomplete.value = false
  showChannelAutocomplete.value = false
  lastCursorPos.value = newContent.length
  return newContent
}

function reset() {
  showMentionMenu.value = false
  showEmojiAutocomplete.value = false
  showChannelAutocomplete.value = false
  showCommandAutocomplete.value = false
  mentionQuery.value = ''
  emojiQuery.value = ''
  channelQuery.value = ''
  commandQuery.value = ''
}

function handleMentionSelect(username: string) {
  const newContent =
    props.content.substring(0, autocompleteStartPos.value) +
    `@${username} ` +
    props.content.substring(lastCursorPos.value)
  emit('update:content', newContent)
  emit('focus-cursor', autocompleteStartPos.value + username.length + 2)
  emit('save-draft')
  emit('auto-resize')
  showMentionMenu.value = false
}

function handleEmojiAutocompleteSelect(emojiName: string) {
  const newContent =
    props.content.substring(0, autocompleteStartPos.value) +
    `:${emojiName}: ` +
    props.content.substring(lastCursorPos.value)
  emit('update:content', newContent)
  emit('focus-cursor', autocompleteStartPos.value + emojiName.length + 3)
  emit('save-draft')
  emit('auto-resize')
  showEmojiAutocomplete.value = false
}

function handleChannelAutocompleteSelect(channelName: string) {
  const newContent =
    props.content.substring(0, autocompleteStartPos.value) +
    `~${channelName} ` +
    props.content.substring(lastCursorPos.value)
  emit('update:content', newContent)
  emit('focus-cursor', autocompleteStartPos.value + channelName.length + 2)
  emit('save-draft')
  emit('auto-resize')
  showChannelAutocomplete.value = false
}

function handleCommandAutocompleteSelect(command: string) {
  const suffix = props.content.substring(lastCursorPos.value).replace(/^\s+/, '')
  const prefix = `^k ${command} `
  const newContent = suffix.length > 0 ? `${prefix}${suffix}` : prefix
  emit('update:content', newContent)
  emit('focus-cursor', prefix.length)
  emit('save-draft')
  emit('auto-resize')
  showCommandAutocomplete.value = false
}

defineExpose({
  onInput,
  openCommandMenu,
  reset,
  showMentionMenu,
  showEmojiAutocomplete,
  showChannelAutocomplete,
  showCommandAutocomplete,
  hasMentionSuggestions,
  hasEmojiSuggestions,
  hasChannelSuggestions,
  hasCommandSuggestions,
  selectPrevious,
  selectNext,
  selectCurrent,
})
</script>

<template>
  <MentionAutocomplete
    :show="showMentionMenu"
    :query="mentionQuery"
    @select="handleMentionSelect"
    @close="showMentionMenu = false"
  />
  <CommandAutocomplete
    ref="commandRef"
    :show="showCommandAutocomplete"
    :query="commandQuery"
    @select="handleCommandAutocompleteSelect"
    @close="showCommandAutocomplete = false"
  />
  <EmojiAutocomplete
    ref="emojiRef"
    :show="showEmojiAutocomplete"
    :query="emojiQuery"
    @select="handleEmojiAutocompleteSelect"
    @close="showEmojiAutocomplete = false"
  />
  <ChannelAutocomplete
    ref="channelRef"
    :show="showChannelAutocomplete"
    :query="channelQuery"
    @select="handleChannelAutocompleteSelect"
    @close="showChannelAutocomplete = false"
  />
</template>
