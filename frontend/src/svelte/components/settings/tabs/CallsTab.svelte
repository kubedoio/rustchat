<script lang="ts">
  let micDevice = 'default'
  let speakerDevice = 'default'
  let turnServer = false
</script>

<div class="space-y-5">
  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <!-- Microphone -->
    <div class="border-b border-border-1 px-5 py-4">
      <label class="mb-1 block text-sm font-medium text-text-1">Microphone</label>
      <p class="mb-2 text-xs text-text-3">Select your preferred microphone device</p>
      <select
        bind:value={micDevice}
        class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard focus:border-brand focus:ring-2 focus:ring-brand/15"
      >
        <option value="default">Default microphone</option>
        <option value="built-in">Built-in Microphone</option>
        <option value="headset">Headset Microphone</option>
      </select>
    </div>

    <!-- Speaker -->
    <div class="border-b border-border-1 px-5 py-4">
      <label class="mb-1 block text-sm font-medium text-text-1">Speaker</label>
      <p class="mb-2 text-xs text-text-3">Select your preferred speaker device</p>
      <select
        bind:value={speakerDevice}
        class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard focus:border-brand focus:ring-2 focus:ring-brand/15"
      >
        <option value="default">Default speaker</option>
        <option value="built-in">Built-in Speakers</option>
        <option value="headset">Headset</option>
      </select>
    </div>

    <!-- TURN Server -->
    <div class="flex items-center justify-between gap-4 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Use TURN server
        </div>
        <div class="mt-1 text-sm text-text-3">Relay calls through a TURN server for better connectivity</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={turnServer} class="sr-only peer" />
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
