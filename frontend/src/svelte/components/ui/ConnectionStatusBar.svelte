<script lang="ts">
  import { connectionStatus, retryConnection } from '../../stores/websocket'

  let countdown = 5
  let countdownInterval: ReturnType<typeof setInterval> | null = null

  $: {
    if ($connectionStatus === 'disconnected') {
      countdown = 5
      if (countdownInterval) clearInterval(countdownInterval)
      countdownInterval = setInterval(() => {
        countdown = Math.max(0, countdown - 1)
      }, 1000)
    } else {
      if (countdownInterval) {
        clearInterval(countdownInterval)
        countdownInterval = null
      }
    }
  }

  function handleRetry() {
    retryConnection()
  }
</script>

{#if $connectionStatus !== 'connected' && $connectionStatus !== 'connecting'}
  <div
    data-testid="connection-status-bar"
    data-status={$connectionStatus}
    role="status"
    aria-live="polite"
    class={`flex items-center justify-center px-4 py-2 text-sm font-medium ${
      $connectionStatus === 'reconnecting'
        ? 'bg-amber-50 text-amber-800'
        : $connectionStatus === 'disconnected'
          ? 'bg-orange-50 text-orange-800'
          : 'bg-red-50 text-red-800'
    }`}
  >
    {#if $connectionStatus === 'reconnecting'}
      <span class="mr-2 h-2 w-2 animate-pulse rounded-full bg-amber-500"></span>
      Reconnecting...
    {:else if $connectionStatus === 'disconnected'}
      <span class="mr-2 h-2 w-2 rounded-full bg-orange-500"></span>
      Connection lost. Retrying in {countdown}s
      <button
        data-testid="retry-connection-button"
        type="button"
        class="ml-3 rounded-md bg-orange-100 px-3 py-1 text-xs font-semibold text-orange-800 hover:bg-orange-200"
        on:click={handleRetry}
      >
        Retry now
      </button>
    {:else if $connectionStatus === 'failed'}
      <span class="mr-2 h-2 w-2 rounded-full bg-red-500"></span>
      Disconnected. Your conversation may be out of date.
    {/if}
  </div>
{/if}


