<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import ChatSidebar from '../../components/chat/ChatSidebar.svelte'
  import ChannelHeader from '../../components/chat/ChannelHeader.svelte'
  import ChannelInfoPanel from '../../components/chat/ChannelInfoPanel.svelte'
  import MessageComposer from '../../components/chat/MessageComposer.svelte'
  import MessageList from '../../components/chat/MessageList.svelte'
  import UserProfileModal from '../../components/modals/UserProfileModal.svelte'
  import ConnectionStatusBar from '../../components/ui/ConnectionStatusBar.svelte'
  import ConnectionLostModal from '../../components/ui/ConnectionLostModal.svelte'
  import SettingsModal from '../../components/settings/SettingsModal.svelte'
  import ThreadPanel from '../../components/thread/ThreadPanel.svelte'
  import SearchModal from '../../components/search/SearchModal.svelte'
  import QuickSwitcherModal from '../../components/search/QuickSwitcherModal.svelte'
  import ActivityFeed from '../../components/activity/ActivityFeed.svelte'
  import IncomingCallModal from '../../components/calls/IncomingCallModal.svelte'
  import ActiveCall from '../../components/calls/ActiveCall.svelte'
  import PinnedMessagesPanel from '../../components/chat/PinnedMessagesPanel.svelte'
  import SavedMessagesPanel from '../../components/chat/SavedMessagesPanel.svelte'
  import TypingIndicator from '../../components/chat/TypingIndicator.svelte'
  import CreateChannelModal from '../../components/modals/CreateChannelModal.svelte'
  import BrowseChannelsModal from '../../components/modals/BrowseChannelsModal.svelte'
  import DirectMessageModal from '../../components/modals/DirectMessageModal.svelte'
  import SetStatusModal from '../../components/modals/SetStatusModal.svelte'
  import { chatStore } from '../../stores/chat'
  import type { SvelteChatChannel, SvelteChatMember } from '../../stores/chat'
  import { uiStore } from '../../stores/ui'
  import { quickSwitcherStore } from '../../stores/quickSwitcher'
  import { activityStore } from '../../stores/activity'
  import { authStore } from '../../stores/auth'
  import { connectionStatus, retryConnection } from '../../stores/websocket'
  import { callsStore, registerCallWebSocketHandlers } from '../../stores/calls.svelte'

  const currentChannel = $derived(
    $chatStore.channels.find((channel) => channel.id === $chatStore.currentChannelId) ?? null,
  )
  const currentMessages = $derived(
    currentChannel ? ($chatStore.messagesByChannel[currentChannel.id] ?? []) : [],
  )
  const currentMembers = $derived(
    currentChannel
      ? currentChannel.channel_type === 'direct'
        ? resolveDirectMembers(
            currentChannel,
            $chatStore.membersByTeam[currentChannel.team_id] ?? [],
            $authStore.user?.id,
          )
        : ($chatStore.membersByTeam[currentChannel.team_id] ?? [
          { user_id: 'adam', username: 'adam', display_name: 'Adam Builder' },
          { user_id: 'member', username: 'member', display_name: 'Member' },
        ])
      : [],
  )
  const sidebarMembers = $derived(
    $chatStore.teams.flatMap((team) => $chatStore.membersByTeam[team.id] ?? []),
  )
  const isDisconnected = $derived($connectionStatus !== 'connected')
  const contentOpacityClass = $derived(
    $connectionStatus === 'failed'
      ? 'blur-sm'
      : $connectionStatus === 'disconnected'
        ? 'opacity-60'
        : $connectionStatus === 'reconnecting'
          ? 'opacity-80'
          : '',
  )

  let infoPanelOpen = $state(false)
  let profileUserId = $state<string | null>(null)
  let threadPanelOpen = $state(false)
  let activeThreadId = $state<string | null>(null)
  let searchOpen = $state(false)
  let createChannelOpen = $state(false)
  let browseChannelsOpen = $state(false)
  let dmOpen = $state(false)
  let setStatusOpen = $state(false)
  let requestedChannelLoad = $state<string | null>(null)

  function resolveDirectMembers(
    channel: SvelteChatChannel,
    members: SvelteChatMember[],
    currentUserId: string | undefined,
  ) {
    const counterparty = members.find((member) => {
      if (currentUserId && member.user_id === currentUserId) return false
      const displayName = member.display_name ?? ''
      return (
        channel.name.toLowerCase().includes(member.username.toLowerCase()) ||
        channel.display_name.toLowerCase() === displayName.toLowerCase() ||
        channel.display_name.toLowerCase().includes(member.username.toLowerCase())
      )
    })

    return counterparty
      ? [counterparty]
      : members.filter((member) => !currentUserId || member.user_id !== currentUserId).slice(0, 1)
  }

  onMount(() => {
    void bootstrapChat()
    registerCallWebSocketHandlers()
  })

  $effect(() => {
    const channelId = $chatStore.currentChannelId
    if (!channelId || requestedChannelLoad === channelId || $chatStore.messagesByChannel[channelId]) {
      return
    }

    requestedChannelLoad = channelId
    void chatStore.fetchMessages(channelId).finally(() => {
      if (requestedChannelLoad === channelId) {
        requestedChannelLoad = null
      }
    })
  })

  async function bootstrapChat() {
    await chatStore.bootstrap()

    const channelId = get(chatStore).currentChannelId
    if (channelId && window.location.pathname === '/') {
      window.history.replaceState({}, '', `/channels/${channelId}`)
      window.dispatchEvent(new PopStateEvent('popstate'))
    }
  }

  async function sendMessage(event: CustomEvent<{ content: string; file_ids?: string[] }>) {
    if (!currentChannel) {
      return
    }

    await chatStore.sendMessage(currentChannel.id, event.detail.content, event.detail.file_ids ?? [])
  }

  function toggleInfoPanel() {
    infoPanelOpen = !infoPanelOpen
  }

  function handleThread(event: CustomEvent<{ messageId: string; channelId: string }>) {
    activeThreadId = event.detail.messageId
    threadPanelOpen = true
  }

  function handleQuickSelect(event: CustomEvent<{ id: string; type: string }>) {
    const item = event.detail
    if (item.type === 'channel' || item.type === 'dm') {
      chatStore.selectChannel(item.id)
    }
    quickSwitcherStore.close()
  }

  function handleSearchClose() {
    searchOpen = false
  }
</script>

<main class="flex h-screen overflow-hidden bg-bg-app text-text-1">
  <ChatSidebar
    teams={$chatStore.teams}
    channels={$chatStore.channels}
    members={sidebarMembers}
    currentUserId={$authStore.user?.id}
    unreadCounts={$chatStore.unreadCounts}
    currentChannelId={$chatStore.currentChannelId}
    onSelectChannel={(channelId) => chatStore.selectChannel(channelId)}
    on:createChannel={() => (createChannelOpen = true)}
    on:browseChannels={() => (browseChannelsOpen = true)}
    on:directMessage={() => (dmOpen = true)}
    on:setStatus={() => (setStatusOpen = true)}
  />

  <section class="flex min-w-0 flex-1 flex-col bg-bg-surface-1">
    <ConnectionStatusBar />

    <ChannelHeader
      channel={currentChannel}
      members={currentMembers}
      onToggleInfo={toggleInfoPanel}
      on:search={() => (searchOpen = true)}
      on:toggleActivity={() => activityStore.toggleFeed()}
      on:togglePinned={() => uiStore.toggleRhs('pinned')}
      on:toggleSaved={() => uiStore.toggleRhs('saved')}
      on:startCall={() => { if (currentChannel) void callsStore.startCall(currentChannel.id) }}
    />

    {#if $chatStore.error}
      <div class="border-b border-danger/30 bg-danger/10 px-5 py-3 text-sm text-danger" role="alert">
        {$chatStore.error}
      </div>
    {/if}

    <div data-testid="main-content" class="flex min-w-0 flex-1 flex-col transition-opacity duration-300 {contentOpacityClass}">
      <MessageList messages={currentMessages} on:openProfile={(e) => { profileUserId = e.detail }} on:thread={handleThread} />

      {#if currentChannel}
        <TypingIndicator channelId={currentChannel.id} />
        <MessageComposer
          channelId={currentChannel.id}
          channelName={currentChannel.display_name || currentChannel.name}
          members={currentMembers}
          disabled={isDisconnected}
          on:send={sendMessage}
        />
      {/if}
    </div>
  </section>

  {#if infoPanelOpen}
    <ChannelInfoPanel
      channel={currentChannel}
      members={currentMembers}
      open={infoPanelOpen}
      on:close={() => (infoPanelOpen = false)}
    />
  {/if}

  {#if profileUserId}
    <UserProfileModal
      userId={profileUserId}
      open={true}
      on:close={() => (profileUserId = null)}
      on:message={() => (profileUserId = null)}
    />
  {/if}

  {#if $connectionStatus === 'failed'}
    <ConnectionLostModal
      open={true}
      on:reconnect={() => retryConnection()}
      on:refresh={() => window.location.reload()}
    />
  {/if}

  {#if $uiStore.isSettingsOpen}
    <SettingsModal open={true} on:close={uiStore.closeSettings} />
  {/if}

  {#if threadPanelOpen && activeThreadId}
    <ThreadPanel
      threadId={activeThreadId}
      channelId={currentChannel?.id ?? null}
      open={threadPanelOpen}
      on:close={() => { threadPanelOpen = false; activeThreadId = null }}
    />
  {/if}

  {#if searchOpen}
    <SearchModal open={true} on:close={handleSearchClose} />
  {/if}

  <ActivityFeed />

  {#if $uiStore.rhsView === 'pinned' && currentChannel}
    <PinnedMessagesPanel channelId={currentChannel.id} open={true} on:close={() => uiStore.closeRhs()} on:jump={() => { /* TODO: scroll to message */ }} />
  {/if}

  {#if $uiStore.rhsView === 'saved'}
    <SavedMessagesPanel open={true} on:close={() => uiStore.closeRhs()} on:jump={() => { /* TODO: scroll to message */ }} />
  {/if}

  {#if $quickSwitcherStore.open}
    <QuickSwitcherModal
      open={true}
      on:close={quickSwitcherStore.close}
      on:select={handleQuickSelect}
    />
  {/if}

  <IncomingCallModal />
  <ActiveCall />

  <CreateChannelModal open={createChannelOpen} on:close={() => (createChannelOpen = false)} />
  <BrowseChannelsModal open={browseChannelsOpen} on:close={() => (browseChannelsOpen = false)} />
  <DirectMessageModal open={dmOpen} on:close={() => (dmOpen = false)} on:select={(e) => { chatStore.selectChannel(e.detail); dmOpen = false }} />
  <SetStatusModal open={setStatusOpen} on:close={() => (setStatusOpen = false)} />
</main>
