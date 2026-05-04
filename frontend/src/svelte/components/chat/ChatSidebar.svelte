<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import type { ChatChannel, ChatMember, ChatTeam } from './types'

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
  export let channels: ChatChannel[] = []
  export let activeChannelId = 'general'
  export let currentChannelId: string | null | undefined = activeChannelId
  export let members: ChatMember[] = [
    { id: 'adam', username: 'adam', displayName: 'Adam' },
    { id: 'member', username: 'member', displayName: 'Member' }
  ]
  export let onSelectChannel: ((channelId: string) => void | Promise<void>) | undefined = undefined

  const dispatch = createEventDispatcher<{
    selectChannel: ChatChannel
  }>()

  $: selectedChannelId = currentChannelId ?? activeChannelId

  function teamName(team: ChatTeam) {
    return team.displayName ?? team.display_name ?? team.name
  }

  function channelName(channel: ChatChannel) {
    return channel.displayName ?? channel.display_name ?? channel.name
  }

  function channelsForTeam(team: ChatTeam) {
    if (team.channels?.length) return team.channels
    return channels.filter((channel) => !channel.team_id || channel.team_id === team.id)
  }

  function selectChannel(channel: ChatChannel) {
    void onSelectChannel?.(channel.id)
    dispatch('selectChannel', channel)
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
        <h3 class="px-2 text-xs font-semibold uppercase tracking-[0.16em] text-slate-400">{teamName(team)}</h3>
        <div class="mt-2 space-y-1">
          {#each channelsForTeam(team) as channel (channel.id)}
            <button
              type="button"
              class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${channel.id === selectedChannelId ? 'bg-white text-slate-950' : 'text-slate-200 hover:bg-white/10'}`}
              aria-current={channel.id === selectedChannelId ? 'page' : undefined}
              on:click={() => selectChannel(channel)}
            >
              <span># {channelName(channel)}</span>
              {#if channel.unreadCount}
                <span class="rounded-full bg-indigo-500 px-2 py-0.5 text-xs font-semibold text-white">{channel.unreadCount}</span>
              {/if}
            </button>
          {/each}
        </div>
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
</aside>
