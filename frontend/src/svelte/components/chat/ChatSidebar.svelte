<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { svelteApi } from '../../stores/http'
  import { fly } from 'svelte/transition'
  import {
    Hash,
    Lock,
    Plus,
    Globe,
    MessageCircle,
    ChevronRight,
    ChevronDown,
    Check,
    BellOff,
    LogOut,
    Settings,
  } from 'lucide-svelte'
  import { authStore } from '../../stores/auth'
  import type { ChatMember, ChatTeam } from './types'
  import type { SvelteChatChannel } from '../../stores/chat'
  import UserMenu from '../ui/UserMenu.svelte'

  interface SidebarChannel extends SvelteChatChannel {
    displayName?: string
    unreadCount?: number
    is_favorite?: boolean
  }

  interface Props {
    teams?: ChatTeam[]
    channels?: SidebarChannel[]
    unreadCounts?: Record<string, number>
    activeChannelId?: string
    currentChannelId?: string | null
    currentUserId?: string
    members?: ChatMember[]
    onSelectChannel?: (channelId: string) => void | Promise<void>
  }

  let {
    teams = [],
    channels = [],
    unreadCounts = {},
    activeChannelId = '',
    currentChannelId = null,
    currentUserId = undefined,
    members = [],
    onSelectChannel = undefined,
  }: Props = $props()

  const dispatch = createEventDispatcher<{
    selectChannel: SidebarChannel
    createChannel: void
    browseChannels: void
    directMessage: void
    setStatus: void
    createTeam: void
    editProfile: void
    markAllRead: void
    teamSettings: void
    browseTeams: void
  }>()

  let selectedChannelId = $derived(currentChannelId ?? activeChannelId ?? '')
  let currentTeam = $derived(teams[0])

  let collapsedCategories = $state<Record<string, boolean>>({})
  let contextMenu = $state<{ x: number; y: number; channel: SidebarChannel } | null>(null)
  let showTeamMenu = $state(false)

  function teamName(team: ChatTeam | undefined) {
    return team?.displayName ?? team?.display_name ?? team?.name ?? 'Select Team'
  }

  function channelDisplayName(channel: SidebarChannel) {
    return channel.display_name ?? channel.displayName ?? channel.name
  }

  function channelsForTeam(team: ChatTeam | undefined): SidebarChannel[] {
    if (!team) return channels
    if (team.channels?.length) {
      return team.channels as SidebarChannel[]
    }
    return channels.filter((channel) => !channel.team_id || channel.team_id === team.id)
  }

  function selectChannel(channel: SidebarChannel) {
    void onSelectChannel?.(channel.id)
    dispatch('selectChannel', channel)
  }

  function getInitials(name: string) {
    return name
      .split(' ')
      .map((n) => n[0])
      .filter(Boolean)
      .join('')
      .toUpperCase()
      .slice(0, 2)
  }

  function presenceLabel(presence?: string | null): string {
    switch (presence) {
      case 'online':
        return 'Online'
      case 'away':
        return 'Away'
      case 'dnd':
        return 'Do not disturb'
      default:
        return 'Offline'
    }
  }

  function presenceDotClass(presence?: string | null): string {
    switch (presence) {
      case 'online':
        return 'bg-success'
      case 'away':
        return 'bg-warning'
      case 'dnd':
        return 'bg-danger'
      default:
        return 'bg-text-4'
    }
  }

  function unreadCount(channel: SidebarChannel): number {
    return unreadCounts[channel.id] ?? channel.unreadCount ?? 0
  }

  function memberForDm(channel: SidebarChannel): ChatMember | null {
    const normalizedName = channel.name.toLowerCase()
    const normalizedDisplayName = channelDisplayName(channel).toLowerCase()

    return (
      members.find((member) => {
        if (currentUserId && (member.user_id ?? member.id) === currentUserId) return false
        const displayName = member.displayName ?? member.display_name ?? ''
        return (
          normalizedName.includes(member.username.toLowerCase()) ||
          normalizedDisplayName === displayName.toLowerCase() ||
          normalizedDisplayName.includes(member.username.toLowerCase())
        )
      }) ??
      members.find((member) => !currentUserId || (member.user_id ?? member.id) !== currentUserId) ??
      null
    )
  }

  function memberStatusText(member: ChatMember | null): string {
    return member?.status_text ?? member?.statusText ?? ''
  }

  function memberStatusEmoji(member: ChatMember | null): string {
    return member?.status_emoji ?? member?.statusEmoji ?? ''
  }

  function toggleCategory(category: string) {
    collapsedCategories[category] = !collapsedCategories[category]
  }

  function openContextMenu(event: MouseEvent, channel: SidebarChannel) {
    contextMenu = { x: event.clientX, y: event.clientY, channel }
  }

  function closeContextMenu() {
    contextMenu = null
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      showTeamMenu = false
      contextMenu = null
    }
  }

  async function markChannelRead(channelId: string) {
    try {
      await svelteApi.post(`/channels/${channelId}/read`)
      unreadCounts[channelId] = 0
    } catch (e) {
      console.error('Failed to mark channel as read:', e)
    }
  }

  function handleAddCategory(catId: string) {
    if (catId === 'dms') {
      dispatch('directMessage')
    } else {
      dispatch('createChannel')
    }
  }

  let teamChs = $derived(channelsForTeam(currentTeam))
  let favoriteChannels = $derived(teamChs.filter((c) => c.is_favorite))
  let publicChannels = $derived(teamChs.filter((c) => c.channel_type === 'public' && !c.is_favorite))
  let privateChannels = $derived(teamChs.filter((c) => c.channel_type === 'private' && !c.is_favorite))
  let dmChannels = $derived(
    teamChs.filter((c) => (c.channel_type === 'direct' || c.channel_type === 'group') && !c.is_favorite),
  )

  let hasAnyUnread = $derived(channels.some((c) => unreadCount(c) > 0))

  let isAdmin = $derived(['system_admin', 'org_admin'].includes($authStore.user?.role || ''))
</script>

{#snippet channelRow(channel: SidebarChannel)}
  {@const isSelected = channel.id === selectedChannelId}
  {@const count = unreadCount(channel)}
  {@const mention = channel.mentionCount ?? 0}
  <div class="group/item relative">
    <button
      class="flex items-center gap-2 w-full px-3 py-1.5 text-sm transition-standard rounded-r-1
        {isSelected ? 'bg-brand text-brand-foreground shadow-1' : 'text-text-2 hover:bg-bg-surface-1'}"
      onclick={() => selectChannel(channel)}
      oncontextmenu={(e) => { e.preventDefault(); openContextMenu(e, channel) }}
      aria-current={isSelected ? 'page' : undefined}
    >
      {#if channel.channel_type === 'direct'}
        {@const member = memberForDm(channel)}
        {@const presence = member?.presence ?? 'offline'}
        {@const statusText = memberStatusText(member)}
        {@const statusEmoji = memberStatusEmoji(member)}
        {@const statusLabel = presenceLabel(presence)}
        <span class="relative shrink-0">
          <div class="h-6 w-6 rounded-full bg-brand/10 text-brand flex items-center justify-center text-[10px] font-bold">
            {getInitials(channelDisplayName(channel))}
          </div>
          <span class="absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full ring-2 ring-bg-surface-2 {presenceDotClass(presence)}" aria-hidden="true"></span>
        </span>
        <span class="min-w-0 flex flex-col items-start">
          <span class="truncate {count > 0 || mention > 0 ? 'font-semibold' : 'font-medium'}">{channelDisplayName(channel)}</span>
          <span class="mt-0.5 block truncate text-[11px] {isSelected ? 'text-text-3' : 'text-text-4'}">
            {#if statusText || statusEmoji}
              {#if statusEmoji && statusText}
                {statusEmoji} {statusText}
              {:else}
                {statusText || statusEmoji || statusLabel}
              {/if}
            {:else}
              {statusLabel}
            {/if}
          </span>
        </span>
      {:else if channel.channel_type === 'group'}
        <MessageCircle class="w-4 h-4 shrink-0" />
        <span class="truncate flex-1 text-left {count > 0 || mention > 0 ? 'font-semibold' : ''}">{channelDisplayName(channel)}</span>
      {:else if channel.channel_type === 'private'}
        <Lock class="w-4 h-4 shrink-0" />
        <span class="truncate flex-1 text-left {count > 0 || mention > 0 ? 'font-semibold' : ''}">{channelDisplayName(channel)}</span>
      {:else}
        <Hash class="w-4 h-4 shrink-0" />
        <span class="truncate flex-1 text-left {count > 0 || mention > 0 ? 'font-semibold' : ''}">{channelDisplayName(channel)}</span>
      {/if}

      {#if mention > 0}
        <span class="shrink-0 min-w-[18px] h-[18px] px-1 flex items-center justify-center rounded-full bg-danger text-white text-[10px] font-bold">
          {mention > 99 ? '99+' : mention}
        </span>
      {:else if count > 0}
        <span class="shrink-0 w-2 h-2 rounded-full {isSelected ? 'bg-brand-foreground' : 'bg-text-2'}"></span>
      {/if}
    </button>

    {#if count > 0 && !isSelected}
      <button
        class="absolute right-1 top-1/2 -translate-y-1/2 opacity-0 group-hover/item:opacity-100 p-1 hover:bg-bg-surface-1 rounded transition-standard"
        title="Mark as read"
        onclick={(e) => { e.stopPropagation(); markChannelRead(channel.id) }}
      >
        <Check class="w-3.5 h-3.5 text-text-3" />
      </button>
    {/if}
  </div>
{/snippet}

{#snippet categorySection(id: string, label: string, categoryChannels: SidebarChannel[])}
  <div class="group mb-1">
    <div
      class="flex items-center gap-1 px-3 py-1 text-xs font-semibold text-text-3 uppercase tracking-wider hover:text-text-2 transition-standard w-full cursor-pointer"
      onclick={() => toggleCategory(id)}
      role="button"
      tabindex="0"
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleCategory(id) } }}
    >
      <ChevronRight class="w-3 h-3 transition-transform {collapsedCategories[id] ? '' : 'rotate-90'}" />
      <span class="flex-1 text-left">{label}</span>
      <button
        class="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-bg-surface-1 rounded transition-standard"
        onclick={(e) => { e.stopPropagation(); handleAddCategory(id) }}
        title={id === 'dms' ? 'New direct message' : 'Create channel'}
      >
        <Plus class="w-3 h-3" />
      </button>
    </div>

    {#if !collapsedCategories[id]}
      <div class="mt-0.5 space-y-0.5">
        {#each categoryChannels as channel (channel.id)}
          {@render channelRow(channel)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<svelte:window onkeydown={handleKeydown} />

<aside class="flex w-72 shrink-0 flex-col border-r border-border-1 bg-bg-surface-2 text-text-1" aria-label="Chat sidebar">
  <!-- Team Header -->
  <div class="relative border-b border-border-1">
    <button
      class="flex items-center gap-1 px-3 py-2 text-sm font-semibold text-text-1 hover:bg-bg-surface-1 rounded transition-standard w-full"
      onclick={() => (showTeamMenu = !showTeamMenu)}
    >
      <span class="truncate flex-1 text-left">{teamName(currentTeam)}</span>
      <ChevronDown class="w-4 h-4 shrink-0 transition-transform {showTeamMenu ? 'rotate-180' : ''}" />
    </button>

    {#if showTeamMenu}
      <div class="fixed inset-0 z-30" onclick={() => (showTeamMenu = false)} role="presentation"></div>
      <div class="absolute left-2 right-2 top-full mt-1 bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 py-1 z-40" role="menu" transition:fly={{ duration: 150, y: -5 }}>
        {#if isAdmin}
          <button
            class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard"
            onclick={() => { /* system console */ showTeamMenu = false }}
            role="menuitem"
          >
            <Settings class="w-4 h-4" />
            System Console
          </button>
        {/if}
        <button
          class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard"
          onclick={() => { dispatch('browseTeams'); showTeamMenu = false }}
          role="menuitem"
        >
          <Globe class="w-4 h-4" />
          Browse Teams
        </button>
        <div class="my-1 border-t border-border-1"></div>
        <button
          class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard"
          onclick={() => { dispatch('teamSettings'); showTeamMenu = false }}
          role="menuitem"
        >
          <Settings class="w-4 h-4" />
          Team Settings
        </button>
        <button
          class="flex items-center gap-2 w-full px-3 py-2 text-sm text-danger hover:bg-danger/5 transition-standard"
          onclick={() => { /* leave team */ showTeamMenu = false }}
          role="menuitem"
        >
          <LogOut class="w-4 h-4" />
          Leave Team
        </button>
      </div>
    {/if}
  </div>

  <!-- Scrollable Channel List -->
  <nav class="flex-1 overflow-y-auto custom-scrollbar-thin p-3" aria-label="Channels">
    {#if favoriteChannels.length > 0}
      {@render categorySection('favorites', 'Favorites', favoriteChannels)}
    {/if}
    {@render categorySection('channels', 'Channels', publicChannels)}
    {@render categorySection('private', 'Private Channels', privateChannels)}
    {@render categorySection('dms', 'Direct Messages', dmChannels)}
  </nav>

  <!-- Members -->
  <section class="border-t border-border-1 p-4" aria-label="Members">
    <h3 class="text-xs font-semibold uppercase tracking-[0.16em] text-text-3">Members</h3>
    <ul class="mt-3 space-y-2 text-sm text-text-2">
      {#each members as member (member.id ?? member.user_id ?? member.username)}
        <li class="flex items-center gap-2">
          <span class="h-2 w-2 rounded-full bg-success" aria-hidden="true"></span>
          <span>{member.displayName ?? member.display_name ?? member.username}</span>
        </li>
      {/each}
    </ul>
  </section>

  <!-- Footer Actions -->
  <div class="border-t border-border-1 pt-2 pb-1 px-3 space-y-0.5">
    {#if hasAnyUnread}
      <button class="flex items-center gap-2 w-full px-2 py-1.5 text-xs text-text-3 hover:text-text-2 hover:bg-bg-surface-1 rounded transition-standard" onclick={() => dispatch('markAllRead')}>
        <Check class="w-3.5 h-3.5" />
        Mark all as read
      </button>
    {/if}
    <button class="flex items-center gap-2 w-full px-2 py-1.5 text-xs text-text-3 hover:text-text-2 hover:bg-bg-surface-1 rounded transition-standard" onclick={() => dispatch('browseChannels')}>
      <Globe class="w-3.5 h-3.5" />
      Browse channels
    </button>
    <button class="flex items-center gap-2 w-full px-2 py-1.5 text-xs text-text-3 hover:text-text-2 hover:bg-bg-surface-1 rounded transition-standard" onclick={() => dispatch('createChannel')}>
      <Plus class="w-3.5 h-3.5" />
      Create channel
    </button>
  </div>

  <!-- User Menu -->
  <div class="border-t border-border-1 p-3">
    <UserMenu on:setStatus={() => dispatch('setStatus')} on:editProfile={() => dispatch('editProfile')} />
  </div>
</aside>

{#if contextMenu}
  <!-- Backdrop -->
  <div class="fixed inset-0 z-40" onclick={closeContextMenu} role="presentation"></div>
  <!-- Menu -->
  <div
    class="fixed z-50 w-48 bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-2 py-1"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
    role="menu"
    transition:fly={{ duration: 150, y: -5 }}
  >
    <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard" onclick={() => { if (contextMenu) { markChannelRead(contextMenu.channel.id); closeContextMenu() } }} role="menuitem">
      <Check class="w-4 h-4" />
      Mark as Read
    </button>
    <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-text-1 hover:bg-bg-surface-2 transition-standard" onclick={() => { /* mute */ closeContextMenu() }} role="menuitem">
      <BellOff class="w-4 h-4" />
      Mute Channel
    </button>
    <div class="my-1 border-t border-border-1"></div>
    <button class="flex items-center gap-2 w-full px-3 py-2 text-sm text-danger hover:bg-danger/5 transition-standard" onclick={() => { /* leave */ closeContextMenu() }} role="menuitem">
      <LogOut class="w-4 h-4" />
      Leave Channel
    </button>
  </div>
{/if}
