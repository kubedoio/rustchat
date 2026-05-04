<script lang="ts">
  import { onMount } from 'svelte'
  import ChatSidebar from '../../components/chat/ChatSidebar.svelte'
  import MessageComposer from '../../components/chat/MessageComposer.svelte'
  import MessageList from '../../components/chat/MessageList.svelte'
  import { chatStore } from '../../stores/chat'

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

  onMount(() => {
    void chatStore.bootstrap()
  })

  async function sendMessage(event: CustomEvent<{ content: string; file_ids?: string[] }>) {
    if (!currentChannel) {
      return
    }

    await chatStore.sendMessage(currentChannel.id, event.detail.content, event.detail.file_ids ?? [])
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
    <header class="flex h-[var(--header-height)] items-center border-b border-border-1 px-5">
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.14em] text-text-3">Channel</p>
        <h1 class="text-lg font-semibold text-text-1">
          {#if currentChannel}
            #{currentChannel.display_name || currentChannel.name}
          {:else if $chatStore.loading}
            Loading channels...
          {:else}
            No channel selected
          {/if}
        </h1>
      </div>
    </header>

    {#if $chatStore.error}
      <div class="border-b border-danger/30 bg-danger/10 px-5 py-3 text-sm text-danger" role="alert">
        {$chatStore.error}
      </div>
    {/if}

    <MessageList messages={currentMessages} />

    {#if currentChannel}
      <MessageComposer
        channelId={currentChannel.id}
        channelName={currentChannel.display_name || currentChannel.name}
        members={currentMembers}
        on:send={sendMessage}
      />
    {/if}
  </section>
</main>
