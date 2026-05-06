/**
 * Quick Switcher Composable - manages Cmd+K navigation
 */

import { ref, computed } from 'vue'
import type { RouteLocationRaw } from 'vue-router'
import { useTeamStore } from '../stores/teams'
import { useChannelStore } from '../stores/channels'

export interface QuickSwitcherItem {
  id: string
  type: 'channel' | 'dm' | 'team'
  name: string
  subtitle?: string
  icon: string
  to: RouteLocationRaw
}

// Module-level state shared across all composable instances
const isOpen = ref(false)
const recentItemIds = ref<string[]>([])

// Initialize from localStorage
try {
  const saved = localStorage.getItem('qs_recent')
  if (saved) recentItemIds.value = JSON.parse(saved)
} catch {
  // ignore
}

export function useQuickSwitcher() {
  const teamStore = useTeamStore()
  const channelStore = useChannelStore()

  const allItems = computed((): QuickSwitcherItem[] => {
    const items: QuickSwitcherItem[] = []

    // Channels
    for (const channel of channelStore.channels) {
      const team = teamStore.teams.find(t => t.id === channel.team_id)
      items.push({
        id: `channel-${channel.id}`,
        type: 'channel',
        name: channel.display_name || channel.name,
        subtitle: team?.display_name || team?.name,
        icon: (channel.channel_type === 'private' || (channel as any).type === 'private') ? 'Lock' : 'Hash',
        to: `/channels/${channel.id}`
      })
    }

    // Teams
    for (const team of teamStore.teams) {
      items.push({
        id: `team-${team.id}`,
        type: 'team',
        name: team.display_name || team.name,
        icon: 'Users',
        to: `/teams/${team.id}`
      })
    }

    return items
  })

  const recentItems = computed((): QuickSwitcherItem[] => {
    return recentItemIds.value
      .map(id => allItems.value.find(item => item.id === id))
      .filter((item): item is QuickSwitcherItem => item !== undefined)
  })

  function open() { isOpen.value = true }
  function close() { isOpen.value = false }
  function toggle() { isOpen.value = !isOpen.value }

  function addRecentItem(id: string) {
    recentItemIds.value = [id, ...recentItemIds.value.filter(i => i !== id)].slice(0, 10)
    try {
      localStorage.setItem('qs_recent', JSON.stringify(recentItemIds.value))
    } catch {
      // ignore
    }
  }

  return { isOpen, allItems, recentItems, open, close, toggle, addRecentItem }
}
