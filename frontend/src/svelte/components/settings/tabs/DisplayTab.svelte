<script lang="ts">
  let theme = 'dark'
  let timeFormat = '12h'
  let messageDisplay = 'standard'
  let linkPreviews = true
  let imagePreviews = true

  function themeButtonClass(selected: boolean): string {
    const base = 'rounded-r-2 border px-3 py-2 text-sm transition-standard '
    if (selected) {
      return base + 'border-brand bg-brand/10 text-brand'
    }
    return base + 'border-border-1 bg-bg-surface-2 text-text-2'
  }

  function timeButtonClass(selected: boolean): string {
    const base = 'rounded-r-2 border px-3 py-2 text-sm transition-standard '
    if (selected) {
      return base + 'border-brand bg-brand/10 text-brand'
    }
    return base + 'border-border-1 bg-bg-surface-2 text-text-2'
  }

  function displayButtonClass(selected: boolean): string {
    const base = 'rounded-r-2 border px-3 py-2 text-sm transition-standard '
    if (selected) {
      return base + 'border-brand bg-brand/10 text-brand'
    }
    return base + 'border-border-1 bg-bg-surface-2 text-text-2'
  }
</script>

<div class="space-y-5">
  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <!-- Theme -->
    <div class="border-b border-border-1 px-5 py-4">
      <span class="mb-1 block text-sm font-medium text-text-1">Theme</span>
      <p class="mb-2 text-xs text-text-3">Choose a color theme for the application</p>
      <div class="grid grid-cols-3 gap-2">
        <button
          type="button"
          class={themeButtonClass(theme === 'dark')}
          on:click={() => (theme = 'dark')}
        >
          Dark
        </button>
        <button
          type="button"
          class={themeButtonClass(theme === 'light')}
          on:click={() => (theme = 'light')}
        >
          Light
        </button>
        <button
          type="button"
          class={themeButtonClass(theme === 'system')}
          on:click={() => (theme = 'system')}
        >
          System
        </button>
      </div>
    </div>

    <!-- Time Format -->
    <div class="border-b border-border-1 px-5 py-4">
      <span class="mb-1 block text-sm font-medium text-text-1">Time format</span>
      <p class="mb-2 text-xs text-text-3">Select your preferred clock format</p>
      <div class="flex gap-2">
        <button
          type="button"
          class={timeButtonClass(timeFormat === '12h')}
          on:click={() => (timeFormat = '12h')}
        >
          12-hour
        </button>
        <button
          type="button"
          class={timeButtonClass(timeFormat === '24h')}
          on:click={() => (timeFormat = '24h')}
        >
          24-hour
        </button>
      </div>
    </div>

    <!-- Message Display -->
    <div class="border-b border-border-1 px-5 py-4">
      <span class="mb-1 block text-sm font-medium text-text-1">Message display</span>
      <p class="mb-2 text-xs text-text-3">Choose how messages appear in channels</p>
      <div class="flex gap-2">
        <button
          type="button"
          class={displayButtonClass(messageDisplay === 'standard')}
          on:click={() => (messageDisplay = 'standard')}
        >
          Standard
        </button>
        <button
          type="button"
          class={displayButtonClass(messageDisplay === 'compact')}
          on:click={() => (messageDisplay = 'compact')}
        >
          Compact
        </button>
      </div>
    </div>

    <!-- Link Previews -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Link previews
        </div>
        <div class="mt-1 text-sm text-text-3">Display previews when links are posted</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={linkPreviews} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Image Previews -->
    <div class="flex items-center justify-between gap-4 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Image previews
        </div>
        <div class="mt-1 text-sm text-text-3">Display image previews in messages</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={imagePreviews} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>
  </div>
</div>
