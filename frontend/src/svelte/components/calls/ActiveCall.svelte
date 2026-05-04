<script lang="ts">
  import { callsStore } from '../../stores/calls.svelte'
  import { authStore } from '../../stores/auth'
  import { chatStore } from '../../stores/chat'
  import {
    Maximize2,
    Minimize2,
    Mic,
    MicOff,
    PhoneOff,
    Hand,
    Monitor,
    MoreVertical,
    Users,
    Bell,
    Trash2,
    Shield,
  } from 'lucide-svelte'

  let showParticipants = $state(false)
  let showMenu = $state(false)
  let participantMenuOpen = $state<string | null>(null)
  let screenVideoRef = $state<HTMLVideoElement | null>(null)

  let activeCall = $derived(callsStore.currentCall)
  let isExpanded = $derived(callsStore.isExpanded)
  let isMuted = $derived(callsStore.isMuted)
  let isHandRaised = $derived(callsStore.isHandRaised)
  let isScreenSharing = $derived(callsStore.isScreenSharing)
  let participants = $derived(callsStore.currentCallParticipants)
  let speakingParticipants = $derived(callsStore.speakingParticipants)

  let screenShareStream = $derived.by(() => {
    if (!activeCall) return null
    if (activeCall.screenStream) return activeCall.screenStream
    const remoteStreams = Array.from(activeCall.remoteStreams.values())
    const screenSessionId = activeCall.call.screen_sharing_session_id
    if (screenSessionId) {
      const matched = remoteStreams.find((stream) => {
        if (!stream.getVideoTracks().length) return false
        if (stream.id.includes(screenSessionId)) return true
        return stream.getVideoTracks().some((track) => track.id.includes(screenSessionId))
      })
      if (matched) return matched
    }
    return remoteStreams.find((stream) => stream.getVideoTracks().length > 0) || null
  })

  let channelName = $derived.by(() => {
    if (!activeCall) return ''
    const channel = $chatStore.channels.find((c) => c.id === activeCall.channelId)
    return channel?.name || 'Unknown Channel'
  })

  let isHost = $derived.by(() => {
    if (!activeCall || !$authStore.user) return false
    return activeCall.call.host_id === $authStore.user.id || activeCall.call.owner_id === $authStore.user.id
  })

  $effect(() => {
    if (screenVideoRef && screenShareStream) {
      screenVideoRef.srcObject = screenShareStream
    }
  })

  function toggleExpand() {
    callsStore.toggleExpanded()
  }

  function handleHangup() {
    callsStore.leaveCall()
  }

  function handleEndCall() {
    callsStore.endCall()
  }

  function toggleMute() {
    callsStore.toggleMute()
  }

  function toggleHand() {
    callsStore.toggleHand()
  }

  function toggleScreenShare() {
    callsStore.toggleScreenShare()
  }

  function handleRingAll() {
    if (activeCall) {
      callsStore.ring(activeCall.channelId)
    }
  }

  function handleMuteAll() {
    callsStore.hostMuteOthers()
  }

  function handleHostMute(sessionId: string) {
    callsStore.hostMute(sessionId)
    participantMenuOpen = null
  }

  function handleHostRemove(sessionId: string) {
    callsStore.hostRemove(sessionId)
    participantMenuOpen = null
  }

  function formatDuration(startAt: number) {
    const elapsed = Math.floor((Date.now() - startAt) / 1000)
    const minutes = Math.floor(elapsed / 60)
    const seconds = elapsed % 60
    return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
  }
</script>

{#if activeCall}
  <div
    data-testid="active-call"
    class="fixed transition-all duration-300 bg-bg-surface-1 border border-border-1 shadow-2xl rounded-xl overflow-hidden z-50 flex flex-col"
    class:inset-4={isExpanded}
    class:bottom-4={!isExpanded}
    class:right-4={!isExpanded}
    class:w-80={!isExpanded}
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 bg-app border-b border-border-1 shrink-0">
      <div class="flex items-center space-x-3 min-w-0">
        <span class="w-2.5 h-2.5 rounded-full bg-success animate-pulse shrink-0"></span>
        <div class="min-w-0">
          <h3 class="text-text-1 font-medium text-sm truncate">{channelName}</h3>
          <p class="text-xs text-text-3">
            {participants.length} participant{participants.length !== 1 ? 's' : ''}
            • {formatDuration(activeCall.call.start_at)}
          </p>
        </div>
      </div>
      <div class="flex items-center space-x-1 shrink-0">
        <button
          onclick={toggleExpand}
          class="p-1.5 text-text-3 hover:text-text-1 rounded hover:bg-bg-surface-1 transition-colors"
          title={isExpanded ? 'Minimize' : 'Maximize'}
        >
          {#if isExpanded}
            <Minimize2 class="w-4 h-4" />
          {:else}
            <Maximize2 class="w-4 h-4" />
          {/if}
        </button>
      </div>
    </div>

    <!-- Expanded Mode -->
    {#if isExpanded}
      <div class="flex-1 overflow-hidden flex">
        <!-- Main Area -->
        <div class="flex-1 bg-app flex items-center justify-center relative overflow-hidden">
          {#if screenShareStream}
            <div class="absolute inset-0 flex items-center justify-center bg-app">
              <video
                bind:this={screenVideoRef}
                autoplay
                playsinline
                class="max-w-full max-h-full object-contain"
              ></video>
              {#if activeCall.screenStream}
                <div class="absolute top-4 left-4 bg-brand px-3 py-1 rounded text-xs font-medium text-text-1 shadow-lg">
                  You are sharing your screen
                </div>
              {/if}
            </div>
          {:else}
            <div class="text-center">
              <div class="w-24 h-24 rounded-full bg-bg-surface-2 flex items-center justify-center mb-4 mx-auto">
                <Users class="w-12 h-12 text-text-3" />
              </div>
              <p class="text-text-3 text-sm">Audio Call in Progress</p>
              <p class="text-text-3 text-xs mt-1">{participants.length} participants</p>
            </div>
          {/if}
        </div>

        <!-- Participants Sidebar -->
        {#if showParticipants}
          <div class="w-64 bg-bg-surface-1 border-l border-border-1 overflow-y-auto">
            <div class="p-3 border-b border-border-1">
              <h4 class="text-text-1 font-medium text-sm">Participants</h4>
            </div>
            <div class="p-2 space-y-1">
              {#each participants as participant (participant.session_id)}
                <div
                  class="flex items-center space-x-2 p-2 rounded hover:bg-bg-surface-2 relative group"
                >
                  <div
                    class="w-8 h-8 rounded-full flex items-center justify-center transition-all duration-300 {speakingParticipants.has(participant.session_id) ? 'bg-success/20 ring-2 ring-success' : 'bg-brand/20'}"
                  >
                    <span class="text-brand text-xs font-medium">
                      {participant.user_id.slice(0, 2).toUpperCase()}
                    </span>
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center space-x-1">
                      <p class="text-text-1 text-sm truncate">
                        {(participant.user_id_raw || participant.user_id) === $authStore.user?.id
                          ? 'You'
                          : (participant.display_name || participant.username || participant.user_id.slice(0, 8))}
                      </p>
                      {#if (participant.user_id_raw || participant.user_id) === activeCall.call.host_id_raw || participant.user_id === activeCall.call.host_id}
                        <span title="Host"><Shield class="w-3 h-3 text-brand" /></span>
                      {/if}
                    </div>
                  </div>
                  <div class="flex items-center space-x-1">
                    {#if !participant.unmuted}
                      <MicOff class="w-3.5 h-3.5 text-text-3" />
                    {/if}
                    {#if participant.raised_hand > 0}
                      <Hand class="w-3.5 h-3.5 text-warning" />
                    {/if}

                    <!-- Participant Moderation Menu -->
                    {#if isHost && participant.user_id !== $authStore.user?.id}
                      <div class="relative ml-1">
                        <button
                          onclick={() => {
                            participantMenuOpen =
                              participantMenuOpen === participant.session_id ? null : participant.session_id
                          }}
                          class="p-1 text-text-3 hover:text-text-1 rounded hover:bg-bg-surface-1 opacity-0 group-hover:opacity-100 transition-opacity"
                        >
                          <MoreVertical class="w-3.5 h-3.5" />
                        </button>

                        {#if participantMenuOpen === participant.session_id}
                          <div
                            class="absolute right-0 top-full mt-1 w-32 bg-bg-surface-2 border border-border-1 rounded shadow-xl z-50 py-1"
                          >
                            <button
                              onclick={() => handleHostMute(participant.session_id)}
                              class="w-full px-3 py-1.5 text-left text-xs text-text-2 hover:bg-bg-surface-2 flex items-center"
                            >
                              <MicOff class="w-3 h-3 mr-2" />
                              Mute
                            </button>
                            <button
                              onclick={() => handleHostRemove(participant.session_id)}
                              class="w-full px-3 py-1.5 text-left text-xs text-danger hover:bg-bg-surface-2 flex items-center"
                            >
                              <Trash2 class="w-3 h-3 mr-2" />
                              Remove
                            </button>
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <!-- Compact Mode -->
      <div class="flex-1 bg-app p-3 overflow-hidden">
        <div class="flex items-center space-x-2">
          {#each participants.slice(0, 5) as participant, idx (participant.session_id)}
            <div
              class="w-10 h-10 rounded-full flex items-center justify-center shrink-0 transition-all duration-300 {speakingParticipants.has(participant.session_id) ? 'ring-2 ring-success bg-success/20' : 'bg-brand/20'}"
              style:margin-left={idx > 0 ? '-0.5rem' : '0'}
              style:z-index={speakingParticipants.has(participant.session_id) ? 30 : 10 - idx}
            >
              <span class="text-brand text-xs font-medium">
                {participant.user_id.slice(0, 2).toUpperCase()}
              </span>
            </div>
          {/each}
          {#if participants.length > 5}
            <div class="w-10 h-10 rounded-full bg-bg-surface-2 flex items-center justify-center shrink-0 -ml-2">
              <span class="text-text-3 text-xs">+{participants.length - 5}</span>
            </div>
          {/if}
        </div>
        <div class="mt-2 text-xs text-text-3">
          {isMuted ? 'Muted' : 'Unmuted'}
          {#if isHandRaised}
            <span class="ml-2 text-warning">Hand raised</span>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Controls -->
    <div class="flex items-center justify-center space-x-3 px-4 py-3 bg-app border-t border-border-1 shrink-0">
      <!-- Mute/Unmute -->
      <button
        onclick={toggleMute}
        class="w-12 h-12 rounded-full flex items-center justify-center transition-all {isMuted ? 'bg-danger/20 text-danger hover:bg-danger/30' : 'bg-bg-surface-2 text-text-1 hover:bg-bg-surface-1'}"
        title={isMuted ? 'Unmute' : 'Mute'}
      >
        {#if isMuted}
          <MicOff class="w-5 h-5" />
        {:else}
          <Mic class="w-5 h-5" />
        {/if}
      </button>

      <!-- Raise Hand -->
      <button
        onclick={toggleHand}
        class="w-10 h-10 rounded-full flex items-center justify-center transition-all {isHandRaised ? 'bg-warning/20 text-warning' : 'bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1'}"
        title={isHandRaised ? 'Lower hand' : 'Raise hand'}
      >
        <Hand class="w-4 h-4" />
      </button>

      <!-- Screen Share -->
      <button
        onclick={toggleScreenShare}
        class="w-10 h-10 rounded-full flex items-center justify-center transition-all {isScreenSharing ? 'bg-success/20 text-success' : 'bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1'}"
        title={isScreenSharing ? 'Stop sharing' : 'Share screen'}
      >
        <Monitor class="w-4 h-4" />
      </button>

      <!-- Participants Toggle (Expanded only) -->
      {#if isExpanded}
        <button
          onclick={() => { showParticipants = !showParticipants }}
          class="w-10 h-10 rounded-full flex items-center justify-center transition-all {showParticipants ? 'bg-brand/20 text-brand' : 'bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1'}"
          title={showParticipants ? 'Hide participants' : 'Show participants'}
        >
          <Users class="w-4 h-4" />
        </button>
      {/if}

      <!-- More Options -->
      <div class="relative">
        <button
          onclick={() => { showMenu = !showMenu }}
          class="w-10 h-10 rounded-full flex items-center justify-center bg-bg-surface-2 text-text-3 hover:bg-bg-surface-1 transition-all"
          title="More options"
        >
          <MoreVertical class="w-4 h-4" />
        </button>

        {#if showMenu}
          <div
            class="absolute bottom-full mb-2 right-0 w-48 bg-bg-surface-2 border border-border-1 rounded-lg shadow-xl py-1 z-50"
          >
            <div
              class="fixed inset-0 z-[-1]"
              onclick={() => { showMenu = false }}
              role="none"
            ></div>

            {#if isHost}
              <button
                onclick={() => { handleMuteAll(); showMenu = false }}
                class="w-full px-4 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center"
              >
                <MicOff class="w-4 h-4 mr-2" />
                Mute All
              </button>
              <button
                onclick={() => { handleRingAll(); showMenu = false }}
                class="w-full px-4 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 flex items-center"
              >
                <Bell class="w-4 h-4 mr-2" />
                Ring Everyone
              </button>
              <div class="my-1 border-t border-border-1"></div>
              <button
                onclick={() => { handleEndCall(); showMenu = false }}
                class="w-full px-4 py-2 text-left text-sm text-danger hover:bg-bg-surface-2 flex items-center"
              >
                <PhoneOff class="w-4 h-4 mr-2" />
                End Call for Everyone
              </button>
            {:else}
              <p class="px-4 py-2 text-xs text-text-3 italic text-center">No host options available</p>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Hangup -->
      <button
        onclick={handleHangup}
        class="w-12 h-12 rounded-full bg-danger hover:bg-danger/90 text-text-1 flex items-center justify-center transition-all"
        title="Leave call"
      >
        <PhoneOff class="w-5 h-5" />
      </button>
    </div>
  </div>
{/if}
