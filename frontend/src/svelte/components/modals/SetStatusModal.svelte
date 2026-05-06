<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { X } from 'lucide-svelte'
  import { svelteApi } from '../../stores/http'
  import { authStore } from '../../stores/auth'

  export let open = false

  const dispatch = createEventDispatcher<{
    close: void
  }>()

  let emoji = '💬'
  let text = ''
  let duration = ''

  // Initialize from current status when opening
  $: if (open && $authStore.user) {
    text = $authStore.user.status_text || ''
    emoji = $authStore.user.status_emoji || '💬'
    duration = ''
  }

  const durations = [
    { label: "Don't clear", value: '' },
    { label: '30 minutes', value: 'thirty_minutes' },
    { label: '1 hour', value: 'one_hour' },
    { label: '4 hours', value: 'four_hours' },
    { label: 'Today', value: 'today' },
    { label: 'This week', value: 'this_week' },
    { label: 'Custom date and time', value: 'custom_date_time' },
  ]

  async function save() {
    await svelteApi.put('/users/me/status', {
      text,
      emoji,
      duration: duration || undefined,
    })
    dispatch('close')
  }

  async function clear() {
    await svelteApi.put('/users/me/status', {
      text: '',
      emoji: '',
      duration: undefined,
    })
    dispatch('close')
  }

  function handleClose() {
    dispatch('close')
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose()
    } else if (event.key === 'Enter') {
      save()
    }
  }
</script>

<svelte:window on:keydown={(e) => open && handleKeydown(e)} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4"
    data-testid="set-status-modal"
    role="dialog"
    aria-modal="true"
  >
    <!-- Backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm transition-opacity"
      on:click={handleClose}
      role="button"
      tabindex="-1"
      aria-label="Close modal"
    ></div>

    <!-- Modal Panel -->
    <div class="relative w-full max-w-lg overflow-hidden rounded-lg bg-white shadow-xl transform transition-all">
      <!-- Header -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200">
        <h3 class="text-lg font-semibold text-gray-900">Set a status</h3>
        <button
          on:click={handleClose}
          class="text-gray-400 hover:text-gray-500 transition-colors"
          aria-label="Close"
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-6 space-y-4">
        <!-- Status Input -->
        <div class="flex space-x-2">
          <button
            type="button"
            class="flex-shrink-0 w-10 h-10 flex items-center justify-center rounded-md border border-gray-300 hover:bg-gray-50 text-xl transition-colors"
            aria-label="Choose emoji"
          >
            {emoji}
          </button>
          <input
            bind:value={text}
            type="text"
            placeholder="What's your status?"
            class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
            on:keydown={handleKeydown}
            autofocus
          />
        </div>

        <!-- Clear After -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1" for="status-duration">Clear after</label>
          <select
            id="status-duration"
            bind:value={duration}
            class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
          >
            {#each durations as opt (opt.value)}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
      </div>

      <!-- Footer -->
      <div class="bg-gray-50 px-4 py-3 sm:flex sm:flex-row-reverse border-t border-gray-200">
        <button
          type="button"
          class="inline-flex w-full justify-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 sm:ml-3 sm:w-auto transition-colors"
          on:click={save}
        >
          Save
        </button>
        {#if $authStore.user?.status_text}
          <button
            type="button"
            class="mt-3 inline-flex w-full justify-center rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 sm:mt-0 sm:w-auto sm:mr-auto transition-colors"
            on:click={clear}
          >
            Clear Status
          </button>
        {/if}
        <button
          type="button"
          class="mt-3 inline-flex w-full justify-center rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 sm:mt-0 sm:w-auto transition-colors"
          on:click={handleClose}
        >
          Cancel
        </button>
      </div>
    </div>
  </div>
{/if}
