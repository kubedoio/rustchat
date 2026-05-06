<script lang="ts">
  import { fade, fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'

  interface Props {
    children: import('svelte').Snippet
    duration?: number
    y?: number
    mode?: 'fade' | 'fly' | 'scale'
  }

  let { children, duration = 150, y = 10, mode = 'fade' }: Props = $props()
</script>

{#if mode === 'fade'}
  <div transition:fade={{ duration, easing: cubicOut }}>
    {@render children()}
  </div>
{:else if mode === 'fly'}
  <div transition:fly={{ duration, y, easing: cubicOut }}>
    {@render children()}
  </div>
{:else}
  <div transition:fade={{ duration: duration / 2 }}>
    {@render children()}
  </div>
{/if}
