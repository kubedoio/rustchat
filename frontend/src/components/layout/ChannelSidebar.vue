<script setup lang="ts">
import { log } from '@/utils/log'
import { ref, computed, watch, onMounted } from 'vue'
import {
  Hash,
  Lock,
  ChevronDown,
  ChevronRight,
  Plus,
  MessageCircle,
  Settings,
  Compass,
  Shield,
  Check,
  LogOut,
  MoreVertical,
} from 'lucide-vue-next'
import { useTeamStore } from '@/features/teams/stores/teamStore'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { useAuthStore } from '../../features/auth/stores/authStore'
import { useUnreadStore } from '@/features/unreads/stores/unreadStore'
import { useChannelPreferencesStore } from '@/features/channels/stores/channelPreferencesStore'
import { usePresenceStore } from '../../features/presence'
import CreateChannelModal from '../modals/CreateChannelModal.vue'
import DirectMessageModal from '../modals/DirectMessageModal.vue'
import TeamSettingsModal from '../modals/TeamSettingsModal.vue'
import BrowseTeamsModal from '../modals/BrowseTeamsModal.vue'
import BrowseChannelsModal from '../modals/BrowseChannelsModal.vue'
import ChannelContextMenu from '../channels/ChannelContextMenu.vue'
import AddChannelMembersModal from '../modals/AddChannelMembersModal.vue'
import EditChannelModal from '../channels/EditChannelModal.vue'
import type { SidebarCategory } from '../../api/channels'
import { channelRepository } from '../../features/channels/repositories/channelRepository'
import RcAvatar from '../ui/RcAvatar.vue'
import { getDirectMessageCounterpartyId } from '../../utils/directMessage'
import { getPresencePresentation } from '../../features/presence/presencePresentation'
import { getUserSummarySnapshot, prefetchUserSummaries } from '../../composables/useUserSummary'
import {
  canCreateChannel as canCreateChannelForRole,
  useCurrentTeamManagementPermission,
} from '../../features/permissions/capabilities'

const teamStore = useTeamStore()
const channelStore = useChannelStore()
const authStore = useAuthStore()
const unreadStore = useUnreadStore()
const channelPrefsStore = useChannelPreferencesStore()
const presenceStore = usePresenceStore()

const showCreateModal = ref(false)
const showDirectMessageModal = ref(false)
const showTeamSettings = ref(false)
const showTeamMenu = ref(false)
const showBrowseTeams = ref(false)
const showBrowseChannels = ref(false)
const showAddMembersModal = ref(false)
const addMembersChannelId = ref('')
const addMembersChannelName = ref('')

// Edit channel modal state
const showEditChannelModal = ref(false)
const editingChannel = ref<any>(null)

// Context menu state
const contextMenuChannel = ref<{
  id: string
  name: string
  type: 'public' | 'private' | 'dm' | 'group'
  unread: number
  isOwner: boolean
  creatorId: string | null
} | null>(null)
const contextMenuTrigger = ref<HTMLElement | null>(null)
const showMoveToModal = ref(false)
const moveToCategories = ref<SidebarCategory[]>([])
const moveToChannelId = ref('')
const { canManageTeam: canManageCurrentTeam } = useCurrentTeamManagementPermission(
  () => teamStore.currentTeamId
)
const canCreateChannelsInCurrentTeam = computed(
  () => !!teamStore.currentTeamId && canCreateChannelForRole(authStore.user?.role)
)

// Reload channels when team changes
watch(
  () => teamStore.currentTeamId,
  teamId => {
    if (teamId) {
      channelStore.fetchChannels(teamId)
      teamStore.fetchMembers(teamId)
    } else {
      channelStore.clearChannels()
    }
  },
  { immediate: true }
)

const directMessageUserIds = computed(() => {
  return [
    ...new Set(
      channelStore.directMessages
        .map(channel => getDirectMessageCounterpartyId(channel.name, authStore.user?.id))
        .filter((userId): userId is string => Boolean(userId))
    ),
  ]
})

watch(
  directMessageUserIds,
  userIds => {
    prefetchUserSummaries(userIds)
  },
  { immediate: true }
)

// Helper to deduplicate channels by ID
function dedupeChannels(channels: any[]) {
  const unique = new Map()
  channels.forEach(c => {
    if (!unique.has(c.id)) {
      unique.set(c.id, c)
    }
  })
  return Array.from(unique.values())
}

const categories = computed(() => {
  const allChannels = dedupeChannels([
    ...channelStore.publicChannels,
    ...channelStore.privateChannels,
    ...channelStore.directMessages,
  ])

  // Separate favorites
  const favoriteChannels = allChannels.filter(c => channelPrefsStore.isFavorite(c.id))
  const nonFavoritePublic = channelStore.publicChannels.filter(
    c => !channelPrefsStore.isFavorite(c.id)
  )
  const nonFavoritePrivate = channelStore.privateChannels.filter(
    c => !channelPrefsStore.isFavorite(c.id)
  )
  const nonFavoriteDMs = channelStore.directMessages.filter(
    c => !channelPrefsStore.isFavorite(c.id)
  )

  const result = []

  // Favorites category (if any favorites)
  if (favoriteChannels.length > 0) {
    result.push({
      id: 'favorites',
      name: 'Favorites',
      collapsed: false,
      channels: favoriteChannels.map(c => normalizeChannelForDisplay(c)),
    })
  }

  // Combine public and private channels, sort alphabetically by display name
  const allNonFavoriteChannels = dedupeChannels([...nonFavoritePublic, ...nonFavoritePrivate]).sort(
    (a, b) => {
      const nameA = (a.display_name || a.name || '').toLowerCase()
      const nameB = (b.display_name || b.name || '').toLowerCase()
      return nameA.localeCompare(nameB)
    }
  )

  // Regular categories
  result.push(
    {
      id: 'channels',
      name: 'Channels',
      collapsed: false,
      channels: allNonFavoriteChannels.map(c => normalizeChannelForDisplay(c)),
    },
    {
      id: 'dms',
      name: 'Direct Messages',
      collapsed: false,
      channels: dedupeChannels(nonFavoriteDMs).map(c => normalizeChannelForDisplay(c)),
    }
  )

  return result
})

function normalizeChannelForDisplay(c: any) {
  let displayName = c.display_name || c.name
  let otherId = ''
  let status = 'offline'
  let statusLabel = 'Offline'
  let statusText = ''
  let statusEmoji = ''
  let avatarUrl = ''
  let username = ''

  // Support both 'type' and 'channel_type' field names
  const channelTypeValue = c.type || c.channel_type

  // Handle DM channels
  if (channelTypeValue === 'direct' || c.name?.startsWith('dm_')) {
    otherId = getDirectMessageCounterpartyId(c.name, authStore.user?.id) || ''
    const summary = otherId ? getUserSummarySnapshot(otherId) : null
    const member = otherId ? teamStore.members.find(m => m.user_id === otherId) : null
    displayName =
      summary?.displayName ||
      member?.display_name ||
      summary?.username ||
      member?.username ||
      displayName
    avatarUrl = summary?.avatarUrl || member?.avatar_url || ''
    username = summary?.username || member?.username || ''
    status = summary?.presence || member?.presence || 'offline'
    statusText = summary?.statusText || ''
    statusEmoji = summary?.statusEmoji || ''
    statusLabel = getPresencePresentation(status).label
  }

  // Get unread counts from channel store
  const isCurrentChannel = channelStore.currentChannelId === c.id
  const unreadCount = isCurrentChannel ? 0 : c.unreadCount || 0
  const mentionCount = isCurrentChannel ? 0 : c.mentionCount || 0

  const channelType =
    channelTypeValue === 'direct'
      ? 'dm'
      : channelTypeValue === 'group'
        ? 'group'
        : channelTypeValue === 'private'
          ? 'private'
          : 'public'

  return {
    id: c.id,
    name: displayName,
    username: username || displayName,
    avatarUrl,
    userId: otherId,
    type: channelType,
    status: status,
    statusLabel,
    statusText,
    statusEmoji,
    unread: unreadCount,
    mention: mentionCount > 0,
    mentionCount: mentionCount,
    creator_id: c.creator_id,
    isTyping: presenceStore.hasTypingUsers(c.id),
  }
}

function isChannelOwner(channel: any): boolean {
  return channel.creator_id === authStore.user?.id
}

function isUserAdmin(): boolean {
  return ['system_admin', 'org_admin', 'admin'].includes(authStore.user?.role || '')
}

async function markChannelAsRead(channelId: string) {
  try {
    // Clear counts locally first (optimistic)
    channelStore.clearCounts(channelId)
    // Also call API
    const { useUnreadStore } = await import('@/features/unreads/stores/unreadStore')
    const store = useUnreadStore()
    await store.markAsRead(channelId)
  } catch (error) {
    log.error('Failed to mark channel as read:', error)
  }
}

const hasAnyUnread = computed(() => {
  return channelStore.channels.some(c => (c.unreadCount || 0) > 0)
})

async function markAllAsRead() {
  try {
    // Clear all counts in channel store
    channelStore.channels.forEach(c => {
      c.unreadCount = 0
      c.mentionCount = 0
    })
    // Call API
    await unreadStore.markAllAsRead()
  } catch (error) {
    log.error('Failed to mark all as read:', error)
  }
}

function openContextMenu(channel: any, event: MouseEvent) {
  event.stopPropagation()
  const channelType =
    channel.type === 'dm' ||
    channel.type === 'group' ||
    channel.type === 'public' ||
    channel.type === 'private'
      ? channel.type
      : 'public'
  contextMenuChannel.value = {
    id: channel.id,
    name: channel.name,
    type: channelType,
    unread: channel.unread,
    isOwner: isChannelOwner(channel),
    creatorId: channel.creator_id ?? null,
  }
  contextMenuTrigger.value = event.currentTarget as HTMLElement
}

function closeContextMenu() {
  contextMenuChannel.value = null
  contextMenuTrigger.value = null
}

function handleContextMenuAction(action: string) {
  log.debug('Context menu action:', action)
  if (action === 'leave' || action === 'delete') {
    if (contextMenuChannel.value) {
      channelStore.removeChannel(contextMenuChannel.value.id)
    }
    if (teamStore.currentTeamId) {
      channelStore.fetchChannels(teamStore.currentTeamId)
    }
  }
}

function handleOpenAddMembers() {
  if (contextMenuChannel.value) {
    addMembersChannelId.value = contextMenuChannel.value.id
    addMembersChannelName.value = contextMenuChannel.value.name
    showAddMembersModal.value = true
  }
}

function handleOpenMoveTo(cats: SidebarCategory[]) {
  moveToCategories.value = cats
  if (contextMenuChannel.value) {
    moveToChannelId.value = contextMenuChannel.value.id
    showMoveToModal.value = true
  }
}

async function handleMoveToCategory(cat: SidebarCategory) {
  if (!authStore.user?.id || !teamStore.currentTeamId) return
  try {
    await channelRepository.updateCategories(authStore.user.id, teamStore.currentTeamId, [cat])
    showMoveToModal.value = false
  } catch (e) {
    log.error('Failed to move channel:', e)
  }
}

// Handle edit channel
function handleOpenEditChannel() {
  if (contextMenuChannel.value) {
    // Find the full channel data
    const channel = channelStore.channels.find(c => c.id === contextMenuChannel.value?.id)
    if (channel) {
      editingChannel.value = channel
      showEditChannelModal.value = true
    }
  }
}

// Handle channel updated
function handleChannelUpdated() {
  showEditChannelModal.value = false
  editingChannel.value = null
}

// Handle delete channel
async function handleOpenDeleteChannel() {
  // This is handled within ChannelContextMenu now
}

onMounted(() => {
  channelPrefsStore.fetchPreferences()
})

function selectChannel(channelId: string) {
  channelStore.selectChannel(channelId)
}

const collapsedCategories = ref(new Set<string>())

function toggleCategory(catId: string) {
  if (collapsedCategories.value.has(catId)) {
    collapsedCategories.value.delete(catId)
  } else {
    collapsedCategories.value.add(catId)
  }
}

function isCategoryCollapsed(catId: string) {
  return collapsedCategories.value.has(catId)
}

function handleTeamDeleted() {
  teamStore.removeTeam(teamStore.currentTeamId || '')
}

function handleAddCategory(catId: string) {
  if (catId === 'dms') {
    showDirectMessageModal.value = true
  } else {
    if (!canCreateChannelsInCurrentTeam.value) {
      return
    }
    showCreateModal.value = true
  }
}

async function handleLeaveTeam() {
  if (!teamStore.currentTeam) return
  if (
    !confirm(
      `Are you sure you want to leave ${teamStore.currentTeam.display_name || teamStore.currentTeam.name}?`
    )
  )
    return

  try {
    await teamStore.leaveTeam(teamStore.currentTeam.id)
    showTeamMenu.value = false
  } catch (e) {
    log.error('Failed to leave team', e)
  }
}
</script>

<template>
  <aside
    class="relative z-20 flex w-[var(--sidebar-width)] shrink-0 select-none flex-col bg-bg-surface-2/95"
  >
    <!-- Team Header -->
    <div
      class="group flex h-[var(--header-height)] cursor-pointer items-center justify-between border-b border-border-1 px-3 transition-standard hover:bg-bg-surface-1"
      @click="showTeamMenu = !showTeamMenu"
    >
      <div class="min-w-0">
        <div class="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-3">
          Workspace
        </div>
        <h2 class="truncate text-sm font-semibold tracking-tight text-text-1">
          {{ teamStore.currentTeam?.display_name || teamStore.currentTeam?.name || 'Select Team' }}
        </h2>
      </div>
      <div class="ml-3 flex shrink-0 items-center gap-2">
        <span
          class="hidden rounded-full border border-border-1 bg-bg-surface-1 px-2 py-0.5 text-[11px] font-medium text-text-2 md:inline-flex"
        >
          {{ channelStore.channels.length }} spaces
        </span>
        <ChevronDown
          class="h-4 w-4 shrink-0 text-text-3 transition-standard group-hover:text-text-1"
          :class="{ 'rotate-180': showTeamMenu }"
        />
      </div>

      <!-- Team Dropdown Menu -->
      <div
        v-if="showTeamMenu"
        class="absolute top-[calc(var(--header-height)-4px)] left-3 right-3 bg-bg-surface-1 rounded-r-2 shadow-2xl border border-border-1 py-1 z-50 animate-fade-in"
        @click.stop
      >
        <button
          v-if="authStore.user?.role === 'system_admin' || authStore.user?.role === 'org_admin'"
          @click="
            $router.push('/admin')
            showTeamMenu = false
          "
          class="w-full flex items-center gap-3 px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-standard"
        >
          <Shield class="w-4 h-4 text-brand" />
          System Console
        </button>

        <div
          v-if="authStore.user?.role === 'system_admin' || authStore.user?.role === 'org_admin'"
          class="h-px bg-border-1 my-1"
        />

        <button
          @click="
            showBrowseTeams = true
            showTeamMenu = false
          "
          class="w-full flex items-center gap-3 px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-standard"
        >
          <Compass class="w-4 h-4" />
          Browse Teams
        </button>
        <button
          v-if="canManageCurrentTeam"
          @click="
            showTeamSettings = true
            showTeamMenu = false
          "
          class="w-full flex items-center gap-3 px-4 py-2 text-sm text-text-2 hover:bg-bg-surface-2 transition-standard"
        >
          <Settings class="w-4 h-4" />
          Team Settings
        </button>

        <div class="h-px bg-border-1 my-1" />

        <button
          @click="handleLeaveTeam"
          class="w-full flex items-center gap-3 px-4 py-2 text-sm text-danger hover:bg-danger/5 transition-standard"
        >
          <LogOut class="w-4 h-4" />
          Leave Team
        </button>
      </div>
    </div>

    <!-- Click outside to close menu -->
    <div v-if="showTeamMenu" class="fixed inset-0 z-40" @click="showTeamMenu = false" />

    <!-- Scrollable Content -->
    <div class="flex-1 overflow-y-auto py-3 custom-scrollbar-thin">
      <!-- Loading State -->
      <div
        v-if="channelStore.loading"
        class="flex flex-col items-center justify-center py-8 text-text-3"
      >
        <div
          class="w-6 h-6 border-2 border-brand border-t-transparent rounded-full animate-spin mb-3"
        />
        <p class="text-xs">Loading channels...</p>
      </div>

      <!-- Categories -->
      <div v-else class="space-y-2 px-2">
        <div v-for="cat in categories" :key="cat.id">
          <!-- Category Header -->
          <div
            class="group flex cursor-pointer items-center justify-between rounded-r-1 px-2 py-1.5 text-text-3 hover:text-text-1"
            @click="toggleCategory(cat.id)"
          >
            <div
              class="flex items-center gap-1 text-[11px] font-semibold uppercase tracking-[0.16em]"
            >
              <component
                :is="isCategoryCollapsed(cat.id) ? ChevronRight : ChevronDown"
                class="h-3.5 w-3.5 transition-transform"
              />
              {{ cat.name }}
            </div>
            <div class="flex items-center gap-1">
              <span class="text-[11px] font-medium normal-case tracking-normal text-text-4">
                {{ cat.channels.length }}
              </span>
              <button
                v-if="cat.id === 'dms' || canCreateChannelsInCurrentTeam"
                @click.stop="handleAddCategory(cat.id)"
                class="rounded p-1 opacity-0 transition-standard hover:bg-bg-surface-1 group-hover:opacity-100"
                :title="cat.id === 'dms' ? 'New direct message' : 'Create channel'"
              >
                <Plus class="h-3.5 w-3.5" />
              </button>
            </div>
          </div>

          <!-- Channels List -->
          <div v-if="!isCategoryCollapsed(cat.id)" class="mt-1 space-y-1">
            <div
              v-for="channel in cat.channels"
              :key="channel.id"
              :data-testid="channel.type === 'dm' ? 'dm-sidebar-row' : 'channel-sidebar-row'"
              :data-channel-id="channel.id"
              :data-user-id="channel.userId"
              @click="selectChannel(channel.id)"
              @contextmenu.prevent="openContextMenu(channel, $event)"
              class="group/item relative flex cursor-pointer items-center justify-between rounded-r-2 border px-2.5 py-2 transition-standard"
              :class="{
                'border-brand/25 bg-bg-surface-1 shadow-1':
                  channelStore.currentChannelId === channel.id,
                'border-transparent hover:border-border-1 hover:bg-bg-surface-1':
                  channelStore.currentChannelId !== channel.id,
              }"
            >
              <div
                v-if="channelStore.currentChannelId === channel.id"
                class="absolute left-0 top-1/2 h-7 w-[3px] -translate-y-1/2 rounded-r-full bg-brand"
              />
              <div
                class="flex items-center min-w-0 gap-2"
                :class="{
                  'opacity-80':
                    channelStore.currentChannelId !== channel.id && channel.unread === 0,
                  'opacity-100': channelStore.currentChannelId === channel.id || channel.unread > 0,
                }"
              >
                <!-- Channel Icon -->
                <span
                  class="shrink-0"
                  :class="
                    channelStore.currentChannelId === channel.id ? 'text-brand' : 'text-text-3'
                  "
                >
                  <Hash v-if="channel.type === 'public'" class="w-4 h-4" />
                  <Lock v-else-if="channel.type === 'private'" class="w-3.5 h-3.5" />
                  <RcAvatar
                    v-else-if="channel.type === 'dm'"
                    :user-id="channel.userId"
                    :username="channel.username"
                    :src="channel.avatarUrl"
                    size="sm"
                    class="ring-1 ring-border-1/70"
                  />
                  <MessageCircle v-else class="w-4 h-4" />
                </span>

                <!-- Channel Name -->
                <div class="min-w-0">
                  <span
                    class="block truncate text-sm"
                    :class="{
                      'text-brand font-semibold': channelStore.currentChannelId === channel.id,
                      'text-text-1 font-semibold':
                        channelStore.currentChannelId !== channel.id &&
                        (channel.unread > 0 || channel.mention),
                      'text-text-2 font-medium':
                        channel.unread === 0 &&
                        !channel.mention &&
                        channelStore.currentChannelId !== channel.id,
                    }"
                  >
                    {{ channel.name }}
                  </span>
                  <span
                    v-if="channel.type === 'dm'"
                    data-testid="dm-sidebar-status"
                    class="mt-0.5 block truncate text-[11px]"
                    :class="
                      channelStore.currentChannelId === channel.id ? 'text-text-2' : 'text-text-3'
                    "
                  >
                    <template v-if="channel.statusText || channel.statusEmoji">
                      <span v-if="channel.statusEmoji">{{ channel.statusEmoji }}</span>
                      <span v-if="channel.statusEmoji && channel.statusText" class="mx-1">·</span>
                      {{ channel.statusText || channel.statusLabel }}
                    </template>
                    <template v-else>
                      {{ channel.statusLabel }}
                    </template>
                  </span>
                </div>
              </div>

              <!-- Status/Unread Indicators -->
              <div class="flex items-center gap-1.5 ml-2 shrink-0">
                <!-- Mark as read button (on hover) -->
                <button
                  v-if="channel.unread > 0"
                  @click.stop="markChannelAsRead(channel.id)"
                  class="flex h-5 w-5 items-center justify-center rounded opacity-0 transition-opacity group-hover/item:opacity-100"
                  :class="
                    channelStore.currentChannelId === channel.id
                      ? 'text-brand hover:bg-brand/10'
                      : 'text-text-3 hover:bg-bg-surface-2 hover:text-text-1'
                  "
                  title="Mark as read"
                >
                  <Check class="w-3.5 h-3.5" />
                </button>

                <!-- Mention badge -->
                <div
                  v-if="channel.mentionCount > 0"
                  class="flex h-5 shrink-0 items-center justify-center rounded-full bg-danger px-1.5 text-[11px] font-bold text-white"
                >
                  {{ channel.mentionCount > 99 ? '99+' : channel.mentionCount }}
                </div>

                <!-- Unread badge -->
                <div
                  v-else-if="channel.unread > 0"
                  class="shrink-0 w-2 h-2 rounded-full bg-text-2"
                  :class="channelStore.currentChannelId === channel.id ? 'bg-brand' : 'bg-text-2'"
                />

                <!-- Typing indicator badge -->
                <div
                  v-else-if="channel.isTyping && channelStore.currentChannelId !== channel.id"
                  class="shrink-0 flex items-center gap-0.5"
                  title="Someone is typing..."
                >
                  <div
                    class="w-1 h-1 rounded-full bg-blue-500 animate-bounce"
                    style="animation-delay: 0ms"
                  ></div>
                  <div
                    class="w-1 h-1 rounded-full bg-blue-500 animate-bounce"
                    style="animation-delay: 150ms"
                  ></div>
                  <div
                    class="w-1 h-1 rounded-full bg-blue-500 animate-bounce"
                    style="animation-delay: 300ms"
                  ></div>
                </div>

                <!-- Context Menu Trigger -->
                <button
                  @click.stop="openContextMenu(channel, $event)"
                  class="flex h-6 w-6 items-center justify-center rounded opacity-0 transition-opacity group-hover/item:opacity-100"
                  :class="
                    channelStore.currentChannelId === channel.id
                      ? 'text-text-3 hover:bg-bg-surface-2 hover:text-text-1'
                      : 'text-text-3 hover:bg-bg-surface-2 hover:text-text-1'
                  "
                  title="More actions"
                >
                  <MoreVertical class="w-4 h-4" />
                </button>

                <!-- Context Menu -->
                <Teleport to="body" v-if="contextMenuChannel?.id === channel.id">
                  <ChannelContextMenu
                    :channel-id="contextMenuChannel!.id"
                    :channel-name="contextMenuChannel!.name"
                    :channel-type="contextMenuChannel!.type"
                    :creator-id="contextMenuChannel!.creatorId"
                    :is-owner="contextMenuChannel!.isOwner"
                    :is-admin="isUserAdmin()"
                    :unread-count="channel.unread"
                    :trigger-element="contextMenuTrigger"
                    @close="closeContextMenu"
                    @action="handleContextMenuAction"
                    @open-add-members="handleOpenAddMembers"
                    @open-move-to="handleOpenMoveTo"
                    @open-edit="handleOpenEditChannel"
                    @open-delete="handleOpenDeleteChannel"
                  />
                </Teleport>
              </div>
            </div>

            <!-- Empty Category State -->
            <div v-if="cat.channels.length === 0" class="px-8 py-3 text-xs text-text-3 italic">
              No channels
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer Actions -->
    <div class="shrink-0 space-y-1 border-t border-border-1 p-2">
      <div class="px-2 pb-1 text-[10px] font-semibold uppercase tracking-[0.18em] text-text-3">
        Quick Actions
      </div>
      <button
        v-if="hasAnyUnread"
        @click="markAllAsRead()"
        class="flex w-full items-center gap-3 rounded-r-1 px-2 py-1.5 text-left text-xs text-text-3 transition-standard hover:bg-bg-surface-1 hover:text-text-1"
      >
        <Check class="w-4 h-4" />
        <span>Mark all as read</span>
      </button>
      <button
        @click="showBrowseChannels = true"
        class="flex w-full items-center gap-3 rounded-r-1 px-2 py-1.5 text-left text-xs text-text-3 transition-standard hover:bg-bg-surface-1 hover:text-text-1"
      >
        <Compass class="w-4 h-4" />
        <span>Browse channels</span>
      </button>
      <button
        v-if="canCreateChannelsInCurrentTeam"
        @click="showCreateModal = true"
        class="flex w-full items-center gap-3 rounded-r-1 px-2 py-1.5 text-left text-xs text-text-3 transition-standard hover:bg-bg-surface-1 hover:text-text-1"
      >
        <Plus class="w-4 h-4" />
        <span>Create channel</span>
      </button>
    </div>

    <!-- Modals -->
    <CreateChannelModal :show="showCreateModal" @close="showCreateModal = false" />
    <DirectMessageModal :show="showDirectMessageModal" @close="showDirectMessageModal = false" />
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
    <AddChannelMembersModal
      :show="showAddMembersModal"
      :channel-id="addMembersChannelId"
      :channel-name="addMembersChannelName"
      @close="showAddMembersModal = false"
    />
    <EditChannelModal
      :is-open="showEditChannelModal"
      :channel="editingChannel"
      @close="showEditChannelModal = false"
      @updated="handleChannelUpdated"
    />

    <!-- Move to Category Modal -->
    <Teleport to="body" v-if="showMoveToModal">
      <div class="fixed inset-0 z-50 flex items-center justify-center">
        <div
          class="absolute inset-0 bg-black/50 backdrop-blur-sm"
          @click="showMoveToModal = false"
        />
        <div
          class="relative bg-bg-surface-1 rounded-r-2 shadow-2xl w-64 py-2 border border-border-1"
        >
          <div
            class="px-3 py-2 text-xs font-bold text-text-3 uppercase tracking-wider border-b border-border-1 mb-1"
          >
            Move to...
          </div>
          <button
            v-for="cat in moveToCategories"
            :key="cat.id"
            @click="handleMoveToCategory(cat)"
            class="w-full px-4 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 hover:text-text-1 transition-standard"
          >
            {{ cat.display_name }}
          </button>
          <div
            v-if="moveToCategories.length === 0"
            class="px-4 py-3 text-sm text-text-3 text-center"
          >
            No categories available
          </div>
        </div>
      </div>
    </Teleport>
  </aside>
</template>
