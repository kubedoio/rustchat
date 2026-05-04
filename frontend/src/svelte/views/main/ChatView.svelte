<script lang="ts">
  import { onMount } from 'svelte'
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
  import { chatStore } from '../../stores/chat'
  import { uiStore } from '../../stores/ui'
  import { quickSwitcherStore } from '../../stores/quickSwitcher'
  import { activityStore } from '../../stores/activity'
  import { connectionStatus, registerWebSocketHandlers, retryConnection } from '../../stores/websocket'
  import { callsStore, registerCallWebSocketHandlers } from '../../stores/calls.svelte'

  const currentChannel = $derived(
    $chatStore.channels.find((channel) => channel.id === $chatStore.currentChannelId) ?? null,
  )
  const currentMessages = $derived(
    currentChannel ? ($chatStore.messagesByChannel[currentChannel.id] ?? []) : [],
  )
  const currentMembers = $derived(
    currentChannel
      ? ($chatStore.membersByTeam[currentChannel.team_id] ?? [
          { user_id: 'adam', username: 'adam', display_name: 'Adam Builder' },
          { user_id: 'member', username: 'member', display_name: 'Member' },
        ])
      : [],
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

  onMount(() => {
    void chatStore.bootstrap()
    const cleanupWs = registerWebSocketHandlers()
    registerCallWebSocketHandlers()
    return cleanupWs
  })

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
    currentChannelId={$chatStore.currentChannelId}
    onSelectChannel={(channelId) => chatStore.selectChannel(channelId)}
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
</main>
