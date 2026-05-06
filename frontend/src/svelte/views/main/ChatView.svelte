<script lang="ts">
  import { flushSync, onMount } from 'svelte'
  import { get } from 'svelte/store'
  import GlobalHeader from '../../components/layout/GlobalHeader.svelte'
  import TeamRail from '../../components/layout/TeamRail.svelte'
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
  import AddChannelMembersModal from '../../components/modals/AddChannelMembersModal.svelte'
  import CreateTeamModal from '../../components/modals/CreateTeamModal.svelte'
  import EditProfileModal from '../../components/modals/EditProfileModal.svelte'
  import CommandPalette from '../../components/ui/CommandPalette.svelte'
  import { chatStore } from '../../stores/chat'
  import type { SvelteChatChannel, SvelteChatMember, SvelteChatPost } from '../../stores/chat'
  import { uiStore } from '../../stores/ui'
  import { quickSwitcherStore } from '../../stores/quickSwitcher'
  import { activityStore } from '../../stores/activity'
  import { authStore } from '../../stores/auth'
  import { connectionStatus, retryConnection, type ConnectionStatus } from '../../stores/websocket'
  import { callsStore, registerCallWebSocketHandlers } from '../../stores/calls.svelte'

  const currentChannel = $derived(
    $chatStore.channels.find((channel) => channel.id === $chatStore.currentChannelId) ?? null,
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
  let infoPanelOpen = $state(false)
  let profileUserId = $state<string | null>(null)
  let threadPanelOpen = $state(false)
  let activeThreadId = $state<string | null>(null)
  let searchOpen = $state(false)
  let createChannelOpen = $state(false)
  let browseChannelsOpen = $state(false)
  let dmOpen = $state(false)
  let setStatusOpen = $state(false)
  let addMembersOpen = $state(false)
  let createTeamOpen = $state(false)
  let editProfileOpen = $state(false)
  let commandPaletteOpen = $state(false)
  let requestedChannelLoad = $state<string | null>(null)
  let currentConnectionStatus = $state<ConnectionStatus>('connecting')
  let visibleMessages = $state.raw<SvelteChatPost[]>([])
  let messageListVersion = $state(0)
  let messageListRef: MessageList | null = $state(null)

  // Mobile drawer state
  let mobileSidebarOpen = $state(false)
  let isMobile = $state(false)

  function checkMobile() {
    isMobile = window.innerWidth < 768
  }

  function closeMobileSidebar() {
    mobileSidebarOpen = false
  }

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
    checkMobile()
    window.addEventListener('resize', checkMobile)

    const unsubscribeConnectionStatus = connectionStatus.subscribe((status) => {
      currentConnectionStatus = status
    })
    const unsubscribeChat = chatStore.subscribe((state) => {
      const channelId = state.currentChannelId
      const nextMessages = channelId ? (state.messagesByChannel[channelId] ?? []) : []
      if (nextMessages !== visibleMessages) {
        flushSync(() => {
          visibleMessages = nextMessages
          messageListVersion += 1
        })
      }
    })

    void bootstrapChat()
    registerCallWebSocketHandlers()

    return () => {
      window.removeEventListener('resize', checkMobile)
      unsubscribeConnectionStatus()
      unsubscribeChat()
    }
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

  function handleJumpToMessage(messageId: string) {
    uiStore.closeRhs()
    messageListRef?.scrollToMessage(messageId)
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'k' && (event.metaKey || event.ctrlKey) && event.shiftKey) {
      event.preventDefault()
      commandPaletteOpen = true
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex flex-col h-screen overflow-hidden bg-bg-app text-text-1">
  <GlobalHeader />

  <div class="flex flex-1 overflow-hidden relative">
    <!-- Mobile sidebar overlay -->
    {#if isMobile && mobileSidebarOpen}
      <div class="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm" onclick={closeMobileSidebar} onkeydown={(e) => e.key === 'Escape' && closeMobileSidebar()} role="presentation" tabindex="-1"></div>
      <div class="fixed top-0 left-0 bottom-0 z-50 flex shadow-2xl">
        <TeamRail
          onSelectTeam={(id) => { chatStore.selectTeam(id); closeMobileSidebar() }}
          onCreateTeam={() => { createTeamOpen = true; closeMobileSidebar() }}
        />
        <ChatSidebar
          teams={$chatStore.teams}
          channels={$chatStore.channels}
          members={sidebarMembers}
          currentUserId={$authStore.user?.id}
          unreadCounts={$chatStore.unreadCounts}
          currentChannelId={$chatStore.currentChannelId}
          onSelectChannel={(channelId) => { chatStore.selectChannel(channelId); closeMobileSidebar() }}
          on:createChannel={() => (createChannelOpen = true)}
          on:browseChannels={() => (browseChannelsOpen = true)}
          on:directMessage={() => (dmOpen = true)}
          on:setStatus={() => (setStatusOpen = true)}
          on:createTeam={() => (createTeamOpen = true)}
          on:editProfile={() => (editProfileOpen = true)}
        />
      </div>
    {/if}

    <!-- Desktop TeamRail -->
    {#if !isMobile}
      <TeamRail
        onSelectTeam={(id) => chatStore.selectTeam(id)}
        onCreateTeam={() => (createTeamOpen = true)}
      />
    {/if}

    <!-- Desktop Sidebar -->
    {#if !isMobile}
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
        on:createTeam={() => (createTeamOpen = true)}
        on:editProfile={() => (editProfileOpen = true)}
      />
    {/if}

    <!-- Main content -->
    <section class="flex min-w-0 flex-1 flex-col bg-bg-surface-1">
      <ConnectionStatusBar />

      <ChannelHeader
        channel={currentChannel}
        members={currentMembers}
        onToggleInfo={toggleInfoPanel}
        onToggleMembers={() => uiStore.setRhsView('members')}
        onToggleMobileSidebar={() => (mobileSidebarOpen = !mobileSidebarOpen)}
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

      <div
        data-testid="main-content"
        class="flex min-w-0 flex-1 flex-col transition-opacity duration-300"
        class:opacity-80={currentConnectionStatus === 'reconnecting'}
        class:opacity-60={currentConnectionStatus === 'disconnected'}
        class:blur-sm={currentConnectionStatus === 'failed'}
      >
        {#key `${$chatStore.currentChannelId ?? 'none'}:${messageListVersion}`}
          <MessageList
            bind:this={messageListRef}
            messages={visibleMessages}
            channelId={$chatStore.currentChannelId}
            on:openProfile={(e) => { profileUserId = e.detail }}
            on:thread={handleThread}
          />
        {/key}

        {#if currentChannel}
          <TypingIndicator channelId={currentChannel.id} />
          <MessageComposer
            channelId={currentChannel.id}
            channelName={currentChannel.display_name || currentChannel.name}
            members={currentMembers}
            disabled={currentConnectionStatus !== 'connected'}
            on:send={sendMessage}
          />
        {/if}
      </div>
    </section>

    <!-- RHS panels (desktop) -->
    {#if !isMobile}
      {#if infoPanelOpen}
        <ChannelInfoPanel
          channel={currentChannel}
          members={currentMembers}
          open={infoPanelOpen}
          on:close={() => (infoPanelOpen = false)}
        />
      {/if}

      {#if threadPanelOpen && activeThreadId}
        <ThreadPanel
          threadId={activeThreadId}
          channelId={currentChannel?.id ?? null}
          open={threadPanelOpen}
          on:close={() => { threadPanelOpen = false; activeThreadId = null }}
        />
      {/if}

      {#if $uiStore.rhsView === 'pinned' && currentChannel}
        <PinnedMessagesPanel channelId={currentChannel.id} open={true} on:close={() => uiStore.closeRhs()} on:jump={(e) => handleJumpToMessage(e.detail)} />
      {/if}

      {#if $uiStore.rhsView === 'saved'}
        <SavedMessagesPanel open={true} on:close={() => uiStore.closeRhs()} on:jump={(e) => handleJumpToMessage(e.detail)} />
      {/if}
    {/if}
  </div>

  {#if profileUserId}
    <UserProfileModal
      userId={profileUserId}
      open={true}
      on:close={() => (profileUserId = null)}
      on:message={() => (profileUserId = null)}
    />
  {/if}

  {#if currentConnectionStatus === 'failed'}
    <ConnectionLostModal
      open={true}
      on:reconnect={() => retryConnection()}
      on:refresh={() => window.location.reload()}
    />
  {/if}

  {#if $uiStore.isSettingsOpen}
    <SettingsModal open={true} on:close={uiStore.closeSettings} />
  {/if}

  <ActivityFeed />

  {#if $quickSwitcherStore.open}
    <QuickSwitcherModal
      open={true}
      on:close={quickSwitcherStore.close}
      on:select={handleQuickSelect}
    />
  {/if}

  <IncomingCallModal />
  <ActiveCall />

  <CommandPalette open={commandPaletteOpen} on:close={() => (commandPaletteOpen = false)} on:select={() => (commandPaletteOpen = false)} />

  <CreateChannelModal open={createChannelOpen} on:close={() => (createChannelOpen = false)} />
  <BrowseChannelsModal open={browseChannelsOpen} on:close={() => (browseChannelsOpen = false)} />
  <DirectMessageModal open={dmOpen} on:close={() => (dmOpen = false)} on:select={(e) => { chatStore.selectChannel(e.detail); dmOpen = false }} />
  <SetStatusModal open={setStatusOpen} on:close={() => (setStatusOpen = false)} />
  <AddChannelMembersModal open={addMembersOpen} channelId={currentChannel?.id} channelName={currentChannel?.name} onclose={() => (addMembersOpen = false)} />
  <CreateTeamModal open={createTeamOpen} on:close={() => (createTeamOpen = false)} />
  <EditProfileModal open={editProfileOpen} on:close={() => (editProfileOpen = false)} />

  {#if searchOpen}
    <SearchModal open={true} on:close={handleSearchClose} />
  {/if}
</div>
