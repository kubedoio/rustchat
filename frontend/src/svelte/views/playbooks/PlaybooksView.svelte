<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { ArrowRight, BookOpen, CheckCircle2, Loader2, Play, Plus } from 'lucide-svelte'
  import { authStore } from '../../stores/auth'
  import { chatStore } from '../../stores/chat'
  import { playbooksStore } from '../../stores/playbooks'
  import type { Playbook } from '../../stores/playbooks'

  let activeTab = $state<'library' | 'runs'>('library')
  let teamId = $state<string | null>(null)

  function navigate(path: string) {
    window.history.pushState({}, '', path)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }

  async function loadPlaybooks() {
    if (get(chatStore).teams.length === 0) {
      await chatStore.fetchTeams()
    }

    teamId = get(chatStore).teams[0]?.id ?? null
    if (!teamId) return

    await Promise.all([
      playbooksStore.fetchPlaybooks(teamId),
      playbooksStore.fetchRuns(teamId),
    ])
  }

  async function startRun(playbook: Playbook) {
    if (!teamId || $playbooksStore.saving) return

    const run = await playbooksStore.startRun(teamId, {
      playbook_id: playbook.id,
      name: playbook.name,
      owner_id: $authStore.user?.id,
    })
    navigate(`/runs/${run.run.id}`)
  }

  function formatDate(value: string) {
    return new Date(value).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
  }

  onMount(() => {
    void loadPlaybooks()
  })
</script>

<main class="min-h-screen bg-bg-app text-text-1">
  <header class="border-b border-border-1 bg-bg-surface-1 px-6 py-5">
    <div class="mx-auto flex max-w-6xl items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <div class="flex h-11 w-11 items-center justify-center rounded-r-3 bg-brand/10 text-brand">
          <BookOpen class="h-5 w-5" />
        </div>
        <div>
          <h1 class="text-xl font-bold">Playbooks</h1>
          <p class="text-sm text-text-3">Create repeatable workflows and track active runs.</p>
        </div>
      </div>

      <button
        type="button"
        class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2.5 text-sm font-semibold text-brand-foreground transition-standard hover:bg-brand-hover"
        on:click={() => navigate('/playbooks/new')}
      >
        <Plus class="h-4 w-4" />
        Create Playbook
      </button>
    </div>
  </header>

  <section class="mx-auto max-w-6xl px-6 py-6">
    <div class="mb-5 flex gap-2 border-b border-border-1">
      <button
        type="button"
        class="border-b-2 px-1 pb-3 text-sm font-semibold transition-standard {activeTab === 'library' ? 'border-brand text-brand' : 'border-transparent text-text-3 hover:text-text-1'}"
        on:click={() => (activeTab = 'library')}
      >
        Library
      </button>
      <button
        type="button"
        class="border-b-2 px-1 pb-3 text-sm font-semibold transition-standard {activeTab === 'runs' ? 'border-brand text-brand' : 'border-transparent text-text-3 hover:text-text-1'}"
        on:click={() => (activeTab = 'runs')}
      >
        Runs
      </button>
    </div>

    {#if $playbooksStore.error}
      <div class="mb-4 rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
        {$playbooksStore.error}
      </div>
    {/if}

    {#if $playbooksStore.loading}
      <div class="flex items-center justify-center rounded-r-3 border border-border-1 bg-bg-surface-1 py-16 text-text-3">
        <Loader2 class="mr-2 h-5 w-5 animate-spin" />
        Loading playbooks...
      </div>
    {:else if activeTab === 'library'}
      <div class="grid gap-4 md:grid-cols-2">
        {#each $playbooksStore.playbooks as playbook (playbook.id)}
          <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1 transition-standard hover:-translate-y-0.5 hover:shadow-2">
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0">
                <h2 class="truncate text-lg font-bold">{playbook.name}</h2>
                <p class="mt-1 line-clamp-2 text-sm text-text-3">{playbook.description || 'No description yet.'}</p>
              </div>
              <span class="rounded-full bg-bg-surface-2 px-2.5 py-1 text-xs font-semibold text-text-3">
                {playbook.is_public ? 'Public' : 'Private'}
              </span>
            </div>

            <div class="mt-5 flex flex-wrap items-center justify-between gap-3">
              <p class="text-xs text-text-3">Updated {formatDate(playbook.updated_at)}</p>
              <div class="flex gap-2">
                <button
                  type="button"
                  class="rounded-r-2 border border-border-1 px-3 py-2 text-sm font-medium text-text-2 transition-standard hover:bg-bg-surface-2"
                  on:click={() => navigate(`/playbooks/${playbook.id}/edit`)}
                >
                  Edit
                </button>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-3 py-2 text-sm font-semibold text-brand-foreground transition-standard hover:bg-brand-hover disabled:opacity-60"
                  disabled={$playbooksStore.saving}
                  on:click={() => startRun(playbook)}
                >
                  <Play class="h-4 w-4" />
                  Run
                </button>
              </div>
            </div>
          </article>
        {:else}
          <div class="rounded-r-3 border border-dashed border-border-1 bg-bg-surface-1 py-16 text-center md:col-span-2">
            <BookOpen class="mx-auto mb-3 h-10 w-10 text-text-4" />
            <h2 class="text-lg font-semibold">No playbooks yet</h2>
            <p class="mt-1 text-sm text-text-3">Create your first workflow to make repeatable work easier.</p>
          </div>
        {/each}
      </div>
    {:else}
      <div class="space-y-3">
        {#each $playbooksStore.runs as run (run.id)}
          <button
            type="button"
            class="flex w-full items-center justify-between gap-4 rounded-r-3 border border-border-1 bg-bg-surface-1 p-4 text-left shadow-1 transition-standard hover:bg-bg-surface-2"
            on:click={() => navigate(`/runs/${run.id}`)}
          >
            <div class="flex items-center gap-3">
              <div class="flex h-10 w-10 items-center justify-center rounded-r-2 bg-success/10 text-success">
                <CheckCircle2 class="h-5 w-5" />
              </div>
              <div>
                <h2 class="font-semibold">{run.name}</h2>
                <p class="text-xs text-text-3">{run.status.replace('_', ' ')} since {formatDate(run.started_at)}</p>
              </div>
            </div>
            <ArrowRight class="h-5 w-5 text-text-3" />
          </button>
        {:else}
          <div class="rounded-r-3 border border-dashed border-border-1 bg-bg-surface-1 py-16 text-center">
            <Play class="mx-auto mb-3 h-10 w-10 text-text-4" />
            <h2 class="text-lg font-semibold">No active runs</h2>
            <p class="mt-1 text-sm text-text-3">Start a playbook run to track progress here.</p>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</main>
