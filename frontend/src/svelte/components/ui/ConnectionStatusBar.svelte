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

  function handleModalReconnect() {
    retryConnection()
  }

  function handleRefresh() {
    window.location.reload()
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

{#if $connectionStatus === 'failed'}
  <div
    data-testid="connection-lost-modal"
    role="dialog"
    aria-modal="true"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
  >
    <div class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl">
      <h2 class="text-lg font-semibold text-gray-900">Disconnected</h2>
      <p class="mt-2 text-sm text-gray-600">Your conversation may be out of date.</p>
      <div class="mt-6 flex gap-3">
        <button
          data-testid="modal-reconnect-button"
          type="button"
          class="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700"
          on:click={handleModalReconnect}
        >
          Reconnect
        </button>
        <button
          data-testid="modal-refresh-button"
          type="button"
          class="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
          on:click={handleRefresh}
        >
          Refresh
        </button>
      </div>
    </div>
  </div>
{/if}
