<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { ArrowLeft, Loader2, Plus, Save, Trash2 } from 'lucide-svelte'
  import { chatStore } from '../../stores/chat'
  import { playbooksStore } from '../../stores/playbooks'
  import type { ChecklistWithTasks } from '../../stores/playbooks'

  type EditableChecklist = ChecklistWithTasks

  let playbookId = $state<string | null>(null)
  let teamId = $state<string | null>(null)
  let loading = $state(false)
  let saving = $state(false)
  let error = $state<string | null>(null)
  let form = $state({
    name: '',
    description: '',
    icon: 'book',
    is_public: false,
    create_channel_on_run: true,
    channel_name_template: 'incident-{{date}}',
    keyword_triggers: '',
  })
  let checklists = $state<EditableChecklist[]>([])
  let originalChecklists = $state<EditableChecklist[]>([])

  const isEditing = $derived(Boolean(playbookId))

  function navigate(path: string) {
    window.history.pushState({}, '', path)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }

  function back() {
    navigate('/playbooks')
  }

  function makeChecklist(name = 'Default checklist'): EditableChecklist {
    return {
      id: `temp-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      playbook_id: playbookId ?? '',
      name,
      sort_order: checklists.length + 1,
      created_at: new Date().toISOString(),
      tasks: [],
    }
  }

  function addChecklist() {
    checklists = [...checklists, makeChecklist('New checklist')]
  }

  function removeChecklist(index: number) {
    checklists = checklists.filter((_, currentIndex) => currentIndex !== index)
  }

  function addTask(checklistIndex: number) {
    checklists = checklists.map((checklist, currentIndex) => {
      if (currentIndex !== checklistIndex) return checklist

      return {
        ...checklist,
        tasks: [
          ...checklist.tasks,
          {
            id: `temp-task-${Date.now()}-${Math.random().toString(36).slice(2)}`,
            checklist_id: checklist.id,
            title: '',
            description: '',
            default_assignee_id: null,
            assignee_role: null,
            due_after_minutes: null,
            slash_command: null,
            webhook_url: null,
            sort_order: checklist.tasks.length + 1,
            created_at: new Date().toISOString(),
          },
        ],
      }
    })
  }

  function removeTask(checklistIndex: number, taskIndex: number) {
    checklists = checklists.map((checklist, currentIndex) => {
      if (currentIndex !== checklistIndex) return checklist

      return {
        ...checklist,
        tasks: checklist.tasks.filter((_, currentTaskIndex) => currentTaskIndex !== taskIndex),
      }
    })
  }

  function isTemporaryChecklist(checklist: EditableChecklist) {
    return checklist.id.startsWith('temp-')
  }

  function isTemporaryTask(task: EditableChecklist['tasks'][number]) {
    return task.id.startsWith('temp-')
  }

  function taskPayload(task: EditableChecklist['tasks'][number], sortOrder?: number) {
    return {
      title: task.title.trim(),
      description: task.description?.trim() ?? '',
      sort_order: sortOrder ?? task.sort_order,
    }
  }

  function taskChanged(task: EditableChecklist['tasks'][number], originalTask: EditableChecklist['tasks'][number]) {
    const currentPayload = taskPayload(task)

    return currentPayload.title !== originalTask.title
      || currentPayload.description !== (originalTask.description ?? '')
  }

  async function createTasks(checklistId: string, tasks: EditableChecklist['tasks']) {
    for (const [taskIndex, task] of tasks.entries()) {
      const payload = taskPayload(task, taskIndex + 1)
      if (!payload.title) continue

      await playbooksStore.createTask(checklistId, {
        ...payload,
        description: payload.description || null,
      })
    }
  }

  async function createChecklistWithTasks(playbookId: string, checklist: EditableChecklist, sortOrder: number) {
    const createdChecklist = await playbooksStore.createChecklist(playbookId, {
      name: checklist.name.trim(),
      sort_order: sortOrder,
    })
    await createTasks(createdChecklist.id, checklist.tasks)
  }

  async function syncExistingChecklistTasks(checklist: EditableChecklist, originalChecklist: EditableChecklist) {
    const currentTaskIds = new Set(
      checklist.tasks
        .filter((task) => !isTemporaryTask(task))
        .map((task) => task.id),
    )

    for (const originalTask of originalChecklist.tasks) {
      if (!currentTaskIds.has(originalTask.id)) {
        await playbooksStore.deleteTask(originalTask.id)
      }
    }

    const originalTasksById = new Map(originalChecklist.tasks.map((task) => [task.id, task]))
    for (const [taskIndex, task] of checklist.tasks.entries()) {
      if (isTemporaryTask(task)) {
        const payload = taskPayload(task, taskIndex + 1)
        if (!payload.title) continue

        await playbooksStore.createTask(checklist.id, {
          ...payload,
          description: payload.description || null,
        })
        continue
      }

      const originalTask = originalTasksById.get(task.id)
      if (originalTask && taskChanged(task, originalTask)) {
        await playbooksStore.updateTask(task.id, taskPayload(task))
      }
    }
  }

  async function syncChecklists(savedPlaybookId: string) {
    const originalChecklistsById = new Map(originalChecklists.map((checklist) => [checklist.id, checklist]))
    const currentChecklistIds = new Set(
      checklists
        .filter((checklist) => !isTemporaryChecklist(checklist))
        .map((checklist) => checklist.id),
    )
    const replacedChecklistIds = new Set<string>()

    for (const [checklistIndex, checklist] of checklists.entries()) {
      if (isTemporaryChecklist(checklist)) {
        await createChecklistWithTasks(savedPlaybookId, checklist, checklistIndex + 1)
        continue
      }

      const originalChecklist = originalChecklistsById.get(checklist.id)
      if (!originalChecklist) continue

      if (checklist.name.trim() !== originalChecklist.name) {
        await createChecklistWithTasks(savedPlaybookId, checklist, checklistIndex + 1)
        replacedChecklistIds.add(checklist.id)
        continue
      }

      await syncExistingChecklistTasks(checklist, originalChecklist)
    }

    for (const originalChecklist of originalChecklists) {
      if (!currentChecklistIds.has(originalChecklist.id) || replacedChecklistIds.has(originalChecklist.id)) {
        await playbooksStore.deleteChecklist(savedPlaybookId, originalChecklist.id)
      }
    }
  }

  async function load() {
    const match = window.location.pathname.match(/^\/playbooks\/([^/]+)\/edit$/)
    playbookId = match?.[1] ?? null

    if (get(chatStore).teams.length === 0) {
      await chatStore.fetchTeams()
    }
    teamId = get(chatStore).teams[0]?.id ?? null

    if (!playbookId) {
      checklists = [makeChecklist()]
      originalChecklists = []
      return
    }

    loading = true
    error = null
    try {
      const playbook = await playbooksStore.fetchPlaybook(playbookId)
      form = {
        name: playbook.name,
        description: playbook.description ?? '',
        icon: playbook.icon ?? 'book',
        is_public: playbook.is_public,
        create_channel_on_run: playbook.create_channel_on_run,
        channel_name_template: playbook.channel_name_template ?? '',
        keyword_triggers: playbook.keyword_triggers?.join(', ') ?? '',
      }
      checklists = structuredClone(playbook.checklists ?? [])
      originalChecklists = structuredClone(playbook.checklists ?? [])
      if (checklists.length === 0) {
        checklists = [makeChecklist()]
      }
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : 'Failed to load playbook.'
    } finally {
      loading = false
    }
  }

  async function save() {
    if (!teamId || saving) return
    if (!form.name.trim()) {
      error = 'Playbook name is required.'
      return
    }

    saving = true
    error = null
    try {
      const payload = {
        name: form.name.trim(),
        description: form.description.trim(),
        icon: form.icon.trim(),
        is_public: form.is_public,
        create_channel_on_run: form.create_channel_on_run,
        channel_name_template: form.channel_name_template.trim(),
        keyword_triggers: form.keyword_triggers
          .split(',')
          .map((keyword) => keyword.trim())
          .filter(Boolean),
      }

      let savedPlaybookId = playbookId
      if (savedPlaybookId) {
        await playbooksStore.updatePlaybook(savedPlaybookId, payload)
      } else {
        const created = await playbooksStore.createPlaybook(teamId, payload)
        savedPlaybookId = created.id
      }

      await syncChecklists(savedPlaybookId)

      navigate('/playbooks')
    } catch (saveError) {
      error = saveError instanceof Error ? saveError.message : 'Failed to save playbook.'
    } finally {
      saving = false
    }
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
          onclick={back}
        >
          <ArrowLeft class="h-5 w-5" />
        </button>
        <div>
          <h1 class="text-xl font-bold">{isEditing ? 'Edit Playbook' : 'Create Playbook'}</h1>
          <p class="text-sm text-text-3">Define the reusable checklist your team will run.</p>
        </div>
      </div>
      <button
        type="button"
        class="inline-flex items-center gap-2 rounded-r-2 bg-brand px-4 py-2.5 text-sm font-semibold text-brand-foreground transition-standard hover:bg-brand-hover disabled:opacity-60"
        disabled={saving || loading}
        onclick={save}
      >
        {#if saving}
          <Loader2 class="h-4 w-4 animate-spin" />
          Saving...
        {:else}
          <Save class="h-4 w-4" />
          Save Playbook
        {/if}
      </button>
    </div>
  </header>

  <section class="mx-auto max-w-5xl space-y-6 px-6 py-6">
    {#if error}
      <div class="rounded-r-2 border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger" role="alert">
        {error}
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center rounded-r-3 border border-border-1 bg-bg-surface-1 py-16 text-text-3">
        <Loader2 class="mr-2 h-5 w-5 animate-spin" />
        Loading playbook...
      </div>
    {:else}
      <section class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
        <h2 class="mb-4 text-lg font-bold">General</h2>
        <div class="grid gap-4 sm:grid-cols-[8rem_1fr]">
          <label class="block">
            <span class="mb-1 block text-sm font-medium text-text-2">Icon</span>
            <input class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm" bind:value={form.icon} />
          </label>
          <label class="block">
            <span class="mb-1 block text-sm font-medium text-text-2">Name</span>
            <input class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm" bind:value={form.name} placeholder="Incident response" />
          </label>
        </div>
        <label class="mt-4 block">
          <span class="mb-1 block text-sm font-medium text-text-2">Description</span>
          <textarea class="min-h-24 w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm" bind:value={form.description}></textarea>
        </label>
        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <label class="flex items-center gap-2 text-sm text-text-2">
            <input type="checkbox" bind:checked={form.is_public} />
            Public to the team
          </label>
          <label class="flex items-center gap-2 text-sm text-text-2">
            <input type="checkbox" bind:checked={form.create_channel_on_run} />
            Create channel on run
          </label>
        </div>
        <label class="mt-4 block">
          <span class="mb-1 block text-sm font-medium text-text-2">Channel name template</span>
          <input class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm" bind:value={form.channel_name_template} />
        </label>
        <label class="mt-4 block">
          <span class="mb-1 block text-sm font-medium text-text-2">Keyword triggers</span>
          <input class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm" bind:value={form.keyword_triggers} placeholder="incident, outage" />
        </label>
      </section>

      <section class="space-y-4">
        <div class="flex items-center justify-between">
          <h2 class="text-lg font-bold">Checklists</h2>
          <button type="button" class="inline-flex items-center gap-2 rounded-r-2 border border-border-1 px-3 py-2 text-sm font-semibold" onclick={addChecklist}>
            <Plus class="h-4 w-4" />
            Add Checklist
          </button>
        </div>

        {#each checklists as checklist, checklistIndex (checklist.id)}
          <article class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1">
            <div class="flex gap-3">
              <input class="min-w-0 flex-1 rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm font-semibold" bind:value={checklist.name} />
              <button type="button" class="rounded-r-2 p-2 text-text-3 transition-standard hover:bg-danger/10 hover:text-danger" aria-label="Remove checklist" onclick={() => removeChecklist(checklistIndex)}>
                <Trash2 class="h-4 w-4" />
              </button>
            </div>

            <div class="mt-4 space-y-3">
              {#each checklist.tasks as task, taskIndex (task.id)}
                <div class="grid gap-2 rounded-r-2 bg-bg-surface-2 p-3 sm:grid-cols-[1fr_1fr_auto]">
                  <input class="rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm" bind:value={task.title} placeholder="Task title" />
                  <input class="rounded-r-2 border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm" bind:value={task.description} placeholder="Optional description" />
                  <button type="button" class="rounded-r-2 p-2 text-text-3 transition-standard hover:bg-danger/10 hover:text-danger" aria-label="Remove task" onclick={() => removeTask(checklistIndex, taskIndex)}>
                    <Trash2 class="h-4 w-4" />
                  </button>
                </div>
              {/each}
              <button type="button" class="inline-flex items-center gap-2 rounded-r-2 border border-dashed border-border-1 px-3 py-2 text-sm text-text-2" onclick={() => addTask(checklistIndex)}>
                <Plus class="h-4 w-4" />
                Add Task
              </button>
            </div>
          </article>
        {/each}
      </section>
    {/if}
  </section>
</main>
