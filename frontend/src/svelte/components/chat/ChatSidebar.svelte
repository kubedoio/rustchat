<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { Hash, Lock, Plus, Globe, MessageCircle } from 'lucide-svelte'
  import type { ChatMember, ChatTeam } from './types'
  import type { SvelteChatChannel } from '../../stores/chat'
  import UserMenu from '../ui/UserMenu.svelte'

  interface SidebarChannel extends SvelteChatChannel {
    displayName?: string
    unreadCount?: number
  }

  export let teams: ChatTeam[] = [
    {
      id: 'rustchat',
      name: 'rustchat',
      displayName: 'RustChat',
      channels: [
        { id: 'general', name: 'general', displayName: 'general' },
        { id: 'random', name: 'random', displayName: 'random' }
      ]
    }
  ]
  export let channels: SidebarChannel[] = []
  export let unreadCounts: Record<string, number> = {}
  export let activeChannelId = 'general'
  export let currentChannelId: string | null | undefined = activeChannelId
  export let currentUserId: string | undefined = undefined
  export let members: ChatMember[] = [
    { id: 'adam', username: 'adam', displayName: 'Adam' },
    { id: 'member', username: 'member', displayName: 'Member' }
  ]
  export let onSelectChannel: ((channelId: string) => void | Promise<void>) | undefined = undefined

  const dispatch = createEventDispatcher<{
    selectChannel: SidebarChannel
    createChannel: void
    browseChannels: void
    directMessage: void
    setStatus: void
  }>()

  $: selectedChannelId = currentChannelId ?? activeChannelId

  function teamName(team: ChatTeam) {
    return team.displayName ?? team.display_name ?? team.name
  }

  function channelDisplayName(channel: SidebarChannel) {
    return channel.display_name ?? channel.displayName ?? channel.name
  }

  function channelsForTeam(team: ChatTeam): SidebarChannel[] {
    if (team.channels?.length) {
      return team.channels as SidebarChannel[]
    }
    return channels.filter((channel) => !channel.team_id || channel.team_id === team.id)
  }

  function regularChannelsForTeam(team: ChatTeam): SidebarChannel[] {
    return channelsForTeam(team).filter((c) => c.channel_type !== 'direct' && c.channel_type !== 'group')
  }

  function dmChannelsForTeam(team: ChatTeam): SidebarChannel[] {
    return channelsForTeam(team).filter((c) => c.channel_type === 'direct' || c.channel_type === 'group')
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
        return 'bg-emerald-400'
      case 'away':
        return 'bg-amber-400'
      case 'dnd':
        return 'bg-rose-400'
      default:
        return 'bg-slate-500'
    }
  }

  function unreadCount(channel: SidebarChannel): number {
    return unreadCounts[channel.id] ?? channel.unreadCount ?? 0
  }

  function memberForDm(channel: SidebarChannel): ChatMember | null {
    const normalizedName = channel.name.toLowerCase()
    const normalizedDisplayName = channelDisplayName(channel).toLowerCase()

    return members.find((member) => {
      if (currentUserId && (member.user_id ?? member.id) === currentUserId) return false
      const displayName = member.displayName ?? member.display_name ?? ''
      return (
        normalizedName.includes(member.username.toLowerCase()) ||
        normalizedDisplayName === displayName.toLowerCase() ||
        normalizedDisplayName.includes(member.username.toLowerCase())
      )
    }) ?? members.find((member) => !currentUserId || (member.user_id ?? member.id) !== currentUserId) ?? null
  }

  function memberStatusText(member: ChatMember | null): string {
    return member?.status_text ?? member?.statusText ?? ''
  }

  function memberStatusEmoji(member: ChatMember | null): string {
    return member?.status_emoji ?? member?.statusEmoji ?? ''
  }
</script>

<aside class="flex w-72 shrink-0 flex-col border-r border-gray-200 bg-slate-950 text-white" aria-label="Chat sidebar">
  <div class="border-b border-white/10 p-4">
    <p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">Teams</p>
    <h2 class="mt-1 text-lg font-semibold">{teams[0]?.displayName ?? teams[0]?.name ?? 'RustChat'}</h2>
  </div>

  <nav class="flex-1 overflow-y-auto p-3" aria-label="Channels">
    {#each teams as team (team.id)}
      <section class="mb-5" aria-label={teamName(team)}>
        <div class="flex items-center justify-between px-2">
          <h3 class="text-xs font-semibold uppercase tracking-[0.16em] text-slate-400">{teamName(team)}</h3>
          <div class="flex items-center gap-0.5">
            <button
              type="button"
              class="rounded p-1 text-slate-400 hover:text-white hover:bg-white/10 transition"
              title="Create channel"
              aria-label="Create channel"
              on:click={() => dispatch('createChannel')}
            >
              <Plus class="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              class="rounded p-1 text-slate-400 hover:text-white hover:bg-white/10 transition"
              title="Browse channels"
              aria-label="Browse channels"
              on:click={() => dispatch('browseChannels')}
            >
              <Globe class="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              class="rounded p-1 text-slate-400 hover:text-white hover:bg-white/10 transition"
              title="Direct message"
              aria-label="Direct message"
              on:click={() => dispatch('directMessage')}
            >
              <MessageCircle class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
        <div class="mt-2 space-y-1">
          <!-- Regular Channels -->
          {#each regularChannelsForTeam(team) as channel (channel.id)}
            <button
              type="button"
              data-testid="channel-sidebar-row"
              class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${channel.id === selectedChannelId ? 'bg-white text-slate-950' : 'text-slate-200 hover:bg-white/10'}`}
              aria-current={channel.id === selectedChannelId ? 'page' : undefined}
              on:click={() => selectChannel(channel)}
            >
              <span class="flex items-center gap-2">
                {#if channel.channel_type === 'private'}
                  <Lock class="w-3.5 h-3.5 shrink-0" />
                {:else}
                  <Hash class="w-4 h-4 shrink-0" />
                {/if}
                <span>{channelDisplayName(channel)}</span>
              </span>
              {#if unreadCount(channel)}
                <span data-testid="unread-badge" class="rounded-full bg-indigo-500 px-2 py-0.5 text-xs font-semibold text-white">{unreadCount(channel)}</span>
              {/if}
            </button>
          {/each}
        </div>

        <!-- Direct Messages -->
        {#each [dmChannelsForTeam(team)] as dms}
          {#if dms.length > 0}
            <h4 class="mt-4 px-2 text-xs font-semibold uppercase tracking-[0.16em] text-slate-400">Direct Messages</h4>
            <div class="mt-2 space-y-1">
              {#each dms as channel (channel.id)}
                {@const isSelected = channel.id === selectedChannelId}
                {@const member = memberForDm(channel)}
                {@const presence = member?.presence ?? 'offline'}
                {@const statusLabel = presenceLabel(presence)}
                {@const statusText = memberStatusText(member)}
                {@const statusEmoji = memberStatusEmoji(member)}
                <button
                  type="button"
                  data-testid="dm-sidebar-row"
                  class={`group flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${isSelected ? 'bg-white text-slate-950' : 'text-slate-200 hover:bg-white/10'}`}
                  aria-current={isSelected ? 'page' : undefined}
                  on:click={() => selectChannel(channel)}
                >
                  <span class="flex items-center gap-2 min-w-0">
                    <span class="relative shrink-0">
                      <div class="h-6 w-6 rounded-full bg-indigo-500/20 text-indigo-300 flex items-center justify-center text-[10px] font-bold">
                        {getInitials(channelDisplayName(channel))}
                      </div>
                      <span class="absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full ring-2 ring-slate-950 {presenceDotClass(presence)}" aria-hidden="true"></span>
                    </span>
                    <span class="min-w-0 flex flex-col">
                      <span class="truncate">{channelDisplayName(channel)}</span>
                      <span
                        data-testid="dm-sidebar-status"
                        class="mt-0.5 block truncate text-[11px] {isSelected ? 'text-slate-600' : 'text-slate-400'}"
                      >
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
                  </span>
                  {#if unreadCount(channel)}
                    <span data-testid="unread-badge" class="rounded-full bg-indigo-500 px-2 py-0.5 text-xs font-semibold text-white shrink-0 ml-2">{unreadCount(channel)}</span>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        {/each}
      </section>
    {/each}
  </nav>

  <section class="border-t border-white/10 p-4" aria-label="Members">
    <h3 class="text-xs font-semibold uppercase tracking-[0.16em] text-slate-400">Members</h3>
    <ul class="mt-3 space-y-2 text-sm text-slate-200">
      {#each members as member (member.id)}
        <li class="flex items-center gap-2">
          <span class="h-2 w-2 rounded-full bg-emerald-400" aria-hidden="true"></span>
          <span>{member.displayName ?? member.display_name ?? member.username}</span>
        </li>
      {/each}
    </ul>
  </section>

  <div class="border-t border-white/10 p-3">
    <UserMenu on:setStatus={() => dispatch('setStatus')} />
  </div>
</aside>
