<script lang="ts">
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'

  interface Props {
    children: import('svelte').Snippet
    direction?: 'left' | 'right'
    duration?: number
  }

  let { children, direction = 'right', duration = 250 }: Props = $props()
  const x = direction === 'right' ? 400 : -400
</script>

<div transition:fly={{ x, duration, easing: cubicOut }} class="h-full">
  {@render children()}
</div>
