<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { WifiOff, RefreshCw, RotateCcw } from 'lucide-svelte'

  export let open = false

  const dispatch = createEventDispatcher<{ reconnect: void; refresh: void }>()
</script>

{#if open}
  <div class="fixed inset-0 z-[110]">
    <!-- Backdrop with blur -->
    <div class="absolute inset-0 bg-black/40 backdrop-blur-md" aria-hidden="true"></div>

    <!-- Modal -->
    <div class="relative flex h-full items-center justify-center p-4">
      <div
        class="w-full max-w-md rounded-xl border border-border-1 bg-bg-surface-1 p-8 text-center shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-labelledby="disconnect-title"
        data-testid="connection-lost-modal"
      >
        <!-- Icon -->
        <div class="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-red-500/10">
          <WifiOff class="h-8 w-8 text-red-500" />
        </div>

        <!-- Title -->
        <h2 id="disconnect-title" class="mt-4 text-lg font-semibold text-text-1">Disconnected</h2>

        <!-- Message -->
        <p class="mt-2 text-sm text-text-2">
          You've been disconnected from the server. Your conversation may be out of date.
        </p>

        <!-- Actions -->
        <div class="mt-6 flex flex-col gap-3">
          <button
            data-testid="modal-reconnect-button"
            type="button"
            class="inline-flex items-center justify-center gap-2 rounded-lg bg-brand px-4 py-2.5 text-sm font-medium text-brand-foreground transition-standard hover:bg-brand-hover"
            onclick={() => dispatch('reconnect')}
          >
            <RotateCcw class="h-4 w-4" />
            Reconnect
          </button>

          <button
            data-testid="modal-refresh-button"
            type="button"
            class="inline-flex items-center justify-center gap-2 rounded-lg bg-bg-surface-2 px-4 py-2.5 text-sm font-medium text-text-2 transition-standard hover:bg-bg-surface-3 hover:text-text-1"
            onclick={() => dispatch('refresh')}
          >
            <RefreshCw class="h-4 w-4" />
            Refresh page
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
