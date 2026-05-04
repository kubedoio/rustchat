<script lang="ts">
  import { tick } from 'svelte'

  interface Props {
    show: boolean
    anchorEl?: HTMLElement | null
    onSelect?: (emoji: string) => void
    onClose?: () => void
  }

  let { show, anchorEl = null, onSelect, onClose }: Props = $props()

  const categories = [
    { id: 'frequent', name: '👍', emojis: ['👍', '❤️', '😂', '🎉', '🤔', '👀', '🙌', '💯'] },
    { id: 'smileys', name: '😀', emojis: ['😀', '😃', '😄', '😁', '😆', '😅', '🤣', '😂', '🙂', '😊', '😇', '🥰', '😍', '🤩', '😘', '😗', '😚', '😋', '😛', '😜', '🤪', '😝', '🤑', '🤗', '🤭', '🤫', '🤔', '🤐', '🤨', '😐', '😑', '😶', '😏', '😒', '🙄', '😬', '🤥', '😌', '😔', '😪', '🤤', '😴', '😷'] },
    { id: 'gestures', name: '👋', emojis: ['👋', '🤚', '🖐️', '✋', '🖖', '👌', '🤌', '🤏', '✌️', '🤞', '🤟', '🤘', '🤙', '👈', '👉', '👆', '🖕', '👇', '☝️', '👍', '👎', '✊', '👊', '🤛', '🤜', '👏', '🙌', '👐', '🤲', '🤝', '🙏'] },
    { id: 'hearts', name: '❤️', emojis: ['❤️', '🧡', '💛', '💚', '💙', '💜', '🖤', '🤍', '🤎', '💔', '❤️‍🔥', '❤️‍🩹', '❣️', '💕', '💞', '💓', '💗', '💖', '💘', '💝'] },
    { id: 'objects', name: '💡', emojis: ['⭐', '🌟', '✨', '⚡', '🔥', '💫', '🎯', '🎪', '🎨', '🎬', '🎤', '🎧', '🎵', '🎶', '🎹', '🥁', '🎸', '🎺', '🎻', '🎲', '🎮', '🕹️', '🎰', '🧩'] },
    { id: 'symbols', name: '✅', emojis: ['✅', '❌', '❓', '❗', '💯', '🔴', '🟠', '🟡', '🟢', '🔵', '🟣', '⚫', '⚪', '🟤', '🔶', '🔷', '🔸', '🔹', '▪️', '▫️', '◾', '◽', '◼️', '◻️', '⬛', '⬜'] },
  ]

  let activeCategory = $state('frequent')
  let searchQuery = $state('')
  let pickerEl = $state<HTMLDivElement | null>(null)
  let pickerStyle = $state<{ left?: string; top?: string }>({})

  let filteredEmojis = $derived.by(() => {
    const cat = categories.find((c) => c.id === activeCategory)
    if (!cat) return []
    if (searchQuery) {
      return cat.emojis.filter((e) => e.includes(searchQuery))
    }
    return cat.emojis
  })

  function selectEmoji(emoji: string) {
    onSelect?.(emoji)
    onClose?.()
  }

  function updatePosition() {
    if (!show || !anchorEl || !pickerEl) return

    const anchorRect = anchorEl.getBoundingClientRect()
    const panelRect = pickerEl.getBoundingClientRect()
    const viewportPadding = 8
    const gap = 10

    let left = anchorRect.right - panelRect.width
    left = Math.max(viewportPadding, Math.min(left, window.innerWidth - panelRect.width - viewportPadding))

    let top = anchorRect.top - panelRect.height - gap
    if (top < viewportPadding) {
      top = anchorRect.bottom + gap
    }
    top = Math.max(viewportPadding, Math.min(top, window.innerHeight - panelRect.height - viewportPadding))

    pickerStyle = {
      left: `${Math.round(left)}px`,
      top: `${Math.round(top)}px`,
    }
  }

  function handlePointerDown(event: MouseEvent) {
    if (!show) return
    const target = event.target as Node | null
    if (!target) return
    if (pickerEl?.contains(target)) return
    if (anchorEl?.contains(target)) return
    onClose?.()
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (show && event.key === 'Escape') {
      onClose?.()
    }
  }

  $effect(() => {
    if (!show) return
    void anchorEl
    void tick().then(updatePosition)
  })

  $effect(() => {
    if (!show) return
    window.addEventListener('resize', updatePosition)
    window.addEventListener('scroll', updatePosition, true)
    document.addEventListener('mousedown', handlePointerDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('resize', updatePosition)
      window.removeEventListener('scroll', updatePosition, true)
      document.removeEventListener('mousedown', handlePointerDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  })

  $effect(() => {
    if (pickerEl && show) {
      document.body.appendChild(pickerEl)
    }
  })
</script>

{#if show}
  <div
    bind:this={pickerEl}
    style:left={pickerStyle.left}
    style:top={pickerStyle.top}
    class="fixed z-[9999] w-[22rem] max-w-[calc(100vw-1rem)] overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 text-text-1 shadow-2xl animate-fade-in"
    data-testid="emoji-picker"
    role="dialog"
    aria-label="Emoji picker"
  >
    <!-- Header -->
    <div class="border-b border-border-1 p-2">
      <input
        bind:value={searchQuery}
        type="text"
        placeholder="Search emoji..."
        aria-label="Search emoji"
        class="w-full rounded-r-2 border border-border-1 bg-bg-surface-2 px-3 py-1.5 text-sm text-text-1 placeholder:text-text-3 focus:border-brand focus:outline-none focus:ring-2 focus:ring-brand/15"
      />
    </div>

    <!-- Categories -->
    <div class="flex items-center space-x-1 border-b border-border-1 px-2 py-1">
      {#each categories as cat (cat.id)}
        <button
          type="button"
          onclick={() => (activeCategory = cat.id)}
          class="rounded-r-1 p-1.5 text-lg transition-standard hover:bg-bg-surface-2 {activeCategory === cat.id ? 'bg-bg-surface-2 text-brand' : 'text-text-2'}"
          aria-label="{cat.id} emojis"
        >
          {cat.name}
        </button>
      {/each}
    </div>

    <!-- Emojis Grid -->
    <div class="p-2 grid grid-cols-8 gap-1 max-h-56 overflow-y-auto">
      {#each filteredEmojis as emoji (emoji)}
        <button
          type="button"
          onclick={() => selectEmoji(emoji)}
          class="rounded-r-1 p-1.5 text-xl transition-standard hover:bg-bg-surface-2"
          aria-label="Select {emoji}"
        >
          {emoji}
        </button>
      {/each}
    </div>

    <!-- Empty State -->
    {#if filteredEmojis.length === 0}
      <div class="p-4 text-center text-sm text-text-3">
        No emojis found
      </div>
    {/if}
  </div>
{/if}
