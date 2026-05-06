<script lang="ts">
  import { X, ChevronLeft, ChevronRight, Download } from 'lucide-svelte'
  import { scale } from 'svelte/transition'

  interface ImageData {
    id: string
    name: string
    url: string
    thumbnail_url?: string
  }

  interface Props {
    images?: ImageData[]
    initialIndex?: number
    onClose?: () => void
  }

  let { images = [], initialIndex = 0, onClose }: Props = $props()

  function boundedIndex(index: number, length: number): number {
    if (length <= 0 || !Number.isFinite(index)) return 0
    return Math.min(Math.max(Math.trunc(index), 0), length - 1)
  }

  let currentIndex = $state(0)

  let currentImage = $derived(images[currentIndex])

  function next() {
    if (images.length <= 1) return
    currentIndex = (currentIndex + 1) % images.length
  }

  function prev() {
    if (images.length <= 1) return
    currentIndex = (currentIndex - 1 + images.length) % images.length
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose?.()
    if (e.key === 'ArrowRight') next()
    if (e.key === 'ArrowLeft') prev()
  }

  $effect(() => {
    currentIndex = boundedIndex(initialIndex, images.length)
  })

  $effect(() => {
    currentIndex = boundedIndex(currentIndex, images.length)
  })

  $effect(() => {
    window.addEventListener('keydown', handleKeydown)
    document.body.style.overflow = 'hidden'

    return () => {
      window.removeEventListener('keydown', handleKeydown)
      document.body.style.overflow = ''
    }
  })
</script>

<div
  class="fixed inset-0 z-[100] flex flex-col bg-black/95 backdrop-blur-sm animate-in fade-in duration-300"
  data-testid="image-gallery"
  role="dialog"
  aria-modal="true"
  aria-label="Image gallery"
>
  <!-- Header -->
  <div class="flex items-center justify-between p-4 text-white z-10">
    {#if currentImage}
      <div class="flex flex-col">
        <span class="text-sm font-semibold truncate max-w-md">{currentImage.name}</span>
        <span class="text-[11px] text-gray-400 capitalize">{currentIndex + 1} of {images.length}</span>
      </div>
    {:else}
      <div></div>
    {/if}
    <div class="flex items-center space-x-2">
      {#if currentImage}
        <a
          href={currentImage.url}
          download
          class="p-2 hover:bg-white/10 rounded-full transition-colors"
          title="Download"
          aria-label={`Download ${currentImage.name}`}
        >
          <Download class="w-5 h-5" />
        </a>
      {/if}
      <button
        type="button"
        onclick={() => onClose?.()}
        class="p-2 hover:bg-white/10 rounded-full transition-colors"
        title="Close"
        aria-label="Close image gallery"
      >
        <X class="w-6 h-6" />
      </button>
    </div>
  </div>

  <!-- Main View -->
  <div class="flex-1 relative flex items-center justify-center overflow-hidden">
    <!-- Navigation -->
    {#if images.length > 1}
      <button
        type="button"
        onclick={prev}
        class="absolute left-4 z-20 p-3 bg-black/20 hover:bg-black/40 text-white rounded-full backdrop-blur-md transition-all active:scale-95"
        aria-label="Previous image"
      >
        <ChevronLeft class="w-8 h-8" />
      </button>
    {/if}

    <!-- Image -->
    {#key currentImage?.id}
      {#if currentImage}
        <img
          src={currentImage.url}
          alt={currentImage.name}
          class="max-w-[90vw] max-h-[80vh] object-contain shadow-2xl rounded-sm"
          in:scale={{ duration: 300, start: 0.95, opacity: 0 }}
          out:scale={{ duration: 200, start: 0.95, opacity: 0 }}
        />
      {:else}
        <div class="px-6 text-center text-sm text-gray-300">
          No images to display.
        </div>
      {/if}
    {/key}

    {#if images.length > 1}
      <button
        type="button"
        onclick={next}
        class="absolute right-4 z-20 p-3 bg-black/20 hover:bg-black/40 text-white rounded-full backdrop-blur-md transition-all active:scale-95"
        aria-label="Next image"
      >
        <ChevronRight class="w-8 h-8" />
      </button>
    {/if}
  </div>

  <!-- Thumbnails Strip -->
  {#if images.length > 1}
    <div class="h-24 bg-black/40 backdrop-blur-md flex items-center justify-center p-4 space-x-2 overflow-x-auto">
      {#each images as img, index (img.id)}
        <button
          type="button"
          onclick={() => (currentIndex = index)}
          class="h-16 aspect-video rounded overflow-hidden border-2 transition-all flex-shrink-0"
          class:border-primary={currentIndex === index}
          class:scale-105={currentIndex === index}
          class:shadow-lg={currentIndex === index}
          class:border-transparent={currentIndex !== index}
          class:opacity-50={currentIndex !== index}
          class:hover:opacity-100={currentIndex !== index}
          aria-label={`View ${img.name}`}
          aria-current={currentIndex === index ? 'true' : undefined}
        >
          <img src={img.thumbnail_url || img.url} class="w-full h-full object-cover" alt={img.name} />
        </button>
      {/each}
    </div>
  {/if}
</div>
