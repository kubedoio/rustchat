<script lang="ts">
  import { onMount } from 'svelte'
  import { ArrowLeft, CheckCircle2, Circle, Loader2, Send } from 'lucide-svelte'
  import { playbooksStore } from '../../stores/playbooks'

  let runId = $state<string | null>(null)
  let loading = $state(false)
  let error = $state<string | null>(null)
  let updateMessage = $state('')

  const run = $derived($playbooksStore.currentRun?.run ?? null)
  const tasks = $derived($playbooksStore.currentRun?.tasks ?? [])
  const progress = $derived($playbooksStore.currentRun?.progress ?? { total: 0, completed: 0, in_progress: 0, pending: 0 })
  const completion = $derived(progress.total > 0 ? Math.round((progress.completed / progress.total) * 100) : 0)

  function navigate(path: string) {
    window.history.pushState({}, '', path)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }

  async function load() {
    const match = window.location.pathname.match(/^\/runs\/([^/]+)$/)
    runId = match?.[1] ?? null
    if (!runId) {
      error = 'Run not found.'
      return
    }

    loading = true
    error = null
    try {
      await Promise.all([
        playbooksStore.fetchRun(runId),
        playbooksStore.fetchStatusUpdates(runId),
      ])
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : 'Failed to load run.'
    } finally {
      loading = false
    }
  }

  async function toggleTask(taskId: string, done: boolean) {
    if (!runId) return
    await playbooksStore.updateRunTask(runId, taskId, { status: done ? 'pending' : 'done' })
  }

  async function finishRun() {
    if (!runId) return
    await playbooksStore.finishRun(runId)
  }

  async function postUpdate() {
    if (!runId || !updateMessage.trim()) return
    await playbooksStore.createStatusUpdate(runId, updateMessage.trim())
    updateMessage = ''
  }

  function formatDate(value: string | null) {
    if (!value) return 'Not finished'
    return new Date(value).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
  }

  onMount(() => {
    void load()
  })
</script>

<main class="min-h-screen bg-bg-app text-text-1">
  <header class="border-b border-border-1 bg-bg-surface-1 px-6 py-4">
    <div class="mx-auto flex max-w-5xl items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <button
          type="button"
          class="flex h-9 w-9 items-center justify-center rounded-r-2 text-text-3 transition-standard hover:bg-bg-surface-2 hover:text-text-1"
          aria-label="Back to playbooks"
          on:click={() => navigate('/playbooks')}
        >
          <ArrowLeft class="h-5 w-5" />
        </button>
        <div>
          <h1 class="text-xl font-bold">{run?.name ?? 'Playbook Run'}</h1>
          <p class="text-sm text-text-3">{run ? `${run.status.replace('_', ' ')} - started ${formatDate(run.started_at)}` : 'Loading run details'}</p>
        </div>
      </div>
      {#if run && run.status === 'in_progress'}
        <button
          type="button"
          class="inline-flex items-center gap-2 rounded-r-2 bg-success px-4 py-2.5 text-sm font-semibold text-white transition-standard hover:opacity-90"
          on:click={finishRun}
        >
          <CheckCircle2 class="h-4 w-4" />
          Finish Run
        </button>
      {/if}
    </div>
  </header>

  <section class="mx-auto grid max-w-5xl gap-6 px-6 py-6 lg:grid-cols-[1fr_20rem]">
    {#if error}
      <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger lg:col-span-2" role="alert">
        {error}
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center rounded-r-3 border border-border-1 bg-bg-surface-1 py-16 text-text-3 lg:col-span-2">
        <Loader2 class="mr-2 h-5 w-5 animate-spin" />
        Loading run...
      </div>
    {:else}
      <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <div class="mb-5">
          <div class="mb-2 flex items-center justify-between text-sm">
            <span class="font-semibold">Progress</span>
            <span class="text-text-3">{progress.completed} of {progress.total} complete</span>
          </div>
          <div class="h-2 overflow-hidden rounded-full bg-bg-surface-2">
            <div class="h-full rounded-full bg-success transition-all" style={`width: ${completion}%`}></div>
          </div>
        </div>

        <div class="space-y-3">
          {#each tasks as task (task.id)}
            <button
              type="button"
              class="flex w-full items-start gap-3 rounded-r-2 border border-border-1 p-3 text-left transition-standard hover:bg-bg-surface-2"
              on:click={() => toggleTask(task.id, task.status === 'done')}
            >
              {#if task.status === 'done'}
                <CheckCircle2 class="mt-0.5 h-5 w-5 shrink-0 text-success" />
              {:else}
                <Circle class="mt-0.5 h-5 w-5 shrink-0 text-text-4" />
              {/if}
              <span>
                <span class="block text-sm font-semibold">Task {task.task_id}</span>
                <span class="block text-xs text-text-3">{task.status.replace('_', ' ')}{task.notes ? ` - ${task.notes}` : ''}</span>
              </span>
            </button>
          {:else}
            <div class="rounded-r-2 border border-dashed border-border-1 py-10 text-center text-sm text-text-3">
              No tasks are attached to this run.
            </div>
          {/each}
        </div>
      </section>

      <aside class="space-y-4">
        <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
          <h2 class="mb-3 text-sm font-bold uppercase tracking-[0.16em] text-text-3">Run Details</h2>
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="text-text-3">Status</dt>
              <dd class="font-semibold capitalize">{run?.status.replace('_', ' ') ?? 'Unknown'}</dd>
            </div>
            <div>
              <dt class="text-text-3">Started</dt>
              <dd>{formatDate(run?.started_at ?? null)}</dd>
            </div>
            <div>
              <dt class="text-text-3">Finished</dt>
              <dd>{formatDate(run?.finished_at ?? null)}</dd>
            </div>
          </dl>
        </section>

        <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
          <h2 class="mb-3 text-sm font-bold uppercase tracking-[0.16em] text-text-3">Updates</h2>
          <div class="mb-3 space-y-2">
            {#each $playbooksStore.statusUpdates as update (update.id)}
              <article class="rounded-r-2 bg-bg-surface-2 p-3 text-sm">
                <p>{update.message}</p>
                <p class="mt-1 text-xs text-text-3">{formatDate(update.created_at)}</p>
              </article>
            {:else}
              <p class="text-sm text-text-3">No status updates yet.</p>
            {/each}
          </div>
          <div class="flex gap-2">
            <input
              class="min-w-0 flex-1 rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm"
              bind:value={updateMessage}
              placeholder="Post an update"
              on:keydown={(event) => { if (event.key === 'Enter') void postUpdate() }}
            />
            <button type="button" class="rounded-r-2 bg-brand p-2 text-brand-foreground" aria-label="Post update" on:click={postUpdate}>
              <Send class="h-4 w-4" />
            </button>
          </div>
        </section>
      </aside>
    {/if}
  </section>
</main>
