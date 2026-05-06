<script lang="ts">
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X, Search, User, UserPlus } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi, SvelteHttpError } from '../../stores/http'
  import { authStore } from '../../stores/auth'
  import { chatStore } from '../../stores/chat'
  import type { SvelteChatMember } from '../../stores/chat'

  interface Props {
    open?: boolean
    channelId?: string
    channelName?: string
    onclose?: () => void
    onmembersAdded?: (count: number) => void
  }

  let {
    open = false,
    channelId,
    channelName,
    onclose,
    onmembersAdded,
  }: Props = $props()

  let search = $state('')
  let addingMembers = $state<Set<string>>(new Set())
  let error = $state('')
  let success = $state('')
  let addedCount = $state(0)
  let currentMembers = $state<Set<string>>(new Set())
  let channelMembersLoading = $state(false)

  let channelTeamId = $derived((() => {
    if (!channelId) return null
    const channel = $chatStore.channels.find((c) => c.id === channelId)
    return channel?.team_id ?? null
  })())

  let currentTeamId = $derived(
    channelTeamId ??
      (() => {
        const channel = $chatStore.channels.find((c) => c.id === $chatStore.currentChannelId)
        return channel?.team_id ?? $chatStore.teams[0]?.id ?? null
      })()
  )

  $effect(() => {
    if (open) {
      search = ''
      error = ''
      success = ''
      addedCount = 0
      currentMembers = new Set()
      if (currentTeamId) {
        if (!$chatStore.membersByTeam[currentTeamId]?.length) {
          chatStore.fetchMembers(currentTeamId)
        }
      }
      if (channelId) {
        fetchChannelMembers()
      }
    }
  })

  async function fetchChannelMembers() {
    if (!channelId) return
    channelMembersLoading = true
    try {
      const { data } = await svelteApi.get<Array<{ user_id: string }>>(`/channels/${channelId}/members`)
      currentMembers = new Set(data.map((m) => m.user_id))
    } catch (e) {
      console.error('Failed to fetch channel members:', e)
    } finally {
      channelMembersLoading = false
    }
  }

  let members = $derived(currentTeamId ? ($chatStore.membersByTeam[currentTeamId] ?? []) : [])

  let filteredMembers = $derived(((): SvelteChatMember[] => {
    if (!members.length) return []
    const searchLower = search.toLowerCase()
    return members.filter((m) => {
      if (m.user_id === $authStore.user?.id) return false
      if (currentMembers.has(m.user_id)) return false
      return (
        m.username.toLowerCase().includes(searchLower) ||
        (m.display_name && m.display_name.toLowerCase().includes(searchLower))
      )
    })
  })())

  async function addMember(member: SvelteChatMember) {
    if (!channelId || addingMembers.has(member.user_id)) return

    addingMembers.add(member.user_id)
    error = ''
    success = ''

    try {
      await svelteApi.post(`/channels/${channelId}/members`, { user_id: member.user_id })
      currentMembers.add(member.user_id)
      success = `Added ${member.display_name || member.username}`
      addedCount += 1

      setTimeout(() => {
        success = ''
      }, 2000)
    } catch (err: unknown) {
      const message =
        err instanceof SvelteHttpError
          ? ((err.data as { message?: string })?.message ?? `Failed to add ${member.username}`)
          : `Failed to add ${member.username}`
      error = message
    } finally {
      addingMembers.delete(member.user_id)
    }
  }

  function handleClose() {
    const count = addedCount
    search = ''
    error = ''
    success = ''
    addedCount = 0
    onclose?.()
    if (count > 0) {
      onmembersAdded?.(count)
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose()
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    data-testid="add-channel-members-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/50 backdrop-blur-sm"
      onclick={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close modal"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal -->
    <div
      class="relative mx-4 flex max-h-[80vh] w-full max-w-md flex-col overflow-hidden rounded-xl bg-white shadow-2xl"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-gray-200 px-6 py-4">
        <div>
          <h2 class="text-xl font-bold text-gray-900">Add Members</h2>
          {#if channelName}
            <p class="mt-0.5 text-sm text-gray-500">To #{channelName}</p>
          {/if}
        </div>
        <button
          onclick={handleClose}
          class="rounded-lg p-1 transition-colors hover:bg-gray-100"
          aria-label="Close"
        >
          <X class="h-5 w-5 text-gray-500" />
        </button>
      </div>

      <!-- Search -->
      <div class="border-b border-gray-100 px-6 py-4">
        <div class="relative">
          <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
          <input
            bind:value={search}
            type="text"
            placeholder="Search for team members..."
            class="w-full rounded-lg border border-gray-200 bg-gray-50 py-2 pl-10 pr-4 text-sm text-gray-900 outline-none transition-all focus:border-transparent focus:ring-2 focus:ring-indigo-500"
            autofocus
          />
        </div>
      </div>

      <!-- Content -->
      <div class="custom-scrollbar min-h-[200px] flex-1 overflow-y-auto p-2">
        {#if error}
          <div class="m-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-600">
            {error}
          </div>
        {/if}

        {#if success}
          <div class="m-4 rounded-lg border border-green-200 bg-green-50 p-3 text-sm text-green-600">
            {success}
          </div>
        {/if}

        {#if ($chatStore.loading || channelMembersLoading) && members.length === 0}
          <div class="flex flex-col items-center justify-center py-12 text-gray-500">
            <div
              class="mb-4 h-8 w-8 animate-spin rounded-full border-4 border-indigo-500/20 border-t-indigo-500"
            ></div>
            <p class="text-sm">Loading members...</p>
          </div>
        {:else if filteredMembers.length > 0}
          <div class="space-y-1">
            {#each filteredMembers as member (member.user_id)}
              <button
                onclick={() => addMember(member)}
                disabled={addingMembers.has(member.user_id)}
                class="group flex w-full items-center justify-between rounded-lg px-4 py-3 text-left transition-colors hover:bg-indigo-50"
              >
                <div class="flex items-center">
                  <div class="relative mr-4">
                    {#if member.avatar_url}
                      <img
                        src={member.avatar_url}
                        alt=""
                        class="h-10 w-10 rounded-full object-cover shadow-sm ring-2 ring-white"
                      />
                    {:else}
                      <div
                        class="flex h-10 w-10 items-center justify-center rounded-full bg-indigo-100 shadow-sm ring-2 ring-white"
                      >
                        <User class="h-6 w-6 text-indigo-500" />
                      </div>
                    {/if}
                  </div>

                  <div class="min-w-0 flex-1">
                    <p class="truncate text-sm font-semibold text-gray-900">
                      {member.display_name || member.username}
                    </p>
                    <p class="truncate text-xs text-gray-500">
                      @{member.username}
                    </p>
                  </div>
                </div>

                {#if addingMembers.has(member.user_id)}
                  <div class="flex h-8 w-8 items-center justify-center">
                    <div
                      class="h-5 w-5 animate-spin rounded-full border-2 border-indigo-500/20 border-t-indigo-500"
                    ></div>
                  </div>
                {:else}
                  <div
                    class="flex h-8 w-8 items-center justify-center rounded-full bg-indigo-100 text-indigo-600 opacity-0 transition-opacity group-hover:opacity-100"
                  >
                    <UserPlus class="h-4 w-4" />
                  </div>
                {/if}
              </button>
            {/each}
          </div>
        {:else}
          <div class="flex flex-col items-center justify-center px-6 py-12 text-center text-gray-500">
            <div class="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-gray-100">
              <UserPlus class="h-6 w-6 text-gray-400" />
            </div>
            <p class="text-sm font-medium">No members to add</p>
            <p class="mt-1 text-xs">All team members are already in this channel</p>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex justify-end border-t border-gray-200 bg-gray-50 px-6 py-4">
        <button
          type="button"
          onclick={handleClose}
          class="inline-flex items-center rounded-lg border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-50"
        >
          Done
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #cbd5e1;
    border-radius: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: var(--border-2);
  }
</style>
