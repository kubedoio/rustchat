<script lang="ts">
  import { callsStore } from '../../stores/calls.svelte'
  import { Phone, PhoneOff } from 'lucide-svelte'

  let incomingCall = $derived(callsStore.incomingCall)

  async function accept() {
    if (incomingCall) {
      await callsStore.joinCall(incomingCall.channelId)
      callsStore.setIncomingCall(null)
    }
  }

  function decline() {
    callsStore.setIncomingCall(null)
  }
</script>

{#if incomingCall}
  <div data-testid="incoming-call-modal" class="fixed top-4 right-4 z-50 animate-slide-in">
    <div class="bg-bg-surface-1 rounded-lg shadow-xl border border-border-1 p-4 w-72">
      <div class="flex items-center space-x-3 mb-4">
        <div class="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center">
          <Phone class="w-5 h-5 text-primary animate-pulse" />
        </div>
        <div>
          <h3 class="font-medium text-text-1">Incoming Call</h3>
          <p class="text-sm text-text-3">Channel Call</p>
        </div>
      </div>

      <div class="flex space-x-3">
        <button
          onclick={decline}
          class="flex-1 flex items-center justify-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-danger hover:bg-danger/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-danger"
        >
          <PhoneOff class="w-4 h-4 mr-2" />
          Decline
        </button>
        <button
          onclick={accept}
          class="flex-1 flex items-center justify-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-success hover:bg-success/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-success"
        >
          <Phone class="w-4 h-4 mr-2" />
          Accept
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .animate-slide-in {
    animation: slideIn 0.3s ease-out;
  }
  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
</style>
