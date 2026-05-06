<script lang="ts">
  import { onDestroy, onMount } from 'svelte'

  type TurnstileWindow = Window & typeof globalThis & {
    turnstile?: {
      render: (container: HTMLElement | string, options: {
        sitekey: string
        callback?: (token: string) => void
        'error-callback'?: () => void
        'expired-callback'?: () => void
        theme?: 'light' | 'dark' | 'auto'
        size?: 'normal' | 'compact' | 'invisible'
      }) => string
      remove: (widgetId: string) => void
      reset: (widgetId: string) => void
    }
    onTurnstileLoad?: () => void
  }

  export let siteKey: string
  export let onVerify: (token: string) => void = () => {}
  export let onError: () => void = () => {}
  export let onExpired: () => void = () => {}

  let container: HTMLDivElement
  let widgetId: string | undefined

  function renderWidget() {
    const browserWindow = window as TurnstileWindow
    if (!container || !browserWindow.turnstile || widgetId) return

    widgetId = browserWindow.turnstile.render(container, {
      sitekey: siteKey,
      callback: onVerify,
      'error-callback': onError,
      'expired-callback': onExpired,
      theme: 'auto',
    })
  }

  export function reset() {
    const browserWindow = window as TurnstileWindow
    if (widgetId && browserWindow.turnstile) {
      browserWindow.turnstile.reset(widgetId)
    }
  }

  onMount(() => {
    const browserWindow = window as TurnstileWindow
    if (browserWindow.turnstile) {
      renderWidget()
      return
    }

    browserWindow.onTurnstileLoad = renderWidget

    if (!document.querySelector('script[src^="https://challenges.cloudflare.com/turnstile/v0/api.js"]')) {
      const script = document.createElement('script')
      script.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js?onload=onTurnstileLoad'
      script.async = true
      script.defer = true
      document.head.appendChild(script)
    }
  })

  onDestroy(() => {
    const browserWindow = window as TurnstileWindow
    if (widgetId && browserWindow.turnstile) {
      browserWindow.turnstile.remove(widgetId)
    }
  })
</script>

<div bind:this={container} class="turnstile-container"></div>

<style>
  .turnstile-container {
    display: flex;
    justify-content: center;
    min-height: 65px;
  }
</style>
