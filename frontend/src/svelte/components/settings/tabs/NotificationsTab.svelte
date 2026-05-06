<script lang="ts">
  let desktopNotifications = true
  let pushNotifications = true
  let emailNotifications = true
  let soundEnabled = true
  let mentionKeywords = '@username, @channel, @all, @here'
</script>

<div class="space-y-5">
  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <!-- Desktop Notifications -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Desktop notifications
        </div>
        <div class="mt-1 text-sm text-text-3">Receive notifications in your browser</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={desktopNotifications} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Push Notifications -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Push notifications
        </div>
        <div class="mt-1 text-sm text-text-3">Receive push notifications on mobile</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={pushNotifications} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Email Notifications -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Email notifications
        </div>
        <div class="mt-1 text-sm text-text-3">Receive notifications via email</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={emailNotifications} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Sound -->
    <div class="flex items-center justify-between gap-4 border-b border-border-1 px-5 py-4">
      <div>
        <div class="text-base font-semibold leading-tight tracking-[-0.01em] text-text-1 sm:text-lg">
          Notification sounds
        </div>
        <div class="mt-1 text-sm text-text-3">Play a sound for new notifications</div>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input type="checkbox" bind:checked={soundEnabled} class="sr-only peer" />
        <div
          class="h-6 w-11 rounded-full bg-bg-surface-3 peer-focus:ring-2 peer-focus:ring-brand/15 peer-checked:bg-brand transition-standard"
        ></div>
        <div
          class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform peer-checked:translate-x-5"
        ></div>
      </label>
    </div>

    <!-- Mention Keywords -->
    <div class="px-5 py-4">
      <label class="mb-1 block text-sm font-medium text-text-1">Mention keywords</label>
      <p class="mb-2 text-xs text-text-3">Messages containing these keywords will trigger notifications</p>
      <textarea
        bind:value={mentionKeywords}
        rows="3"
        class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15 resize-none"
        placeholder="@username, @channel, @all, @here"
      ></textarea>
    </div>
  </div>
</div>
