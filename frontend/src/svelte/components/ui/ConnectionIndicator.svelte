<script lang="ts">
  import { connectionStatus } from '../../stores/websocket'

  $: status = $connectionStatus
  $: colorClass =
    status === 'connected'
      ? 'bg-green-500'
      : status === 'reconnecting'
        ? 'bg-amber-500'
        : status === 'disconnected'
          ? 'bg-orange-500'
          : 'bg-red-500'
  $: ariaLabel = `Connection: ${status}`
</script>

<span
  data-testid="connection-indicator"
  data-status={status}
  class={`inline-block h-2.5 w-2.5 rounded-full ${colorClass}`}
  aria-label={ariaLabel}
  role="img"
></span>
