<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { X, Hash, Lock, Settings, Users, Shield, Bell, Trash2, Search, Plus, UserMinus } from 'lucide-svelte'
  import { svelteApi } from '../../stores/http'
  import { authStore } from '../../stores/auth'
  import { chatStore } from '../../stores/chat'
  import type { SvelteChatChannel } from '../../stores/chat'

  export let channel: SvelteChatChannel | null = null
  export let open = false

  const dispatch = createEventDispatcher<{ close: void }>()

  interface ChannelMemberItem {
    user_id: string
    username: string
    display_name?: string
    role?: string
  }

  let activeTab = 'overview'
  let loading = false
  let deleting = false

  let displayName = ''
  let purpose = ''
  let headerText = ''
  let channelName = ''

  let channelMembers: ChannelMemberItem[] = []
  let searchQuery = ''
  let addingMember: string | null = null
  let removingMember: string | null = null
  let membersLoading = false

  const tabs = [
    { id: 'overview', label: 'Overview', icon: Settings },
    { id: 'members', label: 'Members', icon: Users },
    { id: 'permissions', label: 'Permissions', icon: Shield },
    { id: 'notifications', label: 'Notifications', icon: Bell },
  ]

  $: if (open && channel) {
    displayName = channel.display_name || ''
    purpose = ''
    headerText = ''
    channelName = channel.name || ''
    activeTab = 'overview'
    searchQuery = ''
    channelMembers = []
    loadChannelDetails()
  }

  async function loadChannelDetails() {
    if (!channel) return
    try {
      const { data } = await svelteApi.get<{ purpose?: string; header?: string }>(`/channels/${channel.id}`)
      purpose = data.purpose || ''
      headerText = data.header || ''
    } catch (e) {
      console.error('Failed to fetch channel details', e)
    }
  }

  $: if (activeTab === 'members' && channel) {
    fetchMembers()
  }

  async function fetchMembers() {
    if (!channel) return
    membersLoading = true
    try {
      const { data } = await svelteApi.get<ChannelMemberItem[]>(`/channels/${channel.id}/members`)
      channelMembers = data
    } catch (e) {
      console.error('Failed to fetch channel members', e)
    } finally {
      membersLoading = false
    }
  }

  $: teamMembers = channel ? $chatStore.membersByTeam[channel.team_id] ?? [] : []
  $: searchResults = (() => {
    if (!searchQuery.trim() || !channel) return []
    const query = searchQuery.toLowerCase()
    const currentMemberIds = new Set(channelMembers.map((m) => m.user_id))
    return teamMembers
      .filter((member) => {
        if (currentMemberIds.has(member.user_id)) return false
        const name = (member.display_name || '').toLowerCase()
        const username = member.username.toLowerCase()
        return name.includes(query) || username.includes(query)
      })
      .slice(0, 5)
  })()

  async function addMember(userId: string) {
    if (!channel) return
    addingMember = userId
    try {
      await svelteApi.post(`/channels/${channel.id}/members`, { user_id: userId })
      await fetchMembers()
      searchQuery = ''
    } catch (e: unknown) {
      console.error('Failed to add member', e)
    } finally {
      addingMember = null
    }
  }

  async function removeMember(userId: string) {
    if (!channel) return
    if (!confirm('Are you sure you want to remove this member?')) return
    removingMember = userId
    try {
      await svelteApi.delete(`/channels/${channel.id}/members/${userId}`)
      await fetchMembers()
    } catch (e: unknown) {
      console.error('Failed to remove member', e)
    } finally {
      removingMember = null
    }
  }

  async function handleSave() {
    if (!channel) return
    loading = true
    try {
      await svelteApi.put(`/channels/${channel.id}`, {
        name: channelName.trim() || undefined,
        display_name: displayName.trim() || undefined,
        purpose: purpose.trim() || undefined,
        header: headerText.trim() || undefined,
      })
      dispatch('close')
    } catch (e: unknown) {
      console.error('Failed to update channel', e)
    } finally {
      loading = false
    }
  }

  async function handleDelete() {
    if (!channel) return
    if (!confirm(`Are you sure you want to delete #${channel.name}? This cannot be undone.`)) return
    deleting = true
    try {
      await svelteApi.delete(`/channels/${channel.id}`)
      dispatch('close')
    } catch (e: unknown) {
      console.error('Failed to delete channel', e)
    } finally {
      deleting = false
    }
  }

  function handleClose() {
    dispatch('close')
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose()
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open && channel}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4"
    data-testid="channel-settings-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      on:click={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close channel settings"
    ></div>

    <!-- Modal -->
    <div
      class="relative bg-bg-surface-1 rounded-r-3 shadow-2xl ring-1 ring-border-1 w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden"
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border-1 shrink-0">
        <div class="flex items-center space-x-2">
          <svelte:component
            this={channel.channel_type === 'private' ? Lock : Hash}
            class="w-5 h-5 text-text-3"
          />
          <h2 class="text-lg font-semibold text-text-1">Channel Settings</h2>
        </div>
        <button
          type="button"
          on:click={handleClose}
          class="flex h-10 w-10 items-center justify-center rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
          aria-label="Close"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-border-1 px-6 shrink-0">
        {#each tabs as tab (tab.id)}
          <button
            type="button"
            on:click={() => (activeTab = tab.id)}
            class="flex items-center px-4 py-3 text-sm font-medium border-b-2 -mb-px transition-colors"
            class:border-brand={activeTab === tab.id}
            class:text-brand={activeTab === tab.id}
            class:border-transparent={activeTab !== tab.id}
            class:text-text-3={activeTab !== tab.id}
            class:hover:text-text-2={activeTab !== tab.id}
          >
            <svelte:component this={tab.icon} class="w-4 h-4 mr-2" />
            {tab.label}
          </button>
        {/each}
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if activeTab === 'overview'}
          <div class="space-y-5">
            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Channel Name</label>
              <input
                type="text"
                bind:value={channelName}
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm"
                disabled={loading}
              />
            </div>

            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Display Name</label>
              <input
                type="text"
                bind:value={displayName}
                placeholder="Optional display name"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm"
                disabled={loading}
              />
            </div>

            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Purpose</label>
              <textarea
                bind:value={purpose}
                rows="2"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-brand focus:border-transparent text-sm"
                placeholder="What is this channel about?"
                disabled={loading}
              ></textarea>
            </div>

            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Header</label>
              <textarea
                bind:value={headerText}
                rows="2"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-brand focus:border-transparent text-sm"
                placeholder="Channel header (shown at the top)"
                disabled={loading}
              ></textarea>
            </div>

            <!-- Danger Zone -->
            <div class="pt-6 border-t border-border-1">
              <h4 class="text-sm font-semibold text-danger mb-3">Danger Zone</h4>
              <button
                type="button"
                on:click={handleDelete}
                disabled={deleting}
                class="flex items-center px-4 py-2 text-sm font-medium text-danger border border-danger/30 rounded-lg hover:bg-danger/10 transition-colors disabled:opacity-50"
              >
                <Trash2 class="w-4 h-4 mr-2" />
                {deleting ? 'Deleting...' : 'Delete Channel'}
              </button>
            </div>
          </div>
        {:else if activeTab === 'members'}
          <div class="space-y-6">
            <!-- Add Member -->
            <div class="space-y-3">
              <h4 class="text-sm font-medium text-text-1">Add Member</h4>
              <div class="relative">
                <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <Search class="h-4 w-4 text-text-4" />
                </div>
                <input
                  type="text"
                  bind:value={searchQuery}
                  placeholder="Search team members to add"
                  class="block w-full pl-10 pr-3 py-2 border border-border-2 rounded-lg leading-5 bg-bg-surface-1 placeholder-text-4 focus:outline-none focus:placeholder-text-4 focus:ring-1 focus:ring-brand focus:border-brand sm:text-sm transition duration-150 ease-in-out"
                />
              </div>

              {#if searchQuery && searchResults.length > 0}
                <div
                  class="bg-bg-surface-1 rounded-lg border border-border-1 divide-y divide-border-1 max-h-48 overflow-y-auto"
                >
                  {#each searchResults as user (user.user_id)}
                    <div class="flex items-center justify-between p-3 hover:bg-bg-surface-2 transition-colors">
                      <div class="flex items-center space-x-3">
                        <div
                          class="w-8 h-8 rounded-full bg-brand/10 flex items-center justify-center text-brand font-medium text-sm"
                        >
                          {(user.display_name || user.username).charAt(0).toUpperCase()}
                        </div>
                        <div>
                          <p class="text-sm font-medium text-text-1">{user.display_name || user.username}</p>
                          <p class="text-xs text-text-3">@{user.username}</p>
                        </div>
                      </div>
                      <button
                        type="button"
                        on:click={() => addMember(user.user_id)}
                        disabled={addingMember === user.user_id}
                        class="p-1.5 bg-brand/10 text-brand rounded-lg hover:bg-brand/20 transition-colors disabled:opacity-50"
                      >
                        <Plus class="w-4 h-4" />
                      </button>
                    </div>
                  {/each}
                </div>
              {:else if searchQuery && searchResults.length === 0}
                <div class="text-center py-4 text-sm text-text-3">No matching team members found</div>
              {/if}
            </div>

            <!-- Member List -->
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h4 class="text-sm font-medium text-text-1">Channel Members</h4>
                <span class="text-xs text-text-3">{channelMembers.length} members</span>
              </div>

              <div
                class="bg-bg-surface-2 rounded-lg border border-border-1 divide-y divide-border-1 max-h-64 overflow-y-auto"
              >
                {#if membersLoading}
                  <div class="p-4 text-center text-text-3 text-sm">Loading members...</div>
                {:else}
                  {#each channelMembers as member (member.user_id)}
                    <div class="flex items-center justify-between p-3">
                      <div class="flex items-center space-x-3">
                        <div
                          class="w-8 h-8 rounded-full bg-bg-surface-2 flex items-center justify-center text-text-2 font-medium text-sm"
                        >
                          {(member.display_name || member.username).charAt(0).toUpperCase()}
                        </div>
                        <div>
                          <div class="flex items-center space-x-2">
                            <p class="text-sm font-medium text-text-1">{member.display_name || member.username}</p>
                            {#if member.role === 'admin' || member.role === 'owner'}
                              <span
                                class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-warning/10 text-warning border border-warning/20"
                              >
                                {member.role}
                              </span>
                            {/if}
                          </div>
                          <p class="text-xs text-text-3">@{member.username}</p>
                        </div>
                      </div>

                      {#if member.user_id !== $authStore.user?.id}
                        <div class="flex items-center">
                          <button
                            type="button"
                            on:click={() => removeMember(member.user_id)}
                            disabled={removingMember === member.user_id}
                            class="p-1.5 text-text-4 hover:text-danger hover:bg-danger/10 rounded-lg transition-colors disabled:opacity-50"
                            title="Remove member"
                          >
                            <UserMinus class="w-4 h-4" />
                          </button>
                        </div>
                      {/if}
                    </div>
                  {/each}

                  {#if channelMembers.length === 0}
                    <div class="p-8 text-center text-text-3 text-sm">No members found</div>
                  {/if}
                {/if}
              </div>
            </div>
          </div>
        {:else if activeTab === 'permissions'}
          <div class="text-center py-10 text-text-3">
            <Shield class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Permission settings coming soon</p>
            <p class="text-sm mt-1">Configure roles and access control</p>
          </div>
        {:else if activeTab === 'notifications'}
          <div class="text-center py-10 text-text-3">
            <Bell class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Notification settings coming soon</p>
            <p class="text-sm mt-1">Configure channel-specific notification preferences</p>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-border-1 flex justify-end space-x-3 shrink-0">
        <button
          type="button"
          on:click={handleClose}
          class="px-4 py-2 text-sm font-medium text-text-2 bg-bg-surface-2 rounded-lg hover:bg-bg-surface-1 border border-border-2 transition-colors"
        >
          Cancel
        </button>
        <button
          type="button"
          on:click={handleSave}
          disabled={loading}
          class="px-4 py-2 text-sm font-medium text-brand-foreground bg-brand rounded-lg hover:bg-brand-hover transition-colors disabled:opacity-50"
        >
          {loading ? 'Saving...' : 'Save Changes'}
        </button>
      </div>
    </div>
  </div>
{/if}
