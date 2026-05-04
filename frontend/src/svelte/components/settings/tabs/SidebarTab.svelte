<script lang="ts">
  let groupUnread = 'never'
  let limitDMs = 'all'

  // Silence unused-variable warnings; bind:group reads are not always detected
  $: groupUnread, limitDMs
</script>

<div class="space-y-5">
  <div class="overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-1">
    <!-- Group Unread Channels -->
    <div class="border-b border-border-1 px-5 py-4">
      <label class="mb-1 block text-sm font-medium text-text-1">Group unread channels separately</label>
      <p class="mb-2 text-xs text-text-3">Group unread channels above read channels in the channel sidebar</p>
      <div class="space-y-2">
        <label class="flex items-center gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 cursor-pointer">
          <input type="radio" bind:group={groupUnread} value="never" class="mt-0.5 h-4 w-4 accent-brand" />
          <span>Off</span>
        </label>
        <label class="flex items-center gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 cursor-pointer">
          <input type="radio" bind:group={groupUnread} value="only_for_favorites" class="mt-0.5 h-4 w-4 accent-brand" />
          <span>For favorites only</span>
        </label>
        <label class="flex items-center gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 cursor-pointer">
          <input type="radio" bind:group={groupUnread} value="always" class="mt-0.5 h-4 w-4 accent-brand" />
          <span>On</span>
        </label>
      </div>
    </div>

    <!-- Limit Visible DMs -->
    <div class="px-5 py-4">
      <label class="mb-1 block text-sm font-medium text-text-1">Number of direct messages to show</label>
      <p class="mb-2 text-xs text-text-3">Limit the number of direct messages shown in the sidebar</p>
      <div class="space-y-2">
        <label class="flex items-center gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 cursor-pointer">
          <input type="radio" bind:group={limitDMs} value="all" class="mt-0.5 h-4 w-4 accent-brand" />
          <span>All direct messages</span>
        </label>
        <label class="flex items-center gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 cursor-pointer">
          <input type="radio" bind:group={limitDMs} value="40" class="mt-0.5 h-4 w-4 accent-brand" />
          <span>40 direct messages</span>
        </label>
        <label class="flex items-center gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 cursor-pointer">
          <input type="radio" bind:group={limitDMs} value="20" class="mt-0.5 h-4 w-4 accent-brand" />
          <span>20 direct messages</span>
        </label>
        <label class="flex items-center gap-3 rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 text-sm text-text-2 transition-standard hover:border-border-2 hover:bg-bg-surface-2 cursor-pointer">
          <input type="radio" bind:group={limitDMs} value="10" class="mt-0.5 h-4 w-4 accent-brand" />
          <span>10 direct messages</span>
        </label>
      </div>
    </div>
  </div>
</div>
