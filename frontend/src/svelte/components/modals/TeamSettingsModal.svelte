<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { X, Settings, Users, Cog, Trash2, LogOut, Search, Plus, UserMinus } from 'lucide-svelte'
  import { focusTrap } from '../../lib/focusTrap'
  import { svelteApi } from '../../stores/http'
  import { authStore } from '../../stores/auth'
  import { teamStore } from '../../stores/team'

  export let team: { id: string; name: string; display_name: string } | null = null
  export let open = false

  const dispatch = createEventDispatcher<{ close: void }>()

  interface TeamDetail {
    id: string
    name: string
    display_name: string
    description?: string
    invite_id?: string
    is_public?: boolean
    allow_open_invite?: boolean
  }

  interface UserItem {
    id: string
    username: string
    display_name?: string
  }

  let activeTab = 'overview'
  let loading = false
  let leaving = false
  let deleting = false

  let displayName = ''
  let description = ''
  let teamDetail: TeamDetail | null = null

  let searchQuery = ''
  let searchResults: UserItem[] = []
  let searching = false
  let addingMember: string | null = null
  let removingMember: string | null = null

  const tabs = [
    { id: 'overview', label: 'Overview', icon: Settings },
    { id: 'members', label: 'Members', icon: Users },
    { id: 'settings', label: 'Settings', icon: Cog },
  ]

  $: if (open && team) {
    displayName = team.display_name || ''
    description = ''
    activeTab = 'overview'
    searchQuery = ''
    searchResults = []
    fetchTeamDetail()
    teamStore.fetchMembers(team.id)
  }

  async function fetchTeamDetail() {
    if (!team) return
    try {
      const { data } = await svelteApi.get<TeamDetail>(`/teams/${team.id}`)
      teamDetail = data
      description = data.description || ''
    } catch (e) {
      console.error('Failed to fetch team details', e)
    }
  }

  let searchTimeout: ReturnType<typeof setTimeout>
  function onSearchInput() {
    clearTimeout(searchTimeout)
    searchTimeout = setTimeout(handleSearch, 300)
  }

  async function handleSearch() {
    if (!searchQuery.trim()) {
      searchResults = []
      return
    }
    searching = true
    try {
      const { data } = await svelteApi.get<UserItem[]>(
        `/users?q=${encodeURIComponent(searchQuery)}&per_page=5`,
      )
      const memberIds = new Set(
        $teamStore.membersByTeam[team?.id ?? '']?.map((m) => m.user_id) ?? [],
      )
      searchResults = data.filter((u) => !memberIds.has(u.id))
    } catch (e) {
      console.error('Search failed', e)
    } finally {
      searching = false
    }
  }

  async function addMember(user: UserItem) {
    if (!team) return
    addingMember = user.id
    try {
      await svelteApi.post(`/teams/${team.id}/members`, { user_id: user.id })
      await teamStore.fetchMembers(team.id)
      searchResults = searchResults.filter((u) => u.id !== user.id)
    } catch (e: unknown) {
      console.error('Failed to add member', e)
    } finally {
      addingMember = null
    }
  }

  async function removeMember(userId: string) {
    if (!team) return
    if (!confirm('Are you sure you want to remove this member?')) return
    removingMember = userId
    try {
      await svelteApi.delete(`/teams/${team.id}/members/${userId}`)
      await teamStore.fetchMembers(team.id)
    } catch (e: unknown) {
      console.error('Failed to remove member', e)
    } finally {
      removingMember = null
    }
  }

  async function handleSave() {
    if (!team) return
    loading = true
    try {
      await svelteApi.put(`/teams/${team.id}`, {
        display_name: displayName.trim() || undefined,
        description: description.trim() || undefined,
        is_public: teamDetail?.is_public,
        allow_open_invite: teamDetail?.allow_open_invite,
      })
      dispatch('close')
    } catch (e: unknown) {
      console.error('Failed to update team', e)
    } finally {
      loading = false
    }
  }

  async function handleDelete() {
    if (!team) return
    if (
      !confirm(
        `Are you sure you want to delete "${team.display_name || team.name}"? This will delete all channels and messages. This cannot be undone.`,
      )
    )
      return
    deleting = true
    try {
      await svelteApi.delete(`/teams/${team.id}`)
      dispatch('close')
    } catch (e: unknown) {
      console.error('Failed to delete team', e)
    } finally {
      deleting = false
    }
  }

  async function handleLeave() {
    if (!team) return
    if (!confirm(`Are you sure you want to leave "${team.display_name || team.name}"?`)) return
    leaving = true
    try {
      await teamStore.leaveTeam(team.id)
      dispatch('close')
    } catch (e: unknown) {
      console.error('Failed to leave team', e)
    } finally {
      leaving = false
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

  $: currentTeamMembers = team ? $teamStore.membersByTeam[team.id] ?? [] : []
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open && team}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4"
    data-testid="team-settings-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      on:click={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close team settings"
      transition:fade={{ duration: 150, easing: cubicOut }}
    ></div>

    <!-- Modal -->
    <div
      class="relative bg-bg-surface-1 rounded-r-3 shadow-2xl ring-1 ring-border-1 w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden"
      use:focusTrap
      transition:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border-1 shrink-0">
        <div class="flex items-center space-x-3">
          <div
            class="flex h-10 w-10 items-center justify-center rounded-lg bg-brand text-lg font-bold text-brand-foreground"
          >
            {(team.display_name || team.name).charAt(0).toUpperCase()}
          </div>
          <div>
            <h2 class="text-lg font-semibold text-text-1">{team.display_name || team.name}</h2>
            <p class="text-sm text-text-3">Team Settings</p>
          </div>
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
              <label class="block text-sm font-medium text-text-2 mb-1">Team Name</label>
              <div class="px-3 py-2 bg-bg-surface-2 rounded-lg text-text-3 text-sm">
                {team.name}
              </div>
              <p class="mt-1 text-xs text-text-3">Team identifier cannot be changed</p>
            </div>

            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Display Name</label>
              <input
                type="text"
                bind:value={displayName}
                placeholder="My Team"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand focus:border-transparent text-sm"
                disabled={loading}
              />
            </div>

            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Description</label>
              <textarea
                bind:value={description}
                rows="3"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-brand focus:border-transparent text-sm"
                placeholder="What is this team about?"
                disabled={loading}
              ></textarea>
            </div>

            <!-- Danger Zone -->
            <div class="pt-6 border-t border-border-1">
              <h4 class="text-sm font-semibold text-danger mb-3">Danger Zone</h4>
              <div class="space-y-3">
                <button
                  type="button"
                  on:click={handleLeave}
                  disabled={leaving || deleting}
                  class="flex items-center px-4 py-2 text-sm font-medium text-danger border border-danger/30 rounded-lg hover:bg-danger/10 transition-colors disabled:opacity-50"
                >
                  <LogOut class="w-4 h-4 mr-2" />
                  {leaving ? 'Leaving...' : 'Leave Team'}
                </button>

                <button
                  type="button"
                  on:click={handleDelete}
                  disabled={deleting || leaving}
                  class="flex items-center px-4 py-2 text-sm font-medium text-danger border border-red-300 rounded-lg hover:bg-red-50 transition-colors disabled:opacity-50"
                >
                  <Trash2 class="w-4 h-4 mr-2" />
                  {deleting ? 'Deleting...' : 'Delete Team'}
                </button>
              </div>
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
                  on:input={onSearchInput}
                  placeholder="Search users by name or username"
                  class="block w-full pl-10 pr-3 py-2 border border-border-2 rounded-lg leading-5 bg-bg-surface-1 placeholder-text-4 focus:outline-none focus:placeholder-text-4 focus:ring-1 focus:ring-brand focus:border-brand sm:text-sm transition duration-150 ease-in-out"
                />
                {#if searching}
                  <div class="absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none">
                    <div class="animate-spin h-4 w-4 border-2 border-text-4 border-t-transparent rounded-full"></div>
                  </div>
                {/if}
              </div>

              {#if searchResults.length > 0}
                <div
                  class="bg-bg-surface-1 rounded-lg border border-border-1 divide-y divide-border-1 max-h-48 overflow-y-auto"
                >
                  {#each searchResults as user (user.id)}
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
                        on:click={() => addMember(user)}
                        disabled={addingMember === user.id}
                        class="p-1.5 bg-brand/10 text-brand rounded-lg hover:bg-brand/20 transition-colors disabled:opacity-50"
                      >
                        <Plus class="w-4 h-4" />
                      </button>
                    </div>
                  {/each}
                </div>
              {:else if searchQuery && !searching}
                <div class="text-center py-4 text-sm text-text-3">No users found</div>
              {/if}
            </div>

            <!-- Member List -->
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h4 class="text-sm font-medium text-text-1">Team Members</h4>
                <span class="text-xs text-text-3">{currentTeamMembers.length} members</span>
              </div>

              <div class="bg-bg-surface-2 rounded-lg border border-border-1 divide-y divide-border-1">
                {#if $teamStore.loading && !currentTeamMembers.length}
                  <div class="p-4 text-center text-text-3 text-sm">Loading members...</div>
                {:else}
                  {#each currentTeamMembers as member (member.user_id)}
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

                  {#if currentTeamMembers.length === 0}
                    <div class="p-8 text-center text-text-3 text-sm">No members found</div>
                  {/if}
                {/if}
              </div>
            </div>
          </div>
        {:else if activeTab === 'settings'}
          <div class="text-center py-10 text-text-3">
            <Cog class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Team settings coming soon</p>
            <p class="text-sm mt-1">Configure visibility, invites, and advanced options</p>
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
