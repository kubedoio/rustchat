<script lang="ts">
  import { onMount } from 'svelte'
  import Router from './svelte/Router.svelte'
  import ToastManager from './svelte/components/ui/ToastManager.svelte'
  import { authStore } from './svelte/stores/auth'
  import { configStore } from './svelte/stores/config'
  import { registerWebSocketHandlers } from './svelte/stores/websocket'

  onMount(() => {
    const stopAuthExpirySocket = registerWebSocketHandlers()

    void configStore.fetchPublicConfig()
    void authStore.fetchMe()

    return () => {
      stopAuthExpirySocket()
    }
  })
</script>

<Router />
<ToastManager />
