<script lang="ts">
  import { onMount } from 'svelte'
  import ChatSidebar from '../../components/chat/ChatSidebar.svelte'
  import ChannelHeader from '../../components/chat/ChannelHeader.svelte'
  import ChannelInfoPanel from '../../components/chat/ChannelInfoPanel.svelte'
  import MessageComposer from '../../components/chat/MessageComposer.svelte'
  import MessageList from '../../components/chat/MessageList.svelte'
  import ConnectionStatusBar from '../../components/ui/ConnectionStatusBar.svelte'
  import { chatStore } from '../../stores/chat'
  import { connectionStatus, registerWebSocketHandlers } from '../../stores/websocket'

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

  onMount(() => {
    void chatStore.bootstrap()
    const cleanupWs = registerWebSocketHandlers()
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
    />

    {#if $chatStore.error}
      <div class="border-b border-danger/30 bg-danger/10 px-5 py-3 text-sm text-danger" role="alert">
        {$chatStore.error}
      </div>
    {/if}

    <div data-testid="main-content" class="flex min-w-0 flex-1 flex-col transition-opacity duration-300 {contentOpacityClass}">
      <MessageList messages={currentMessages} />

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
</main>
