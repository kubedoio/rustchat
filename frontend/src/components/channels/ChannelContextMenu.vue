<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue';
import { 
    Check, 
    Star, 
    BellOff, 
    Bell, 
    FolderOpen, 
    Link, 
    UserPlus, 
    LogOut,
    Settings,
    Trash2
} from 'lucide-vue-next';
import { useChannelPreferencesStore } from '../../stores/channelPreferences';
import { useUnreadStore } from '../../stores/unreads';
import { useTeamStore } from '../../stores/teams';
import { useAuthStore } from '../../stores/auth';
import { channelRepository, channelService } from '../../features/channels';
import type { SidebarCategory } from '../../api/channels';
import { postsApi } from '../../api/posts';
import { useChannelManagementPermission } from '../../features/permissions/capabilities';

interface ChannelMenuItem {
    id: string
    label: string
    icon?: any
    action: () => void
    disabled?: boolean
    danger?: boolean
    separator?: boolean
}

const props = defineProps<{
    channelId: string
    channelName: string
    channelType: 'public' | 'private' | 'dm' | 'group'
    creatorId?: string | null
    isOwner?: boolean
    isAdmin?: boolean
    unreadCount?: number
    triggerElement?: EventTarget | null
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'action', action: string): void
    (e: 'open-add-members'): void
    (e: 'open-move-to', categories: SidebarCategory[]): void
    (e: 'open-edit'): void
    (e: 'open-delete'): void
}>()

const channelPrefsStore = useChannelPreferencesStore()
const unreadStore = useUnreadStore()
const teamStore = useTeamStore()
const authStore = useAuthStore()
const { canManageChannel: canManageCurrentChannel } = useChannelManagementPermission(
    () => props.channelId,
    () => props.creatorId ?? null,
)

const menuRef = ref<HTMLElement | null>(null)
const categories = ref<SidebarCategory[]>([])

// Menu position state - using fixed positioning to overlay above all panels
interface MenuPosition {
    left: string
    top: string
}

const menuPosition = ref<MenuPosition>({
    left: '0px',
    top: '0px'
})

// Calculate menu position based on trigger element
async function calculatePosition() {
    await nextTick()
    
    let triggerRect: DOMRect | undefined
    
    // Get the trigger element's position
    if (props.triggerElement instanceof HTMLElement) {
        triggerRect = props.triggerElement.getBoundingClientRect()
    } else {
        // Fallback: try to find the button that triggered this menu
        const buttons = document.querySelectorAll('button[title="More actions"]')
        for (const btn of buttons) {
            // Find the button closest to the channel we're showing menu for
            const channelEl = btn.closest('[class*="group/item"]')
            if (channelEl) {
                const channelName = channelEl.textContent?.includes(props.channelName)
                if (channelName) {
                    triggerRect = btn.getBoundingClientRect()
                    break
                }
            }
        }
    }
    
    if (!menuRef.value) return
    
    const menuWidth = 224 // w-56 = 14rem = 224px
    const menuHeight = menuRef.value.offsetHeight || 350 // estimated height
    const viewportWidth = window.innerWidth
    const viewportHeight = window.innerHeight
    
    // Default: position to the right of the trigger button
    let left = triggerRect ? triggerRect.right + 8 : 240
    let top = triggerRect ? triggerRect.top : 100
    
    // Check if menu would go off the right edge of screen
    if (left + menuWidth > viewportWidth - 16) {
        // Position to the left of the trigger instead
        left = triggerRect ? triggerRect.left - menuWidth - 8 : left - menuWidth - 16
    }
    
    // Check if menu would go off the bottom of screen
    if (top + menuHeight > viewportHeight - 16) {
        // Align bottom of menu with bottom of viewport
        top = viewportHeight - menuHeight - 16
    }
    
    // Ensure menu doesn't go off the left edge
    if (left < 16) {
        left = 16
    }
    
    // Ensure menu doesn't go off the top
    if (top < 16) {
        top = 16
    }
    
    menuPosition.value = {
        left: `${left}px`,
        top: `${top}px`
    }
}

// Check if favorited
const isFavorite = computed(() => channelPrefsStore.isFavorite(props.channelId))

// Check if muted
const isMuted = computed(() => channelPrefsStore.isMuted(props.channelId))

// Check if has unread
const hasUnread = computed(() => (props.unreadCount || 0) > 0)

// Fetch categories for Move To submenu
async function fetchCategories() {
    if (!authStore.user?.id || !teamStore.currentTeamId) return
    try {
        categories.value = await channelRepository.getCategories(
            authStore.user.id, 
            teamStore.currentTeamId
        )
    } catch (e) {
        console.error('Failed to fetch categories:', e)
    }
}

// Handle mark as read/unread
async function handleMarkReadUnread() {
    if (hasUnread.value) {
        await unreadStore.markAsRead(props.channelId)
    } else {
        const { data } = await postsApi.list(props.channelId, { limit: 1 })
        const latestPost = data.messages?.[0]
        if (latestPost?.id) {
            await unreadStore.markAsUnreadFromPost(latestPost.id)
        } else {
            await unreadStore.markAsUnread(props.channelId)
        }
    }
    emit('action', hasUnread.value ? 'mark-read' : 'mark-unread')
    emit('close')
}

// Handle favorite/unfavorite
async function handleFavorite() {
    await channelPrefsStore.toggleFavorite(props.channelId)
    emit('action', isFavorite.value ? 'unfavorite' : 'favorite')
    emit('close')
}

// Handle mute/unmute
async function handleMute() {
    await channelPrefsStore.toggleMute(props.channelId)
    emit('action', isMuted.value ? 'unmute' : 'mute')
    emit('close')
}

// Handle copy link
function handleCopyLink() {
    const url = `${window.location.origin}/channels/${props.channelId}`
    navigator.clipboard.writeText(url)
    emit('action', 'copy-link')
    emit('close')
}

// Handle add members
function handleAddMembers() {
    emit('open-add-members')
    emit('close')
}

// Handle leave channel
async function handleLeave() {
    if (!confirm(`Are you sure you want to leave #${props.channelName}?`)) {
        return
    }
    try {
        await channelRepository.leave(props.channelId)
        emit('action', 'leave')
    } catch (e) {
        console.error('Failed to leave channel:', e)
    }
    emit('close')
}

// Move to submenu handler
async function handleMoveTo() {
    if (categories.value.length === 0) {
        await fetchCategories()
    }
    emit('open-move-to', categories.value)
    emit('close')
}

// Handle edit channel
function handleEdit() {
    emit('open-edit')
    emit('close')
}

// Handle delete channel
async function handleDelete() {
    if (!confirm(`Are you sure you want to delete #${props.channelName}? This action cannot be undone.`)) {
        return
    }
    try {
        await channelService.deleteChannel(props.channelId)
        emit('action', 'delete')
    } catch (e) {
        console.error('Failed to delete channel:', e)
        alert('Failed to delete channel. Please try again.')
    }
    emit('close')
}

// Check if user can edit/delete (creator or admin)
const canEditOrDelete = computed(() => {
    return canManageCurrentChannel.value
})

// Menu items computed based on state
const menuItems = computed<ChannelMenuItem[]>(() => {
    const items: ChannelMenuItem[] = []

    // 1. Mark as Read / Mark as Unread (contextual)
    items.push({
        id: 'mark-read',
        label: hasUnread.value ? 'Mark as Read' : 'Mark as Unread',
        icon: Check,
        action: handleMarkReadUnread
    })

    // 2. Favorite / Unfavorite
    items.push({
        id: 'favorite',
        label: isFavorite.value ? 'Unfavorite' : 'Favorite',
        icon: Star,
        action: handleFavorite
    })

    // 3. Mute Channel / Unmute Channel
    items.push({
        id: 'mute',
        label: isMuted.value ? 'Unmute Channel' : 'Mute Channel',
        icon: isMuted.value ? Bell : BellOff,
        action: handleMute
    })

    // 4. Move to...
    items.push({
        id: 'move-to',
        label: 'Move to...',
        icon: FolderOpen,
        action: handleMoveTo
    })

    // Separator after Move to...
    items.push({ id: 'sep2', label: '', action: () => {}, separator: true })

    // 5. Copy Link
    items.push({
        id: 'copy-link',
        label: 'Copy Link',
        icon: Link,
        action: handleCopyLink
    })

    // 6. Add Members (not for DMs)
    if (
        props.channelType !== 'dm' &&
        props.channelType !== 'group' &&
        canManageCurrentChannel.value
    ) {
        items.push({
            id: 'add-members',
            label: 'Add Members',
            icon: UserPlus,
            action: handleAddMembers
        })
    }

    // Separator before management options
    items.push({ id: 'sep-management', label: '', action: () => {}, separator: true })

    // 7. Edit Channel (creator or admin only)
    if (canEditOrDelete.value) {
        items.push({
            id: 'edit',
            label: 'Edit Channel',
            icon: Settings,
            action: handleEdit
        })
    }

    // 8. Delete Channel (creator or admin only)
    if (canEditOrDelete.value) {
        items.push({
            id: 'delete',
            label: 'Delete Channel',
            icon: Trash2,
            action: handleDelete,
            danger: true
        })
    }

    // Separator before Leave
    items.push({ id: 'sep-leave', label: '', action: () => {}, separator: true })

    // 9. Leave Channel
    items.push({
        id: 'leave',
        label: 'Leave Channel',
        icon: LogOut,
        action: handleLeave,
        danger: true
    })

    return items
})

onMounted(() => {
    fetchCategories()
    calculatePosition()
})
</script>

<template>
    <!-- Backdrop to capture clicks outside - covers entire screen -->
    <div class="fixed inset-0 z-[9998]" @click="$emit('close')"></div>
    <div 
        ref="menuRef"
        class="fixed z-[9999] w-56 bg-bg-surface-1 rounded-lg shadow-2xl border border-border-1 py-1 animate-fade-in"
        :style="menuPosition"
        @click.stop
    >
        <template v-for="item in menuItems" :key="item.id">
            <!-- Separator -->
            <div v-if="item.separator" class="h-px bg-border-1 my-1"></div>
            
            <!-- Menu Item -->
            <button
                v-else
                @click="item.action"
                :disabled="item.disabled"
                class="w-full flex items-center px-3 py-2 text-sm text-left transition-standard"
                :class="[
                    item.danger 
                        ? 'text-danger hover:bg-danger/5' 
                        : 'text-text-2 hover:bg-bg-surface-2 hover:text-text-1',
                    item.disabled && 'opacity-50 cursor-not-allowed'
                ]"
            >
                <component v-if="item.icon" :is="item.icon" class="w-4 h-4 mr-3 opacity-70" />
                <span class="flex-1">{{ item.label }}</span>
            </button>
        </template>
    </div>
</template>

<style scoped>
.animate-fade-in {
    animation: fadeIn 0.1s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: scale(0.95);
    }
    to {
        opacity: 1;
        transform: scale(1);
    }
}
</style>
