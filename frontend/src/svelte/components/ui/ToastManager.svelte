<script lang="ts">
  import { CheckCircle, AlertCircle, Info, X } from 'lucide-svelte'
  import { toastStore } from '../../stores/toast'

  let toasts = $state<typeof $toastStore>([])

  toastStore.subscribe((value) => {
    toasts = value
  })
</script>

<div
  data-testid="toast-manager"
  class="fixed top-0 right-0 p-6 z-[100] flex flex-col space-y-4 w-full max-w-sm pointer-events-none"
>
  {#each toasts as toast (toast.id)}
    <div
      data-testid="toast-item"
      class="pointer-events-auto w-full max-w-sm overflow-hidden rounded-lg bg-bg-surface-1 shadow-lg ring-1 ring-text-1/5 toast-enter"
    >
      <div class="p-4">
        <div class="flex items-start">
          <div class="flex-shrink-0">
            {#if toast.type === 'success'}
              <CheckCircle class="h-6 w-6 text-success" />
            {:else if toast.type === 'error'}
              <AlertCircle class="h-6 w-6 text-danger" />
            {:else}
              <Info class="h-6 w-6 text-brand" />
            {/if}
          </div>
          <div class="ml-3 w-0 flex-1 pt-0.5">
            <p class="text-sm font-medium text-text-1">{toast.title}</p>
            {#if toast.message}
              <p class="mt-1 text-sm text-text-3">{toast.message}</p>
            {/if}
          </div>
          <div class="ml-4 flex flex-shrink-0">
            <button
              onclick={() => toastStore.remove(toast.id)}
              type="button"
              class="inline-flex rounded-md bg-bg-surface-1 text-text-4 hover:text-text-3 focus:outline-none focus:ring-2 focus:ring-brand focus:ring-offset-2"
            >
              <span class="sr-only">Close</span>
              <X class="h-5 w-5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  {/each}
</div>

<style>
  .toast-enter {
    animation: toastIn 300ms ease-out;
  }

  @keyframes toastIn {
    from {
      opacity: 0;
      transform: translateY(0.5rem) translateX(0.5rem);
    }
    to {
      opacity: 1;
      transform: translateY(0) translateX(0);
    }
  }
</style>
