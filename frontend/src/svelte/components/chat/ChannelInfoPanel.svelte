<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { Hash, Lock, X, Users, Calendar } from 'lucide-svelte'
  import type { SvelteChatChannel, SvelteChatMember } from '../../stores/chat'

  export let channel: SvelteChatChannel | null = null
  export let members: SvelteChatMember[] = []
  export let open = false

  const dispatch = createEventDispatcher<{ close: void }>()

  function channelTypeLabel(type: string): string {
    const labels: Record<string, string> = {
      public: 'Public Channel',
      private: 'Private Channel',
      direct: 'Direct Message',
      group: 'Group Message',
    }
    return labels[type] || 'Channel'
  }

  function presenceLabel(presence?: string): string {
    switch (presence) {
      case 'online':
        return 'Online'
      case 'away':
        return 'Away'
      case 'dnd':
        return 'Do not disturb'
      default:
        return 'Offline'
    }
  }

  function presenceBadgeClass(presence?: string): string {
    switch (presence) {
      case 'online':
        return 'border-success bg-success text-white'
      case 'away':
        return 'border-warning bg-warning/15 text-warning'
      case 'dnd':
        return 'border-danger bg-danger text-white'
      default:
        return 'border-border-2 bg-bg-surface-1 text-text-4'
    }
  }

  function presenceDotClass(presence?: string): string {
    switch (presence) {
      case 'online':
        return 'bg-success'
      case 'away':
        return 'bg-warning'
      case 'dnd':
        return 'bg-danger'
      default:
        return 'bg-text-4'
    }
  }

  function memberInitials(member: SvelteChatMember): string {
    const name = member.display_name || member.username
    return name.slice(0, 2).toUpperCase()
  }

  function formatDate(value: string | undefined): string {
    if (!value) return '—'
    try {
      return new Date(value).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      })
    } catch {
      return '—'
    }
  }

  function handleClose() {
    dispatch('close')
  }
</script>

{#if open}
  <aside
    class="flex w-80 shrink-0 flex-col border-l border-border-1 bg-bg-surface-1 animate-slide-in-right"
    aria-label="Channel info"
  >
    <!-- Header -->
    <div class="flex h-[var(--header-height)] items-center justify-between border-b border-border-1 px-4">
      <h2 class="text-sm font-semibold text-text-1">Channel Info</h2>
      <button
        class="flex h-8 w-8 items-center justify-center rounded-r-1 text-text-3 transition-standard hover:bg-bg-surface-2 hover:text-text-1 focus-ring"
        aria-label="Close channel info"
        on:click={handleClose}
      >
        <X class="h-4 w-4" />
      </button>
    </div>

    <!-- Channel Info Card -->
    <div class="border-b border-border-1 p-5">
      <div class="flex items-start gap-3">
        <div
          class="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl
          {channel?.channel_type === 'private' ? 'bg-amber-100 text-amber-600' : 'bg-blue-100 text-blue-600'}"
        >
          {#if channel?.channel_type === 'private'}
            <Lock class="h-6 w-6" />
          {:else}
            <Hash class="h-6 w-6" />
          {/if}
        </div>
        <div class="min-w-0 flex-1">
          <h3 class="truncate text-lg font-bold text-text-1">
            {channel?.display_name || channel?.name || 'Unknown'}
          </h3>
          <p class="text-xs font-medium text-text-3">
            {channelTypeLabel(channel?.channel_type ?? '')}
          </p>
        </div>
      </div>

      <!-- DM presence & custom status -->
      {#if channel?.channel_type === 'direct' && members.length > 0}
        <div class="mt-3 flex flex-wrap items-center gap-2">
          {#each members.slice(0, 1) as member (member.user_id)}
            <span
              data-testid="channel-info-presence"
              class="inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-sm font-medium {presenceBadgeClass(
                member.presence,
              )}"
            >
              <span class="h-2 w-2 rounded-full {presenceDotClass(member.presence)}"></span>
              {presenceLabel(member.presence)}
            </span>
            <span
              data-testid="channel-info-custom-status"
              class="inline-flex max-w-full items-center gap-1 rounded-full border border-border-1 bg-bg-surface-1 px-3 py-1 text-sm text-text-2"
            >
              <span class="truncate">No status set</span>
            </span>
          {/each}
        </div>
      {/if}

      <!-- Stats row -->
      <div class="mt-4 grid grid-cols-2 gap-3">
        <div class="rounded-xl bg-bg-surface-2/50 p-3">
          <div class="mb-1 flex items-center gap-2 text-text-3">
            <Users class="h-4 w-4" />
            <span class="text-[11px] font-bold uppercase tracking-wider">Members</span>
          </div>
          <p class="text-2xl font-bold text-text-1">{members.length}</p>
        </div>
        <div class="rounded-xl bg-bg-surface-2/50 p-3">
          <div class="mb-1 flex items-center gap-2 text-text-3">
            <Calendar class="h-4 w-4" />
            <span class="text-[11px] font-bold uppercase tracking-wider">Created</span>
          </div>
          <p class="text-sm font-medium text-text-1">{formatDate(undefined)}</p>
        </div>
      </div>
    </div>

    <!-- Members -->
    <div class="flex-1 overflow-y-auto p-4 custom-scrollbar-thin">
      <h3 class="mb-3 text-xs font-semibold uppercase tracking-[0.16em] text-text-3">Members</h3>
      <ul class="space-y-1">
        {#each members as member (member.user_id)}
          <li
            class="flex items-center gap-3 rounded-r-2 px-2 py-2 transition-standard hover:bg-bg-surface-2"
          >
            {#if member.avatar_url}
              <img
                src={member.avatar_url}
                alt={member.display_name || member.username}
                class="h-8 w-8 shrink-0 rounded-full object-cover"
              />
            {:else}
              <div
                class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-brand/10 text-xs font-semibold text-brand"
              >
                {memberInitials(member)}
              </div>
            {/if}
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium text-text-1">
                {member.display_name || member.username}
              </p>
              <p class="truncate text-xs text-text-3">@{member.username}</p>
            </div>
            <span
              class="h-2 w-2 shrink-0 rounded-full {presenceDotClass(member.presence)}"
              aria-hidden="true"
            ></span>
          </li>
        {:else}
          <li class="py-2 text-sm text-text-3">No members available</li>
        {/each}
      </ul>
    </div>
  </aside>
{/if}
