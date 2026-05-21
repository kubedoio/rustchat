<script setup lang="ts">
import { log } from '@/utils/log'
import {
  Users,
  Search,
  Hash,
  Lock,
  Phone,
  Bookmark,
  MoreVertical,
  LogOut,
  Info,
  Pin,
  PhoneCall,
  PanelLeft,
} from 'lucide-vue-next'
import { ref, computed } from 'vue'
import { useCallsStore } from '../../stores/calls'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { useAuthStore } from '../../features/auth/stores/authStore'
import { useUIStore } from '../../features/ui/stores/uiStore'
import { useBreakpoints } from '../../composables/useBreakpoints'

const props = defineProps<{
  name: string
  topic?: string
  channelType?: string
  channelId: string
}>()

const emit = defineEmits<{
  (e: 'openSettings'): void
  (e: 'openSaved'): void
}>()

const callsStore = useCallsStore()
const channelStore = useChannelStore()
const authStore = useAuthStore()
const uiStore = useUIStore()
const { isMobile } = useBreakpoints()
const showMenu = ref(false)

const hasActiveCall = computed(() => {
  return callsStore.currentChannelCall(props.channelId)
})

const isInCurrentCall = computed(() => {
  return callsStore.isInCall && callsStore.currentCall?.channelId === props.channelId
})

const startNativeCall = async () => {
  if (!props.channelId) return

  if (isInCurrentCall.value) {
    callsStore.isExpanded = true
    return
  }

  if (hasActiveCall.value) {
    await callsStore.joinCall(props.channelId)
  } else {
    await callsStore.startCall(props.channelId)
  }
}

const joinExistingCall = async () => {
  if (hasActiveCall.value && props.channelId) {
    await callsStore.joinCall(props.channelId)
  }
}

const toggleView = (view: 'saved' | 'pinned' | 'search' | 'members') => {
  uiStore.toggleRhs(view)
}

const toggleSidebar = () => {
  uiStore.toggleLhs()
}

const handleLeave = async () => {
  if (!confirm('Are you sure you want to leave this channel?')) return

  const userId = authStore.user?.id

  if (props.channelId && userId) {
    try {
      await channelStore.leaveChannel(props.channelId, userId)
      showMenu.value = false

      const firstChannel = channelStore.channels[0]
      if (firstChannel) {
        channelStore.selectChannel(firstChannel.id)
      } else {
        channelStore.clearChannels()
      }
    } catch (e) {
      log.error('Failed to leave channel', e)
    }
  }
}
</script>

<template>
  <header
    class="sticky top-0 z-10 flex h-14 shrink-0 items-center justify-between border-b border-border-1 bg-bg-surface-1/95 px-3 backdrop-blur-sm sm:px-4"
  >
    <!-- Left: Channel Info -->
    <div class="flex min-w-0 items-center gap-2.5">
      <!-- Mobile Menu Toggle -->
      <button
        v-if="isMobile"
        @click="toggleSidebar"
        class="flex items-center justify-center w-8 h-8 rounded-r-2 hover:bg-bg-surface-2 text-text-2 transition-standard focus-ring"
        aria-label="Open channels"
      >
        <PanelLeft class="w-4 h-4" />
      </button>

      <!-- Channel Icon & Name -->
      <div class="flex min-w-0 items-center gap-2">
        <div
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-r-1 bg-brand/10 text-brand"
        >
          <component :is="channelType === 'private' ? Lock : Hash" class="h-4 w-4" />
        </div>
        <div class="min-w-0">
          <div class="truncate text-[10px] font-semibold uppercase tracking-[0.18em] text-brand/70">
            {{ channelType === 'private' ? 'Private channel' : 'Channel' }}
          </div>
          <h1 class="truncate text-sm font-semibold text-brand sm:text-base">
            {{ name }}
          </h1>
        </div>
      </div>

      <!-- Topic (hidden on mobile, truncated) -->
      <div class="ml-2 hidden min-w-0 md:block">
        <p v-if="topic" class="truncate text-xs text-text-3 max-w-xs lg:max-w-md">
          {{ topic }}
        </p>
        <p v-else class="text-xs text-text-4">No topic set yet</p>
      </div>
    </div>

    <!-- Right: Actions -->
    <div
      class="flex shrink-0 items-center gap-0.5 rounded-r-3 border border-border-1 bg-bg-surface-2/70 p-1 sm:gap-1"
    >
      <!-- Members Button -->
      <button
        @click="toggleView('members')"
        class="flex items-center justify-center w-11 h-11 rounded-r-2 transition-standard focus-ring"
        :class="{
          'bg-brand text-brand-foreground': uiStore.rhsView === 'members',
          'hover:bg-bg-surface-2 text-text-2': uiStore.rhsView !== 'members',
        }"
        title="Members"
        aria-label="Members"
      >
        <Users class="w-4 h-4" />
      </button>

      <div class="w-px h-5 bg-border-1 mx-1 hidden sm:block" />

      <!-- Call Buttons -->
      <button
        v-if="hasActiveCall && !isInCurrentCall"
        @click="joinExistingCall"
        class="flex items-center justify-center w-11 h-11 rounded-r-2 bg-success/10 text-success hover:bg-success/20 transition-standard animate-pulse focus-ring"
        title="Join active call"
        aria-label="Join active call"
      >
        <PhoneCall class="w-4 h-4" />
      </button>

      <button
        v-else-if="isInCurrentCall"
        @click="callsStore.isExpanded = true"
        class="flex items-center justify-center w-11 h-11 rounded-r-2 bg-success/10 text-success hover:bg-success/20 transition-standard focus-ring"
        title="Show call"
        aria-label="Show call"
      >
        <Phone class="w-4 h-4" />
      </button>

      <button
        v-else
        @click="startNativeCall"
        class="flex items-center justify-center w-11 h-11 rounded-r-2 hover:bg-success/10 text-success transition-standard focus-ring"
        title="Start audio call"
        aria-label="Start audio call"
      >
        <Phone class="w-4 h-4" />
      </button>

      <!-- Search Button (hidden on smallest screens) -->
      <button
        @click="toggleView('search')"
        class="hidden sm:flex items-center justify-center w-11 h-11 rounded-r-2 transition-standard focus-ring"
        :class="{
          'bg-brand text-brand-foreground': uiStore.rhsView === 'search',
          'hover:bg-bg-surface-2 text-text-2': uiStore.rhsView !== 'search',
        }"
        title="Search"
        aria-label="Search"
      >
        <Search class="w-4 h-4" />
      </button>

      <!-- Pinned Items -->
      <button
        @click="toggleView('pinned')"
        class="flex items-center justify-center w-11 h-11 rounded-r-2 transition-standard focus-ring"
        :class="{
          'bg-brand text-brand-foreground': uiStore.rhsView === 'pinned',
          'hover:bg-bg-surface-2 text-text-2': uiStore.rhsView !== 'pinned',
        }"
        title="Pinned items"
        aria-label="Pinned items"
      >
        <Pin class="w-4 h-4" />
      </button>

      <!-- Saved Items -->
      <button
        @click="toggleView('saved')"
        class="flex items-center justify-center w-11 h-11 rounded-r-2 transition-standard focus-ring"
        :class="{
          'bg-brand text-brand-foreground': uiStore.rhsView === 'saved',
          'hover:bg-bg-surface-2 text-text-2': uiStore.rhsView !== 'saved',
        }"
        title="Saved items"
        aria-label="Saved items"
      >
        <Bookmark class="w-4 h-4" />
      </button>

      <!-- More Options Menu -->
      <div class="relative">
        <button
          data-testid="channel-header-menu"
          @click="showMenu = !showMenu"
          class="flex items-center justify-center w-11 h-11 rounded-r-2 hover:bg-bg-surface-2 text-text-2 transition-standard focus-ring"
          :class="{ 'bg-bg-surface-2': showMenu }"
          title="More options"
          aria-label="More options"
        >
          <MoreVertical class="w-4 h-4" />
        </button>

        <!-- Dropdown Menu -->
        <Transition
          enter-active-class="transition-all duration-200 ease-out"
          enter-from-class="opacity-0 scale-95 -translate-y-1"
          enter-to-class="opacity-100 scale-100 translate-y-0"
          leave-active-class="transition-all duration-150 ease-in"
          leave-from-class="opacity-100 scale-100 translate-y-0"
          leave-to-class="opacity-0 scale-95 -translate-y-1"
        >
          <div
            v-if="showMenu"
            class="absolute right-0 top-full mt-2 w-48 bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2xl py-1 z-20 origin-top-right"
          >
            <button
              data-testid="channel-details-button"
              @click="
                uiStore.toggleRhs('info')
                showMenu = false
              "
              class="w-full px-4 py-2 text-left text-sm flex items-center gap-3 text-text-2 hover:bg-bg-surface-2 transition-standard"
            >
              <Info class="w-4 h-4" />
              Channel Details
            </button>

            <!-- Mobile-only search option -->
            <button
              @click="
                toggleView('search')
                showMenu = false
              "
              class="w-full px-4 py-2 text-left text-sm flex items-center gap-3 text-text-2 hover:bg-bg-surface-2 transition-standard sm:hidden"
            >
              <Search class="w-4 h-4" />
              Search
            </button>

            <div class="h-px bg-border-1 my-1" />

            <button
              @click="handleLeave"
              class="w-full px-4 py-2 text-left text-sm flex items-center gap-3 text-danger hover:bg-danger/5 transition-standard"
            >
              <LogOut class="w-4 h-4" />
              Leave Channel
            </button>
          </div>
        </Transition>

        <!-- Click outside -->
        <div v-if="showMenu" class="fixed inset-0 z-10" @click="showMenu = false" />
      </div>
    </div>
  </header>
</template>
