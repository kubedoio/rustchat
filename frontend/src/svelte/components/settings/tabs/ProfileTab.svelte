<script lang="ts">
  import { tick } from 'svelte'
  import { authStore } from '../../../stores/auth'
  import { svelteApi } from '../../../stores/http'

  let firstName = $state('')
  let lastName = $state('')
  let nickname = $state('')
  let position = $state('')
  let displayName = $state('')
  let syncedUserId = $state<string | null>(null)
  let saving = $state(false)
  let error = $state<string | null>(null)
  let success = $state<string | null>(null)

  function syncFromUser() {
    const user = $authStore.user
    if (!user || syncedUserId === user.id) return

    syncedUserId = user.id
    firstName = user.first_name || user.firstName || ''
    lastName = user.last_name || user.lastName || ''
    nickname = user.nickname || ''
    position = user.position || ''
    displayName = user.display_name || user.displayName || ''
  }

  $effect(() => {
    syncFromUser()
  })

  async function handleSave() {
    const user = $authStore.user
    if (!user || saving) {
      return
    }

    saving = true
    error = null
    success = null

    try {
      await svelteApi.put('/users/me/patch', {
        first_name: firstName.trim(),
        last_name: lastName.trim(),
        nickname: nickname.trim(),
        position: position.trim(),
      }, { baseURL: '/api/v4' })

      await svelteApi.put(`/users/${user.id}`, {
        display_name: displayName.trim(),
        username: user.username,
      })

      syncedUserId = null
      await authStore.fetchMe()
      await tick()
      success = 'Profile saved.'
    } catch (saveError) {
      error = saveError instanceof Error ? saveError.message : 'Failed to save profile.'
    } finally {
      saving = false
    }
  }
</script>

<div class="space-y-6">
  <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
    <div class="border-b border-border-1 pb-3">
      <h4 class="text-sm font-semibold tracking-[0.01em] text-text-1">Profile Information</h4>
      <p class="mt-1 text-xs text-text-3">Keep your identity details current so teammates can recognize and trust who is speaking.</p>
    </div>

    <div class="mt-4 space-y-4">
      <!-- Display Name -->
      <div>
        <label class="mb-1 block text-sm font-medium text-text-1">Display Name</label>
        <input
          type="text"
          bind:value={displayName}
          placeholder="Your Name"
          class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15"
        />
      </div>

      <!-- First & Last Name -->
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label class="mb-1 block text-sm font-medium text-text-1">First Name</label>
          <input
            type="text"
            bind:value={firstName}
            placeholder="John"
            class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15"
          />
        </div>
        <div>
          <label class="mb-1 block text-sm font-medium text-text-1">Last Name</label>
          <input
            type="text"
            bind:value={lastName}
            placeholder="Doe"
            class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15"
          />
        </div>
      </div>

      <!-- Nickname -->
      <div>
        <label class="mb-1 block text-sm font-medium text-text-1">Nickname</label>
        <input
          type="text"
          bind:value={nickname}
          placeholder="Johnny"
          class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15"
        />
      </div>

      <!-- Position -->
      <div>
        <label class="mb-1 block text-sm font-medium text-text-1">Position</label>
        <input
          type="text"
          bind:value={position}
          placeholder="Software Engineer"
          class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15"
        />
      </div>

      <!-- Email (read-only) -->
      <div>
        <label class="mb-1 block text-sm font-medium text-text-2">Email</label>
        <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 px-3 py-2 text-sm text-text-3 break-all">
          {$authStore.user?.email || '—'}
        </div>
      </div>

      <!-- Username (read-only) -->
      <div>
        <label class="mb-1 block text-sm font-medium text-text-2">Username</label>
        <div class="rounded-r-2 border border-border-1 bg-bg-surface-2 px-3 py-2 text-sm text-text-3 break-all">
          {$authStore.user?.username || '—'}
        </div>
      </div>

      <!-- Save Button -->
      {#if error}
        <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger" role="alert">
          {error}
        </div>
      {/if}

      {#if success}
        <div class="rounded-r-2 border border-success/30 bg-success/10 px-3 py-2 text-sm text-success" role="status">
          {success}
        </div>
      {/if}

      <div class="flex justify-end">
        <button
          type="button"
          onclick={handleSave}
          disabled={saving || !$authStore.user}
          class="rounded-r-2 bg-brand px-4 py-2.5 text-sm font-medium text-brand-foreground transition-standard hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-60"
        >
          {saving ? 'Saving...' : 'Save Profile'}
        </button>
      </div>
    </div>
  </div>
</div>
