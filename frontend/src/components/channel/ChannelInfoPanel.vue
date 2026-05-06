<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { 
  Hash, 
  Lock, 
  Users, 
  MessageSquare, 
  Calendar, 
  Bell, 
  Star, 
  LogOut,
  Edit3,
  Copy,
  Mail,
  Briefcase
} from 'lucide-vue-next'
import { format } from 'date-fns'
import { useChannelStore } from '../../stores/channels'
import { useAuthStore } from '../../stores/auth'
import { useMessageStore } from '../../stores/messages'
import { useUIStore } from '../../stores/ui'
import { useTeamStore } from '../../stores/teams'
import api from '../../api/client'
import RcAvatar from '../ui/RcAvatar.vue'
import { getPresencePresentation } from '../../features/presence/presencePresentation'
import { useUserSummary } from '../../composables/useUserSummary'
import { getDirectMessageCounterpartyId } from '../../utils/directMessage'
import { useChannelManagementPermission } from '../../features/permissions/capabilities'

const props = defineProps<{
  channelId: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'openSettings'): void
}>()

const channelStore = useChannelStore()
const authStore = useAuthStore()
const messageStore = useMessageStore()
const uiStore = useUIStore()
const teamStore = useTeamStore()

const loading = ref(false)
const memberCount = ref(0)
const messageCount = ref(0)
const showCopiedToast = ref(false)
const isFavorite = ref(false)
const isMuted = ref(false)

const channel = computed(() => 
  channelStore.channels.find(c => c.id === props.channelId)
)

const isCreator = computed(() => 
  channel.value?.creator_id === authStore.user?.id
)

const channelTypeValue = computed(() => channel.value?.channel_type || (channel.value as any)?.type);

const directContactId = computed(() => {
  if (channelTypeValue.value !== 'direct') {
    return null
  }
  return getDirectMessageCounterpartyId(channel.value!.name, authStore.user?.id)
})

const { userSummary: directContact, isLoading: directContactLoading } = useUserSummary(() => directContactId.value)

const directContactMember = computed(() => {
  if (!directContactId.value) {
    return null
  }
  return teamStore.members.find((member) => member.user_id === directContactId.value) || null
})

const directContactName = computed(() => {
  return (
    directContact.value?.displayName ||
    directContactMember.value?.display_name ||
    directContact.value?.username ||
    directContactMember.value?.username ||
    channel.value?.display_name ||
    'Direct Message'
  )
})

const directContactPresenceMeta = computed(() => getPresencePresentation(directContact.value?.presence || directContactMember.value?.presence))

const showDirectMessageProfile = computed(() => channelTypeValue.value === 'direct' && !!directContactId.value)
const { canManageChannel: canManageCurrentChannel } = useChannelManagementPermission(
  () => props.channelId,
  () => channel.value?.creator_id ?? null,
)

const channelIcon = computed(() => {
  if (!channel.value) return Hash
  return channelTypeValue.value === 'private' ? Lock : Hash
})

const channelTypeLabel = computed(() => {
  if (!channel.value) return 'Channel'
  const types: Record<string, string> = {
    public: 'Public Channel',
    private: 'Private Channel',
    direct: 'Direct Message',
    group: 'Group Message'
  }
  return types[channelTypeValue.value] || 'Channel'
})

async function loadStats() {
  if (!props.channelId) return
  
  loading.value = true
  try {
    // Get member count from stats endpoint
    const statsResponse = await api.get(`/channels/${props.channelId}/stats`)
    memberCount.value = statsResponse.data.member_count || 0
    
    // Get message count from message store (estimate from current loaded messages)
    const messages = messageStore.messagesByChannel[props.channelId]
    messageCount.value = messages?.length || 0
    
    // Check if channel is favorited (would need preferences API)
    // For now, we'll use localStorage as a simple implementation
    const favorites = JSON.parse(localStorage.getItem('favorite_channels') || '[]')
    isFavorite.value = favorites.includes(props.channelId)
    
    // Check if muted
    const muted = JSON.parse(localStorage.getItem('muted_channels') || '[]')
    isMuted.value = muted.includes(props.channelId)
  } catch (e) {
    console.error('Failed to load channel stats:', e)
  } finally {
    loading.value = false
  }
}

function toggleFavorite() {
  const favorites = JSON.parse(localStorage.getItem('favorite_channels') || '[]')
  if (isFavorite.value) {
    const idx = favorites.indexOf(props.channelId)
    if (idx > -1) favorites.splice(idx, 1)
  } else {
    favorites.push(props.channelId)
  }
  localStorage.setItem('favorite_channels', JSON.stringify(favorites))
  isFavorite.value = !isFavorite.value
}

async function toggleMute() {
  if (!authStore.user?.id) return
  
  try {
    const newMuteState = !isMuted.value
    await channelStore.updateNotifyProps(props.channelId, authStore.user.id, {
      mark_unread: newMuteState ? 'mention' : 'all'
    })
    
    const muted = JSON.parse(localStorage.getItem('muted_channels') || '[]')
    if (newMuteState) {
      muted.push(props.channelId)
    } else {
      const idx = muted.indexOf(props.channelId)
      if (idx > -1) muted.splice(idx, 1)
    }
    localStorage.setItem('muted_channels', JSON.stringify(muted))
    isMuted.value = newMuteState
  } catch (e) {
    console.error('Failed to toggle mute:', e)
  }
}

async function handleLeave() {
  if (!confirm('Are you sure you want to leave this channel?')) return
  
  const userId = authStore.user?.id
  if (!userId) return
  
  try {
    await channelStore.leaveChannel(props.channelId, userId)
    uiStore.closeRhs()
    
    // Navigate to first available channel
    const firstChannel = channelStore.channels[0]
    if (firstChannel && firstChannel.id !== props.channelId) {
      channelStore.selectChannel(firstChannel.id)
    } else {
      channelStore.clearChannels()
    }
  } catch (e) {
    console.error('Failed to leave channel:', e)
  }
}

function copyChannelLink() {
  const url = `${window.location.origin}/channels/${props.channelId}`
  navigator.clipboard.writeText(url)
  showCopiedToast.value = true
  setTimeout(() => showCopiedToast.value = false, 2000)
}

watch(() => props.channelId, async () => {
  await loadStats()
}, { immediate: true })
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 bg-bg-surface-1">
    <!-- Channel Header Card -->
    <div class="p-5 border-b border-border-1 bg-bg-surface-2/30">
      <div class="flex items-start space-x-3">
        <RcAvatar
          v-if="showDirectMessageProfile"
          :userId="directContactId || undefined"
          :username="directContact?.username || directContactMember?.username || directContactName"
          :src="directContact?.avatarUrl || directContactMember?.avatar_url"
          size="lg"
          class="shrink-0"
        />
        <div 
          v-else
          class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0"
          :class="channelTypeValue === 'private' ? 'bg-amber-100 text-amber-600' : 'bg-blue-100 text-blue-600'"
        >
          <component :is="channelIcon" class="w-6 h-6" />
        </div>
        <div class="flex-1 min-w-0">
          <h3 class="font-bold text-lg text-text-1 truncate">
            {{ showDirectMessageProfile ? directContactName : (channel?.display_name || channel?.name) }}
          </h3>
          <p class="text-xs text-text-3 font-medium">
            {{ showDirectMessageProfile ? `@${directContact?.username || directContactMember?.username || ''}` : channelTypeLabel }}
          </p>
        </div>
      </div>

      <div v-if="showDirectMessageProfile" data-testid="channel-info-dm-status" class="mt-3 flex flex-wrap items-center gap-2">
        <span data-testid="channel-info-presence" class="inline-flex items-center gap-1 rounded-full border px-3 py-1 text-sm font-medium" :class="directContactPresenceMeta.badgeClass">
          <component :is="directContactPresenceMeta.icon" class="h-3.5 w-3.5" />
          {{ directContactPresenceMeta.label }}
        </span>
        <span
          v-if="directContact?.statusText"
          data-testid="channel-info-custom-status"
          class="inline-flex max-w-full items-center gap-1 rounded-full border border-border-1 bg-bg-surface-1 px-3 py-1 text-sm text-text-2"
        >
          <span v-if="directContact?.statusEmoji">{{ directContact.statusEmoji }}</span>
          <span class="truncate">{{ directContact.statusText }}</span>
        </span>
        <span v-else-if="directContactLoading" class="text-xs text-text-3">Loading profile details...</span>
      </div>
      
      <!-- Purpose -->
      <p v-if="channel?.purpose && !showDirectMessageProfile" class="mt-3 text-sm text-text-2 leading-relaxed">
        {{ channel.purpose }}
      </p>
      
      <!-- Header/Topic -->
      <div v-if="channel?.header && !showDirectMessageProfile" class="mt-3 p-3 bg-bg-surface-1 rounded-lg border border-border-1">
        <p class="text-xs text-text-3 uppercase font-bold tracking-wider mb-1">Topic</p>
        <p class="text-sm text-text-2">{{ channel.header }}</p>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="p-4 border-b border-border-1">
      <div class="grid grid-cols-4 gap-2">
        <button 
          @click="toggleFavorite"
          class="flex flex-col items-center p-3 rounded-xl transition-all"
          :class="isFavorite ? 'bg-amber-100 text-amber-600' : 'hover:bg-surface-2 text-text-2'"
          :title="isFavorite ? 'Remove from favorites' : 'Add to favorites'"
        >
          <Star class="w-5 h-5 mb-1" :class="isFavorite && 'fill-current'" />
          <span class="text-[10px] font-medium">{{ isFavorite ? 'Favorited' : 'Favorite' }}</span>
        </button>
        
        <button 
          @click="toggleMute"
          class="flex flex-col items-center p-3 rounded-xl transition-all"
          :class="isMuted ? 'bg-red-100 text-red-600' : 'hover:bg-surface-2 text-text-2'"
          :title="isMuted ? 'Unmute channel' : 'Mute channel'"
        >
          <Bell class="w-5 h-5 mb-1" :class="isMuted && 'fill-current'" />
          <span class="text-[10px] font-medium">{{ isMuted ? 'Muted' : 'Mute' }}</span>
        </button>
        
        <button
          v-if="canManageCurrentChannel"
          @click="emit('openSettings')"
          class="flex flex-col items-center p-3 rounded-xl hover:bg-surface-2 text-text-2 transition-all"
          title="Edit channel settings"
        >
          <Edit3 class="w-5 h-5 mb-1" />
          <span class="text-[10px] font-medium">Edit</span>
        </button>
        
        <button 
          @click="handleLeave"
          class="flex flex-col items-center p-3 rounded-xl hover:bg-danger/10 text-text-2 hover:text-danger transition-all"
          title="Leave channel"
        >
          <LogOut class="w-5 h-5 mb-1" />
          <span class="text-[10px] font-medium">Leave</span>
        </button>
      </div>
    </div>

    <!-- Stats -->
    <div class="p-4 border-b border-border-1">
      <div class="grid grid-cols-2 gap-3">
        <div class="p-3 bg-bg-surface-2/50 rounded-xl">
          <div class="flex items-center space-x-2 text-text-3 mb-1">
            <Users class="w-4 h-4" />
            <span class="text-[11px] font-bold uppercase tracking-wider">Members</span>
          </div>
          <p class="text-2xl font-bold text-text-1">{{ memberCount }}</p>
        </div>
        
        <div class="p-3 bg-bg-surface-2/50 rounded-xl">
          <div class="flex items-center space-x-2 text-text-3 mb-1">
            <MessageSquare class="w-4 h-4" />
            <span class="text-[11px] font-bold uppercase tracking-wider">Messages</span>
          </div>
          <p class="text-2xl font-bold text-text-1">{{ messageCount }}</p>
        </div>
      </div>
    </div>

    <!-- Channel Details -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      <template v-if="showDirectMessageProfile">
        <div v-if="directContact?.email">
          <label class="text-[11px] font-bold text-text-3 uppercase tracking-wider mb-2 block">Email</label>
          <div class="flex items-center space-x-2 text-text-2">
            <Mail class="w-4 h-4 text-text-3" />
            <span class="text-sm">{{ directContact.email }}</span>
          </div>
        </div>

        <div v-if="directContact?.position">
          <label class="text-[11px] font-bold text-text-3 uppercase tracking-wider mb-2 block">Position</label>
          <div class="flex items-center space-x-2 text-text-2">
            <Briefcase class="w-4 h-4 text-text-3" />
            <span class="text-sm">{{ directContact.position }}</span>
          </div>
        </div>
      </template>

      <!-- Channel ID / Link -->
      <div>
        <label class="text-[11px] font-bold text-text-3 uppercase tracking-wider mb-2 block">
          {{ showDirectMessageProfile ? 'Conversation Link' : 'Channel Link' }}
        </label>
        <div class="flex items-center space-x-2">
          <code class="flex-1 px-3 py-2 bg-bg-surface-2 rounded-lg text-xs text-text-2 font-mono truncate">
            {{ channel?.name }}
          </code>
          <button 
            @click="copyChannelLink"
            class="p-2 hover:bg-surface-2 rounded-lg text-text-3 hover:text-text-1 transition-colors"
            title="Copy link"
          >
            <Copy class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Created -->
      <div v-if="channel?.created_at">
        <label class="text-[11px] font-bold text-text-3 uppercase tracking-wider mb-2 block">Created</label>
        <div class="flex items-center space-x-2 text-text-2">
          <Calendar class="w-4 h-4 text-text-3" />
          <span class="text-sm">{{ format(new Date(channel.created_at), 'MMMM d, yyyy') }}</span>
        </div>
      </div>

      <!-- Creator -->
      <div v-if="channel?.creator_id && !showDirectMessageProfile">
        <label class="text-[11px] font-bold text-text-3 uppercase tracking-wider mb-2 block">Created By</label>
        <div class="flex items-center space-x-2">
          <RcAvatar 
            :userId="channel.creator_id" 
            size="sm"
            class="w-6 h-6 rounded-md"
          />
          <span class="text-sm text-text-2">{{ isCreator ? 'You' : 'Unknown' }}</span>
        </div>
      </div>
    </div>

    <!-- Copied Toast -->
    <Transition name="fade">
      <div 
        v-if="showCopiedToast" 
        class="absolute bottom-4 left-1/2 -translate-x-1/2 px-4 py-2 bg-text-1 text-white rounded-lg text-sm font-medium shadow-lg flex items-center space-x-2"
      >
        <Check class="w-4 h-4" />
        <span>Link copied!</span>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(10px);
}
</style>
