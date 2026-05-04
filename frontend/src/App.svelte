<script lang="ts">
  import { onMount } from 'svelte'
  import Router from './svelte/Router.svelte'
  import { authStore } from './svelte/stores/auth'
  import { configStore } from './svelte/stores/config'
  import { initAuthExpiryWebSocket } from './svelte/stores/websocket'

  onMount(() => {
    const stopAuthExpirySocket = initAuthExpiryWebSocket()

    void configStore.fetchPublicConfig()
    void authStore.fetchMe()

    return () => {
      stopAuthExpirySocket()
    }
  })
</script>

<Router />
