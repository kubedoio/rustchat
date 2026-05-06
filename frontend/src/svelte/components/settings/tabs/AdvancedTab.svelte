<script lang="ts">
  let sendOnCtrlEnter = false
  let enableFormatting = true
  let enableJoinLeave = true
  let performanceDebug = false
</script>

<div class="space-y-5">
  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <!-- Send on Ctrl+Enter -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Send messages on Ctrl+Enter
        </div>
        <div class="mt-1 text-sm text-text-3">When enabled, press Ctrl+Enter to send. Press Enter alone for a new line.</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={sendOnCtrlEnter} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Post Formatting -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Enable post formatting
        </div>
        <div class="mt-1 text-sm text-text-3">Show the formatting toolbar in the message input</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={enableFormatting} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Join/Leave Messages -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Enable join/leave messages
        </div>
        <div class="mt-1 text-sm text-text-3">Display system messages when users join or leave channels</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={enableJoinLeave} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Performance Debugging -->
    <div class="flex items-center justify-between gap-4 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Enable performance debugging
        </div>
        <div class="mt-1 text-sm text-text-3">Show performance metrics and debugging information</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={performanceDebug} class="sr-only peer" />
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
